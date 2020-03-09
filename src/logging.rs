#![cfg(feature = "progress-updates")]

use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::filter::threshold::ThresholdFilter;
use std::error::Error;

/// Returns `true` if initialization is successful, or `false` if the logger was
/// already initialized (in which case this function does nothing, except
/// possibly create the file with the given name).
///
/// `debug_logging` controls which messages are written to the log file: messages
/// with `DEBUG` and higher priority (`true`), or only messages with `INFO` and
/// higher priority (`false`).
pub fn init(filename: impl Into<String>, debug_logging: bool) -> bool {
    let file_appender = FileAppender::builder()
        .append(false)  // truncate the output file
        .build(filename.into())
        .unwrap();
    let appender = Appender::builder()
        .build("logfile", Box::new(file_appender));
    let progress_appender = Appender::builder()
        .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
        .build("progress", Box::new(ProgressAppender::new()));
    let root = Root::builder()
        .appender("logfile")
        .appender("progress")
        .build(
            if debug_logging { LevelFilter::Debug } else { LevelFilter::Info }
        );
    let config = Config::builder()
        .appender(appender)
        .appender(progress_appender)
        .build(root)
        .unwrap();
    log4rs::init_config(config).is_ok()
}

#[derive(Clone, Debug)]
struct ProgressAppender {}

impl ProgressAppender {
    fn new() -> Self {
        Self {}
    }
}

impl log4rs::append::Append for ProgressAppender {
    fn append(&self, record: &log::Record) -> Result<(), Box<dyn Error + Sync + Send>> {
        crate::progress::process_log_message(record)
    }

    fn flush(&self) { }
}
