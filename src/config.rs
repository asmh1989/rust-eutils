use crossbeam_deque::Worker;
use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
};

fn init_request_par() {
    let limit_request = 8;

    let work: Worker<i32> = Worker::new_lifo();
    (0..limit_request).for_each(|f| {
        work.push(f);
    });

    crate::eutils::init_work(work);
}

fn init_log() {
    init_request_par();
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build(format!(
            "log/log_{}.log",
            chrono::Utc::now().format("%y-%m-%d")
        ))
        .unwrap();

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .logger(Logger::builder().build("ssh_rs", LevelFilter::Off))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();
}

pub fn init_config() {
    init_log();
}
