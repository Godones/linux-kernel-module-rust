#include <linux/bug.h>
#include <linux/err.h>
#include <linux/errname.h>
#include <linux/printk.h>
#include <linux/uaccess.h>
#include <linux/version.h>
#include <linux/rcupdate.h>
#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/sched/signal.h>
#include <linux/refcount.h>
#include <linux/wait.h>
#include <linux/workqueue.h>
#include <linux/blk-mq.h>
#include <linux/blk_types.h>
#include <linux/blkdev.h>
#include <linux/percpu.h>

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


struct task_struct *rust_helper_get_current(void){ return current; }
void rust_helper_get_task_struct(struct task_struct *t){ get_task_struct(t); }
void rust_helper_put_task_struct(struct task_struct *t){ put_task_struct(t); }
int rust_helper_signal_pending(struct task_struct *t){ return signal_pending(t); }


long rust_helper_PTR_ERR(__force const void *ptr){ return PTR_ERR(ptr); }
bool rust_helper_IS_ERR(__force const void *ptr){ return IS_ERR(ptr); }



void *rust_helper_blk_mq_rq_to_pdu(struct request *rq)
{
    return blk_mq_rq_to_pdu(rq);
}


struct request *rust_helper_blk_mq_rq_from_pdu(void *pdu)
{
    return blk_mq_rq_from_pdu(pdu);
}


unsigned int rust_helper_num_online_cpus(void){ return num_online_cpus(); }
// dynamically allocate and free per-cpu variables
long long *rust_helper_alloc_percpu_longlong(void){ return alloc_percpu(long long); }
void rust_helper_free_percpu_longlong(long long *p){ free_percpu(p); }
int rust_helper_get_cpu(void){ return get_cpu(); }
void rust_helper_put_cpu(void){ put_cpu(); }

long long *rust_helper_per_cpu_ptr(long long *p, int cpu){ return per_cpu_ptr(p, cpu); }
