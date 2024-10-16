pub use kbind::*;
/// Page size defined in terms of the `PAGE_SHIFT` macro from C.
///
/// [`PAGE_SHIFT`]: ../../../include/asm-generic/page.h
pub const PAGE_SIZE: u32 = 1 << kbind::PAGE_SHIFT;
