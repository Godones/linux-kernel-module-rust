use core::marker::PhantomData;

/// Evidence that the RCU read side lock is held on the current thread/CPU.
///
/// The type is explicitly not `Send` because this property is per-thread/CPU.
///
/// # Invariants
///
/// The RCU read side lock is actually held while instances of this guard exist.
pub struct Guard {
    _not_send: PhantomData<*mut ()>,
}

impl Guard {
    /// Acquires the RCU read side lock and returns a guard.
    pub fn new() -> Self {
        // SAFETY: An FFI call with no additional requirements.
        crate::sys_rcu_read_lock();
        // INVARIANT: The RCU read side lock was just acquired above.
        Self {
            _not_send: PhantomData,
        }
    }

    /// Explicitly releases the RCU read side lock.
    pub fn unlock(self) {}
}

impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: By the type invariants, the rcu read side is locked, so it is ok to unlock it.
        crate::sys_rcu_read_unlock();
    }
}

/// Acquires the RCU read side lock.
pub fn read_lock() -> Guard {
    Guard::new()
}

pub(crate) fn synchronize_rcu() {
    crate::sys_synchronize_rcu()
}
