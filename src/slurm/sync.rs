use bson::{doc, Bson};
use chrono::Duration;

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use rocket::{form::validate::Contains, futures::StreamExt};
use serde_json::json;

use crate::utils::file_exist;

use super::{
    config::{self, Cloud},
    db::{Db, TABLE_NAME},
    model::JobInDb,
    ssh::get_jobs_from_cloud,
};

pub const COLLECTION_JOB: &'static str = "slurm_job";
pub const MAX_RETRY: usize = 3;

async fn find_jobs_in_day(user: &str) -> Vec<JobInDb> {
    let datetime = chrono::Utc::now();
    let starttime = datetime - Duration::days(1);
    let endtime = datetime + Duration::seconds(1);

    // 构建查询条件和排序规则
    let filter = doc! {
        "end": { "$gte": bson::Bson::DateTime(starttime.into()) , "$lt": bson::Bson::DateTime(endtime.into())},
        "user": user,
        "is_send": bson::Bson::Null,
    };

    // log::info!("filter = {:?}", &filter);
    let option = mongodb::options::FindOptions::builder()
        .sort(doc! { "end": -1 })
        // .limit(10)
        .build();

    // 执行查询
    let cur_result = Db::find_with_table(TABLE_NAME, COLLECTION_JOB, filter, option).await;

    let mut v: Vec<JobInDb> = Vec::new();
    // 遍历结果
    if let Ok(mut cursor) = cur_result {
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => {
                    // 处理每个文档
                    let result = bson::from_bson::<JobInDb>(Bson::Document(document.clone()));
                    if let Ok(job) = result {
                        v.push(job);
                    } else {
                        log::error!("parse doc error : {:?}", document);
                    }
                }
                Err(error) => {
                    // 处理错误
                    log::error!("Error: {}", error);
                }
            }
        }
    }

    v
}

async fn send_notification(c: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://oapi.dingtalk.com/robot/send?access_token=1abbebf38e77d1228b63b43333fd2cb60dda34ca48c3e5c73aaa4d709f4ebdb9";
    let content = json!(
        {
            "msgtype": "markdown",
            "markdown": {
                "title":"任务信息",
                "text": c
            },
             "at": {
                 "atMobiles": [
                 ],
                 "atUserIds": [
                 ],
                 "isAtAll": false
             }
    }
    );

    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    // log::info!("content = {}", &content);

    let response = client
        .post(url)
        .headers(headers)
        .json(&content)
        .send()
        .await?;

    // log::info!("Response status: {}", response.status());
    let body = response.text().await?;
    log::info!("Response body: {}", body);

    Ok(())
}

fn succ_notification(job: &JobInDb, local: &str) -> String {
    format!(
        r#"## {} jobId-{} 已完成      
* 名称: {}
* 耗时: {} 秒
* 云上: {}
* 本地: {}"#,
        &job.cloud, job.job_id, &job.job_name, job.elapsed_raw, &job.work_dir, local
    )
}

fn failed_notification(job: &JobInDb) -> String {
    format!(
        r#"## {} jobId-{} 失败了
- 名称: {}
- 云上: {}

```
日志:
{}
```"#,
        &job.cloud,
        job.job_id,
        &job.job_name,
        &job.work_dir,
        &job.stdout.clone().unwrap_or("".to_string())
    )
}

async fn find_failed_times(job: &JobInDb) -> usize {
    let filter = doc! {
        "job_name": job.job_name.clone(),
        "user": job.user.clone(),
        "state": "FAILED"
    };

    Db::count_with_table(TABLE_NAME, COLLECTION_JOB, filter).await as usize
}

async fn save_failed_send(job: &JobInDb) {
    // 发送状态保存
    let filter = doc! {
        "job_id": job.job_id,
        "user": job.user.clone()
    };
    let _ = Db::save_with_table(TABLE_NAME, COLLECTION_JOB, filter, doc! {"is_send": 1 }).await;
}

async fn sync_cloud(cloud: &Cloud) {
    let vv = find_jobs_in_day(&cloud.user()).await;
    for job in &vv {
        // 结束后未处理
        if job.is_send.is_none() {
            // 只关心cloud_sbatch 提交的任务
            let is_complex = job.job_name.ends_with("_complex");
            if is_complex || job.job_name.ends_with("_ligand") {
                if job.state.contains("COMPLETED") {
                    // 完成
                    let p = job.local_tgz();
                    if is_complex {
                        super::ssh::sync_tgz(cloud, job);
                        if file_exist(&p) {
                            // 推送成功
                            log::info!("start send succ notidication");
                            let result = send_notification(&succ_notification(&job, &p)).await;
                            if result.is_ok() {
                                // 发送状态保存
                                let filter = doc! {
                                    "job_id": job.job_id,
                                    "user": job.user.clone()
                                };
                                let _ = Db::save_with_table(
                                    TABLE_NAME,
                                    COLLECTION_JOB,
                                    filter,
                                    doc! {"is_send": 1 },
                                )
                                .await;
                            }
                        }
                    }
                } else if job.state.contains("FAILED") {
                    //失败
                    let times = find_failed_times(job).await;
                    if times < MAX_RETRY {
                        save_failed_send(job).await;
                        log::info!(
                            "{}: job-{}, start retry, times = {}",
                            &job.cloud,
                            &job.job_name,
                            times
                        );
                        super::ssh::job_restart(job, cloud).await;
                    } else {
                        let result = send_notification(&failed_notification(job)).await;
                        if result.is_ok() {
                            save_failed_send(job).await;
                        }
                    }
                }
            }
        }
    }
}

pub async fn start_sync() {
    let v = config::Config::clouds();

    for f in v {
        let start = super::ssh::find_start_time(&f.user()).await;
        let v = get_jobs_from_cloud(&f, &start);
        for f in v {
            let filter = doc! {
                "job_id": f.job_id,
                "user": f.user.clone()
            };

            let result =
                Db::save_with_table(TABLE_NAME, COLLECTION_JOB, filter, f.document().unwrap())
                    .await;

            if result.is_err() {
                log::error!("insert db error : {:?}", result.err());
            }
        }

        sync_cloud(&f).await;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_sync() {
        crate::config::init_config();
        crate::slurm::init().await;
        start_sync().await;
    }

    #[tokio::test]
    async fn test_send_notification() {
        crate::config::init_config();
        // crate::slurm::init().await;

        let _ = send_notification(
            r#"## 测试 并行云 jobId-679155 已完成
            
        * 名称: prkab1_BFL-092A-1_md_complex
        * 耗时: 148198 秒
        * 云上: /data/run01/scz1961/yrl/abfep/prkab1/8-4-2-ligs/BFL-066A-2_md/fep
        * 本地: /mnt/share/cloud_sbatch/yrl/abfep/prkab1/8-4-2-ligs/BFL-066A-1_md-out-xvg.tgz"#,
        )
        .await;
    }

    #[tokio::test]
    async fn test_send_notification2() {
        crate::config::init_config();
        // crate::slurm::init().await;

        let out = "prkab1_AIxFuse_3_702531-2_md_complex\nstart equ run at 08-01 18:32:36 2023\ng0035 res: GPUS=2 CPUS=32 cpu_num=1 lambda_num=32 par_job=32\nfinish equ run at 08-01 20:57:39 2023\nElapsed time: 0 day(s), 2 hour(s), 25 minute(s), 3 second(s)\n Finished equilibration\nStart run\nstart prod run at 08-01 20:57:39 2023\nfinish prod run at 08-03 11:31:07 2023\nElapsed time: 1 day(s), 14 hour(s), 33 minute(s), 28 second(s)\nCannot find MPS control daemon process";

        let _ = send_notification(&format!(
            r#"## 测试 并行云 jobId-671399 失败了

- 名称: prkab1_AIxFuse_3_702531-2_md_complex
- 云上: /data/run01/scz1961/yrl/abfep/prkab1/7-14-50-ligs/AIxFuse_3_702531-2_md/fep

```
日志:
{}
```
"#,
            out
        ))
        .await;
    }

    #[tokio::test]
    async fn test_find_job_in_day() {
        crate::config::init_config();
        crate::slurm::init().await;

        let result = find_jobs_in_day("scz1961").await;
        log::info!("scz1961: find jobs = {:?}", result);
    }

    #[test]
    fn feature() {
        crate::config::init_config();
        let str = "/data/run01/scz1961/yrl/abfep/jak1/8-4-2-ligs/BFL-066A_md/fep";
        let ss = str.split("scz1961").collect::<Vec<&str>>()[1];
        log::info!("ss = {}", ss);
        let path = std::path::Path::new(str);
        let p = path.parent().unwrap().parent().unwrap();
        log::info!("path = {}", p.display().to_string());

        // log::info!("get info = {:?}", get_info_path(str));
    }
}
