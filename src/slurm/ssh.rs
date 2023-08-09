use std::path::Path;

use bson::{doc, Bson};
use chrono::Duration;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rocket::futures::StreamExt;
use ssh_rs::ssh;

use crate::{
    slurm::{
        db::{Db, TABLE_NAME},
        model::Job,
        sync::COLLECTION_JOB,
    },
    utils::read_csv_from_str,
};

use super::{config::Cloud, model::JobInDb};

fn string_2_date(time: &str) -> Option<bson::DateTime> {
    let datetime_string = &format!("{}+08:00", time);
    if let Ok(r) = bson::DateTime::parse_rfc3339_str(datetime_string) {
        Some(r)
    } else {
        None
    }
}

fn job_2_db(job: &Job, cloud: &str) -> JobInDb {
    let mut cpu = 0u32;
    let mut gpu = 0u32;
    let mut mem = "".to_string();
    let stdout = None;

    if !job.alloc_tres.is_empty() {
        let pairs: Vec<&str> = job.alloc_tres.split(',').collect();

        for pair in pairs {
            let key_value: Vec<&str> = pair.split('=').collect();
            if key_value.len() == 2 {
                let key = key_value[0].trim().to_string();
                let value = key_value[1].trim().to_string();
                if key.contains("cpu") {
                    cpu = value.parse::<u32>().unwrap();
                } else if key.contains("gpu") {
                    gpu = value.parse::<u32>().unwrap();
                } else if key.contains("mem") {
                    mem = value;
                }
            } else {
                log::error!(
                    "Invalid key-value pair: alloc_tres={}, node_list={} ",
                    &job.alloc_tres,
                    &job.node_list
                );
            }
        }
    }

    JobInDb {
        user: job.user.clone(),
        job_id: job.job_id.clone(),
        job_name: job.job_name.clone(),
        start: string_2_date(&job.start),
        end: string_2_date(&job.end),
        elapsed_raw: job.elapsed_raw.clone(),
        work_dir: job.work_dir.clone(),
        state: job.state.clone(),
        node_list: job.node_list.clone(),
        cloud: cloud.to_string(),
        is_send: None,
        cpu,
        gpu,
        mem,
        stdout,
    }
}

fn slurm_stdout(f: &JobInDb) -> String {
    let path = format!("{}/slurm-{}.out", &f.work_dir, f.job_id);
    format!(
        "if [ -f \"{}\" ]; then
    cat {}
else
    echo \"\"
fi",
        &path, &path
    )
}

pub async fn job_restart(job: &JobInDb, cloud: &Cloud) {
    log::info!("start ssh: {}", &cloud.username);
    let result = ssh::create_session()
        .username(&cloud.username)
        .password(&cloud.password)
        .timeout(300 * 1000)
        .private_key_path(&cloud.ssh_pri_key)
        .connect(&cloud.ssh_url);
    match result {
        Ok(session) => {
            log::info!(
                "{} ssh 登录成功 for restart job - {}",
                &cloud.username,
                &job.job_name
            );
            let mut s = session.run_local();
            let exec = s.open_exec().unwrap();
            let script_name = if job.job_name.ends_with("_complex") {
                "x_complex.sh"
            } else {
                "x_ligand.sh"
            };
            let command = format!(
                "cd {} && sbatch --gres=gpu:{} -n {} -J {} {}",
                &job.work_dir, job.gpu, job.cpu, &job.job_name, script_name
            );

            log::info!("command = {}", &command);
            let vec: Vec<u8> = exec.send_command(&command).unwrap();
            log::info!(
                "restart job stdout: {}",
                String::from_utf8(vec).unwrap_or("".to_owned())
            );
            s.close();
        }
        Err(err) => {
            log::error!("ssh登录失败: {:?}", &err);
        }
    }
}

pub fn sync_tgz(cloud: &Cloud, job: &JobInDb) {
    log::info!("start ssh: {}", &cloud.username);
    let result = ssh::create_session()
        .username(&cloud.username)
        .password(&cloud.password)
        .timeout(3000 * 1000)
        .private_key_path(&cloud.ssh_pri_key)
        .connect(&cloud.ssh_url);
    match result {
        Ok(session) => {
            log::info!("{} ssh 登录成功 for 获取远程压缩文件", &cloud.username);
            let mut s = session.run_local();
            let exec = s.open_exec().unwrap();
            let work_dir = job.work_dir_root();
            let md = job.md_name();
            let tgz_name = job.tgz_name();
            let command = format!(
                "cd {} && tar cf {} {}/fep/*/lambda*/*xvg",
                work_dir, tgz_name, md
            );

            log::info!("command = {}", &command);
            let _: Vec<u8> = exec.send_command(&command).unwrap();
            let p = job.local_dir();
            let local_path = Path::new(&p);
            let _ = std::fs::create_dir_all(local_path);
            let scp = s.open_scp().unwrap();
            let _ = std::fs::create_dir("dir");
            let remote_path = &format!("{}/{}", work_dir, tgz_name);
            log::info!("start download{} to {}", &remote_path, &p);
            let result = scp.download(&p, remote_path);
            s.close();
            if result.is_err() {
                log::info!("scp download error! {:?}", result.err());
            }
        }
        Err(err) => {
            log::error!("ssh登录失败: {:?}", &err);
        }
    }
}

pub fn get_jobs_from_cloud(cloud: &Cloud, start_time: &str) -> Vec<JobInDb> {
    log::info!("start ssh: {}", &cloud.username);
    let result = ssh::create_session()
        .username(&cloud.username)
        .password(&cloud.password)
        .timeout(300 * 1000)
        .private_key_path(&cloud.ssh_pri_key)
        .connect(&cloud.ssh_url);
    match result {
        Ok(session) => {
            log::info!("{} ssh 登录成功 for 同步远程作业", &cloud.username);
            let mut s = session.run_local();
            let exec = s.open_exec().unwrap();
            let command = format!("sacct --format=user,jobid,jobname,start,end,elapsedraw,workdir,state,NodeList,AllocTRES --parsable2 -S {} | awk -F\"|\" '$1 != \"\" && $NF != \"\"  {{print}}'", start_time);

            log::info!("command = {}", &command);
            let vec: Vec<u8> = exec.send_command(&command).unwrap();
            let job_str = String::from_utf8(vec).unwrap();
            let mut v: Vec<Job> = Vec::new();
            let mut vv = Vec::new();

            let csv_result = read_csv_from_str(&job_str, b'|', &mut v);
            if csv_result.is_err() {
                log::info!("csv parse error : {:?}", csv_result.err());
            } else {
                if !v.is_empty() {
                    log::info!("found job items = {}", v.len());
                    let now = bson::DateTime::now();
                    vv = v
                        .par_iter()
                        .map(|job| job_2_db(&job, &cloud.info))
                        .collect::<Vec<JobInDb>>();

                    log::info!("start check stdout...");
                    vv.iter_mut().for_each(|f| {
                        if let Some(end) = f.end {
                            if now
                                .to_chrono()
                                .signed_duration_since(end.to_chrono())
                                .num_days()
                                < 30
                            {
                                log::info!("parse job_id = {}", f.job_id);
                                let exec = s.open_exec().unwrap();
                                let vec: Vec<u8> = exec.send_command(&slurm_stdout(f)).unwrap();
                                let cc = &String::from_utf8(vec).unwrap();

                                (*f).stdout = Some(cc.trim().to_string());
                            } else {
                                (*f).stdout = Some("".to_string());
                            }
                        }
                    });
                } else {
                    log::info!("{}: 未发现新的任务作业", &cloud.info)
                }
            }
            s.close();
            return vv;
        }
        Err(err) => {
            log::error!("ssh登录失败: {:?}", &err);
            return vec![];
        }
    }
}

pub async fn find_start_time(user: &str) -> String {
    // 构建查询条件和排序规则
    let filter = doc! {
        "stdout": { "$ne": bson::Bson::Null },
        "user": user
    };
    let option = mongodb::options::FindOptions::builder()
        .sort(doc! { "end": -1 })
        .limit(1)
        .build();

    // 执行查询
    let cur_result = Db::find_with_table(TABLE_NAME, COLLECTION_JOB, filter, option).await;
    let mut datetime = mongodb::bson::DateTime::now();

    // 遍历结果
    if let Ok(mut cursor) = cur_result {
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => {
                    // 处理每个文档
                    let result = bson::from_bson::<JobInDb>(Bson::Document(document.clone()));
                    if let Ok(job) = result {
                        datetime = job.end.clone().unwrap();
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

    // 东八区的时间
    let d = datetime.to_chrono() + Duration::hours(8) + Duration::seconds(1);
    format!("{}", d.format("%Y-%m-%dT%H:%M:%S"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        crate::config::init_config();
        crate::slurm::init().await;
        let datetime_string = "Unknown";

        let result = string_2_date(datetime_string);

        println!("DateTime with timezone: {:?}", result);

        let result = find_start_time("scz1961").await;
        log::info!("scz1961: start time = {:?}", result);

        let result = find_start_time("aixplorerbio_wz").await;
        log::info!("aixplorerbio_wz: start time = {:?}", result);
    }
}
