#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;
use alloc::boxed::Box;

use basic::{println, LinuxResult};
use interface::{Basic};
use log::{Log, Metadata, Record};
use interface::logger::{Level, LevelFilter, LogDomain};
use rref::RRefVec;

#[derive(Debug)]
pub struct Logger;

impl Basic for Logger {
    fn domain_id(&self) -> u64 {
        rref::domain_id()
    }
}

impl LogDomain for Logger {
    fn init(&self) -> LinuxResult<()> {
        log::set_logger(&SimpleLogger).unwrap();
        // default log level
        log::set_max_level(log::LevelFilter::Trace);
        println!("Logger init");
        Ok(())
    }

    fn log(&self, level: Level, msg: &RRefVec<u8>) -> LinuxResult<()> {
        let msg = core::str::from_utf8(msg.as_slice()).unwrap();
        let level = match level {
            Level::Error => log::Level::Error,
            Level::Warn => log::Level::Warn,
            Level::Info => log::Level::Info,
            Level::Debug => log::Level::Debug,
            Level::Trace => log::Level::Trace,
        };
        log::log!(level, "{}", msg);
        Ok(())
    }

    fn set_max_level(&self, level: LevelFilter) -> LinuxResult<()> {
        log::set_max_level(match level {
            LevelFilter::Error => log::LevelFilter::Error,
            LevelFilter::Warn => log::LevelFilter::Warn,
            LevelFilter::Info => log::LevelFilter::Info,
            LevelFilter::Debug => log::LevelFilter::Debug,
            LevelFilter::Trace => log::LevelFilter::Trace,
            _ => log::LevelFilter::Off,
        });
        println!("Logger set_max_level: {:?}", level);
        Ok(())
    }
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let color = match record.level() {
            log::Level::Error => 31, // Red
            log::Level::Warn => 93,  // BrightYellow
            log::Level::Info => 35,  // Blue
            log::Level::Debug => 32, // Green
            log::Level::Trace => 90, // BrightBlack
        };
        println!(
            "\u{1B}[{}m[{:>1}] {}\u{1B}[0m",
            color,
            record.level(),
            record.args(),
        );
    }
    fn flush(&self) {}
}


pub fn main() -> Box<dyn LogDomain> {
    Box::new(Logger)
}
