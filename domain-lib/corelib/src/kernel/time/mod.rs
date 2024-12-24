pub mod hrtimer;

pub fn ktime_get_ns() -> u64 {
    crate::sys_ktime_get_ns()
}
