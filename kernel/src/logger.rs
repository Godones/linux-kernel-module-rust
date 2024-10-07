use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::{pr_cont, pr_debug, pr_err, pr_info, pr_warn, println};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let module_path = record.module_path().unwrap_or_default();
        match record.level() {
            Level::Error => {
                pr_err!("[ERROR] [{}] {}", module_path, record.args());
            }
            Level::Warn => {
                pr_warn!("[ WARN] [{}] {}", module_path, record.args());
            }
            Level::Info => {
                pr_info!("[ INFO] [{}] {}", module_path, record.args());
            }
            Level::Debug => {
                pr_debug!("[DEBUG] [{}] {}", module_path, record.args());
            }
            Level::Trace => {
                pr_cont!("[TRACE] [{}] {}", module_path, record.args());
            }
        };
    }
    fn flush(&self) {}
}

pub fn init_logger() {
    println!("Init logger {:?}", option_env!("LOG"));
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("ERROR") => LevelFilter::Error,
        Some("WARN") => LevelFilter::Warn,
        Some("INFO") => LevelFilter::Info,
        Some("DEBUG") => LevelFilter::Debug,
        Some("TRACE") => LevelFilter::Trace,
        _ => LevelFilter::Info,
    });
}
