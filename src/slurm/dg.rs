use std::process::Command;

use bson::{doc, Bson};
use chrono::Duration;
use rocket::futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::{
    config::Cloud,
    db::{Db, TABLE_NAME},
    model::JobInDb,
    sync::COLLECTION_JOB,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalJob {
    pub user: String,
    pub job_id: u32,
    pub work_dir: String,
    pub file: String,
    pub is_done: Option<u32>,
}

const COLLECTION_LOCAL_JOB: &'static str = "local_job";

async fn find_jobs_in_day(user: &str) -> Vec<JobInDb> {
    let datetime = chrono::Utc::now();
    let starttime = datetime - Duration::days(1);
    let endtime = datetime + Duration::seconds(1);

    // 构建查询条件和排序规则
    let filter = doc! {
        "end": { "$gte": bson::Bson::DateTime(starttime.into()) , "$lt": bson::Bson::DateTime(endtime.into())},
        "user": user,
        "is_send": 1,
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

fn run_command(job: &LocalJob, tgz: &str) -> bool {
    log::info!(
        "run_command bash /mnt/archive/ssh/get-dg.sh {} {}",
        &job.work_dir,
        &tgz
    );
    if let Ok(output) = Command::new("bash")
        .arg("/mnt/archive/ssh/get-dg.sh")
        .arg(&job.work_dir)
        .arg(tgz)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            log::info!("Bash file executed successfully:\n{}", stdout);
            return true;
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::info!("Failed to execute the Bash file:\n{}", stderr);
        }
    }

    false
}

pub async fn do_dg(cloud: &Cloud) {
    let vv = find_jobs_in_day(&cloud.user()).await;
    log::info!("found {} job need run do_dg", vv.len());
    for job in &vv {
        let filter = doc! {
            "job_id": job.job_id,
            "user": job.user.clone(),
            "is_done": bson::Bson::Null
        };
        if let Ok(result) = Db::find_one(COLLECTION_LOCAL_JOB, filter.clone(), None).await {
            if let Some(doc) = result {
                if let Ok(local) = bson::from_bson::<LocalJob>(Bson::Document(doc)) {
                    if run_command(&local, &job.local_tgz()) {
                        let _ = Db::save_with_table(
                            TABLE_NAME,
                            COLLECTION_LOCAL_JOB,
                            filter,
                            doc! {"is_done": 1},
                        )
                        .await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::slurm::config;

    use super::*;

    #[tokio::test]
    async fn test_do_dg() {
        crate::config::init_config();
        crate::slurm::init().await;
        let v = config::Config::clouds();

        for f in v {
            do_dg(&f).await;
        }
    }
}
