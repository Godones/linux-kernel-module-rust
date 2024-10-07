use core::ffi::{c_int, c_longlong};

/// Dynamically allocate and free per-cpu variables with long long (i64) type.
#[derive(Debug)]
pub struct LongLongPerCpu {
    ptr: *mut c_longlong,
}

impl LongLongPerCpu {
    pub fn new() -> Self {
        let ptr = unsafe { crate::bindings::alloc_percpu_longlong() };
        Self { ptr }
    }

    /// Get the value of the per-cpu variable.
    pub fn get_value(&self) -> i64 {
        let cpu = unsafe { crate::bindings::get_cpu() };
        let ptr = unsafe { crate::bindings::per_cpu_ptr(self.ptr, cpu) };
        let value = unsafe { *ptr };
        unsafe { crate::bindings::put_cpu() };
        value
    }

    /// Set the value of the per-cpu variable.
    pub fn set_value(&self, value: i64) {
        let cpu = unsafe { crate::bindings::get_cpu() };
        let ptr = unsafe { crate::bindings::per_cpu_ptr(self.ptr, cpu) };
        unsafe { *ptr = value };
        unsafe { crate::bindings::put_cpu() };
    }

    /// Get the value of the per-cpu variable and execute a closure with it.
    pub fn get_with<R>(&self, f: impl Fn(&mut i64) -> R) -> R {
        let cpu = unsafe { crate::bindings::get_cpu() };
        let ptr = unsafe { crate::bindings::per_cpu_ptr(self.ptr, cpu) };
        let value = unsafe { &mut *ptr };
        let result = f(value);
        unsafe { crate::bindings::put_cpu() };
        result
    }

    /// Execute a closure for each CPU.
    pub fn for_each_cpu(&self, f: impl Fn(&mut i64)) {
        for cpu in 0..unsafe { crate::bindings::num_online_cpus() } {
            let ptr = unsafe { crate::bindings::per_cpu_ptr(self.ptr, cpu as c_int) };
            let value = unsafe { &mut *ptr };
            f(value);
        }
    }

    /// Calculate the sum of the per-cpu variables.
    pub fn sum(&self) -> i64 {
        let mut sum = 0;
        for cpu in 0..unsafe { crate::bindings::num_online_cpus() } {
            let ptr = unsafe { crate::bindings::per_cpu_ptr(self.ptr, cpu as c_int) };
            let value = unsafe { *ptr };
            sum += value;
        }
        sum
    }
}

unsafe impl Send for LongLongPerCpu {}
unsafe impl Sync for LongLongPerCpu {}

impl Drop for LongLongPerCpu {
    fn drop(&mut self) {
        unsafe { crate::bindings::free_percpu_longlong(self.ptr) };
    }
}

#[derive(Debug)]
pub struct CpuId;

impl CpuId {
    pub fn read<R>(f: impl Fn(i32) -> R) -> R {
        let cpu = unsafe { crate::bindings::get_cpu() };
        let r = f(cpu);
        unsafe { crate::bindings::put_cpu() };
        r
    }
}
