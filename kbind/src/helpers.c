#include <linux/bug.h>
#include <linux/err.h>
#include <linux/errname.h>
#include <linux/printk.h>
#include <linux/uaccess.h>
#include <linux/version.h>
#include <linux/rcupdate.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>


void bug_helper(void) { BUG(); }

int access_ok_helper(const void __user *addr, unsigned long n) {
#if LINUX_VERSION_CODE >= KERNEL_VERSION(5, 0, 0) /* v5.0-rc1~46 */
  return access_ok(addr, n);
#else
  return access_ok(0, addr, n);
#endif
}

/* see https://github.com/rust-lang/rust-bindgen/issues/1671 */
_Static_assert(__builtin_types_compatible_p(size_t, uintptr_t),
               "size_t must match uintptr_t, what architecture is this??");

const char *rust_helper_errname(int err) { return errname(err); }



// make sure the arg is void if the function doesn't have any args
void rust_helper_rcu_read_unlock(void) { rcu_read_unlock(); }
void rust_helper_rcu_read_lock(void) { rcu_read_lock(); }
void rust_helper_synchronize_rcu(void) { synchronize_rcu(); }

struct rcudata {
    void *a;
};


void * rust_helper_rcu_dereference(struct rcudata *p) {
    return rcu_dereference(p->a);
}


void rust_helper_rcu_assign_pointer(struct rcudata *p, void *v) {
    rcu_assign_pointer(p->a, v);
}


void rust_helper_spin_lock_init(spinlock_t *lock) { spin_lock_init(lock); }
void rust_helper_spin_lock(spinlock_t *lock) { spin_lock(lock); }
void rust_helper_spin_unlock(spinlock_t *lock) { spin_unlock(lock); }

void rust_helper_mutex_init(struct mutex *lock) { mutex_init(lock); }
void rust_helper_mutex_lock(struct mutex *lock) { mutex_lock(lock); }
void rust_helper_mutex_unlock(struct mutex *lock) { mutex_unlock(lock); }

