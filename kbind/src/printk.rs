use core::{cmp, ffi::c_int, fmt};

use crate::bindings;
#[doc(hidden)]
pub fn printk(s: &[u8]) {
    // Don't copy the trailing NUL from `KERN_INFO`.
    let mut fmt_str = [0; bindings::KERN_INFO.len() - 1 + b"%.*s\0".len()];
    fmt_str[..bindings::KERN_INFO.len() - 1]
        .copy_from_slice(&bindings::KERN_INFO[..bindings::KERN_INFO.len() - 1]);
    fmt_str[bindings::KERN_INFO.len() - 1..].copy_from_slice(b"%.*s\0");

    // TODO: I believe printk never fails
    unsafe { bindings::_printk(fmt_str.as_ptr() as _, s.len() as c_int, s.as_ptr()) };
}

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
/// [`println!`]: https://doc.rust-lang.org/stable/std/macro.println.html
#[macro_export]
macro_rules! println {
    () => ({
        $crate::printk::printk("\n".as_bytes());
    });
    ($fmt:expr) => ({
        $crate::printk::printk(concat!($fmt, "\n").as_bytes());
    });
    ($fmt:expr, $($arg:tt)*) => ({
        use ::core::fmt;
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = fmt::write(&mut writer, format_args!(concat!($fmt, "\n"), $($arg)*)).unwrap();
        $crate::printk::printk(writer.as_bytes());
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let mut writer = $crate::printk::LogLineWriter::new();
        let _ = core::fmt::write(&mut writer, format_args!($($arg)*)).unwrap();
        $crate::printk::printk(writer.as_bytes());
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
        $crate::print!(concat!("\x1b[", $color, "m", $fmt, "\x1b[0m\n"));
    };
    ($color:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!("\x1b[", $color, "m", $fmt, "\x1b[0m\n"), $($arg)*);
    };
}
