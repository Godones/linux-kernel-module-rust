use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::{pr_err, println};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.level() {
            Level::Error => {
                pr_err!("[ERROR] {}", record.args());
            }
            Level::Warn => {
                println!("[WARN] {}", record.args());
            }
            Level::Info => {
                println!("[INFO] {}", record.args());
            }
            Level::Debug => {
                println!("[DEBUG] {}", record.args());
            }
            Level::Trace => {
                println!("[TRACE] {}", record.args());
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
