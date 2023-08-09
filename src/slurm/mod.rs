use std::time::Duration;

use tokio::time::sleep;

use self::sync::start_sync;

mod config;
mod db;
mod model;
mod ssh;
mod sync;
pub async fn init() {
    config::Config::get_instance();
    db::init_db("mongodb://root:Sz123456@192.168.2.26:27017").await;
}

pub fn start_timetask() {
    tokio::spawn(async {
        sleep(Duration::from_secs(30)).await;
        loop {
            // 执行任务逻辑
            log::info!("start sync");
            start_sync().await;

            // 等待 10 分钟
            sleep(Duration::from_secs(600)).await;
        }
    });
}
