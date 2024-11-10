use core::{
    cmp, fmt,
    fmt::{Arguments, Write},
};

const LOG_LINE_MAX: usize = 1024 - 32;

#[doc(hidden)]
pub struct LogLineWriter {
    data: [u8; LOG_LINE_MAX],
    pos: usize,
}

#[allow(clippy::new_without_default)]
impl LogLineWriter {
    pub fn new() -> LogLineWriter {
        LogLineWriter {
            data: [0u8; LOG_LINE_MAX],
            pos: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.pos]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.pos]).unwrap()
    }
}

impl fmt::Write for LogLineWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let copy_len = cmp::min(LOG_LINE_MAX - self.pos, s.as_bytes().len());
        self.data[self.pos..self.pos + copy_len].copy_from_slice(&s.as_bytes()[..copy_len]);
        self.pos += copy_len;
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let mut writer = $crate::console::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[0][Domain:{}] {}",rref::domain_id(), format_args!($($arg)*))).unwrap();
        $crate::console::__print(format_args!("{}", writer.as_str()));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

/// Print with color
///
/// The first argument is the color, which should be one of the following:
/// - 30: Black
/// - 31: Red
/// - 32: Green
/// - 33: Yellow
/// - 34: Blue
/// - 35: Magenta
/// - 36: Cyan
/// - 37: White
///
#[macro_export]
macro_rules! println_color {
    ($color:expr,$($arg:tt)*) => {
        let mut writer = $crate::console::LogLineWriter::new();
        let prefix = match $color {
            31 => "[ERROR] ",
            32 => "[INFO] ",
            33 => "[WARN] ",
            34 => "[DEBUG] ",
            _ => "[UNKNOWN] ",
        };
         let _ = core::fmt::write(&mut writer, format_args!("[0][Domain:{}]{} {}\n",rref::domain_id(), prefix, format_args!($($arg)*))).unwrap();
        $crate::console::__print(format_args!("{}", writer.as_str()));
    };
}

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::write_console(s);
        Ok(())
    }
}

pub fn __print(args: Arguments) {
    Stdout.write_fmt(args).unwrap();
}
