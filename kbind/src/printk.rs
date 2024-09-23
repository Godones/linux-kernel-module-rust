use core::{cmp, ffi::c_int, fmt};

use crate::bindings;

macro_rules! printk_define {
    ($name:ident, $level:expr) => {
        pub fn $name(s: &[u8]) {
            let mut fmt_str = [0; $level.len() - 1 + b"%.*s\0".len()];
            fmt_str[..$level.len() - 1].copy_from_slice(&$level[..$level.len() - 1]);
            fmt_str[$level.len() - 1..].copy_from_slice(b"%.*s\0");
            unsafe {
                bindings::_printk(fmt_str.as_ptr() as _, s.len() as c_int, s.as_ptr());
            }
        }
    };
}

printk_define!(printk_debug, bindings::KERN_DEBUG);
printk_define!(printk_info, bindings::KERN_INFO);
printk_define!(printk_notice, bindings::KERN_NOTICE);
printk_define!(printk_warning, bindings::KERN_WARNING);
printk_define!(printk_err, bindings::KERN_ERR);
printk_define!(printk_crit, bindings::KERN_CRIT);
printk_define!(printk_alert, bindings::KERN_ALERT);
printk_define!(printk_emerg, bindings::KERN_EMERG);
printk_define!(printk_cont, bindings::KERN_CONT);

// From kernel/print/printk.c
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
}

impl fmt::Write for LogLineWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let copy_len = cmp::min(LOG_LINE_MAX - self.pos, s.as_bytes().len());
        self.data[self.pos..self.pos + copy_len].copy_from_slice(&s.as_bytes()[..copy_len]);
        self.pos += copy_len;
        Ok(())
    }
}

/// [`println!`] functions the same as it does in `std`, except instead of
/// printing to `stdout`, it writes to the kernel console at the `KERN_INFO`
/// level.
///
/// It's equivalent to calling [`pr_info`] with the format string and
/// arguments.
///
/// [`println!`]: https://doc.rust-lang.org/stable/std/macro.println.html
#[macro_export]
macro_rules! println {
    () => ({
        $crate::printk::printk_info("\n".as_bytes());
    });
    ($fmt:expr) => ({
        $crate::printk::printk_info(concat!("[LKM] " ,$fmt, "\n").as_bytes());
    });
    ($fmt:expr, $($arg:tt)*) => ({
        use ::core::fmt;
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = fmt::write(&mut writer, format_args!(concat!("[LKM] ",$fmt, "\n"), $($arg)*)).unwrap();
        $crate::printk::printk_info(writer.as_bytes());
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_info(writer.as_bytes());
    };
}
#[macro_export]
macro_rules! print_raw {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("{}",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_info(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_debug {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
      let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_debug(writer.as_bytes());
    };
}
#[macro_export]
macro_rules! pr_info {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_info(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_notice {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_notice(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_warning {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_warning(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_err {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_err(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_crit {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_crit(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_alert {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_alert(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_emerg {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_emerg(writer.as_bytes());
    };
}

#[macro_export]
macro_rules! pr_cont {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!("[LKM] {}\n",format_args!($($arg)*))).unwrap();
        $crate::printk::printk_cont(writer.as_bytes());
    };
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
/// # Examples
/// ```rust
/// use kbind::println_color;
/// println_color!(31, "This is red");
/// ```
#[macro_export]
macro_rules! println_color {
    ($color:expr, $fmt:expr) => {
        match $color {
            31 => $crate::printk::printk_err(concat!("[LKM] ", $fmt, "\n").as_bytes()),
            32 => $crate::printk::printk_info(concat!("[LKM] ", $fmt, "\n").as_bytes()),
            33 => $crate::printk::printk_warning(concat!("[LKM] ", $fmt, "\n").as_bytes()),
            34 => $crate::printk::printk_debug(concat!("[LKM] ", $fmt, "\n").as_bytes()),
            _ => $crate::printk::printk_info(concat!("[LKM] ", $fmt, "\n").as_bytes()),
        }
    };
    ($color:expr, $fmt:expr, $($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!(concat!("[LKM] ", $fmt, "\n"), $($arg)*)).unwrap();
        match $color {
            31 => $crate::printk::printk_err(writer.as_bytes()),
            32 => $crate::printk::printk_info(writer.as_bytes()),
            33 => $crate::printk::printk_warning(writer.as_bytes()),
            34 => $crate::printk::printk_debug(writer.as_bytes()),
            _ => $crate::printk::printk_info(writer.as_bytes()),
        }
    };
}
