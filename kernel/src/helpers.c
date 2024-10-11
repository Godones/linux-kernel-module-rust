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
#include <linux/bio.h>
#include <linux/slab.h>
#include <linux/radix-tree.h>
#include <linux/fs.h>
#include <linux/pagemap.h>
void bug_helper(void) { BUG(); }

int access_ok_helper(const void __user *addr, unsigned long n) {
#if LINUX_VERSION_CODE >= KERNEL_VERSION(5, 0, 0) /* v5.0-rc1~46 */
  return access_ok(addr, n);
#else
  return access_ok(0, addr, n);
#endif
}

__force void *rust_helper_ERR_PTR(long err)
{
    return ERR_PTR(err);
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
void * rust_helper_rcu_dereference(struct rcudata *p) { return rcu_dereference(p->a); }
void rust_helper_rcu_assign_pointer(struct rcudata *p, void *v) { rcu_assign_pointer(p->a, v); }


// spinlock
void rust_helper_spin_lock_init(spinlock_t *lock ,const char *name,
                                struct lock_class_key *key){
#ifdef CONFIG_DEBUG_SPINLOCK
    __raw_spin_lock_init(spinlock_check(lock), name, key, LD_WAIT_CONFIG);
#else
    spin_lock_init(lock);
#endif
}
void rust_helper_spin_lock(spinlock_t *lock) { spin_lock(lock); }
void rust_helper_spin_unlock(spinlock_t *lock) { spin_unlock(lock); }
void rust_helper_spin_unlock_irqrestore(spinlock_t *lock, unsigned long flags)
{
    spin_unlock_irqrestore(lock, flags);
}
unsigned long rust_helper_spin_lock_irqsave(spinlock_t *lock)
{
    unsigned long flags;
    spin_lock_irqsave(lock, flags);
    return flags;
}


// mutex
void rust_helper_mutex_init(struct mutex *lock) { mutex_init(lock); }
void rust_helper_mutex_lock(struct mutex *lock) { mutex_lock(lock); }
void rust_helper_mutex_unlock(struct mutex *lock) { mutex_unlock(lock); }


// task
struct task_struct *rust_helper_get_current(void){ return current; }
void rust_helper_get_task_struct(struct task_struct *t){ get_task_struct(t); }
void rust_helper_put_task_struct(struct task_struct *t){ put_task_struct(t); }
int rust_helper_signal_pending(struct task_struct *t){ return signal_pending(t); }


// err
long rust_helper_PTR_ERR(__force const void *ptr){ return PTR_ERR(ptr); }
bool rust_helper_IS_ERR(__force const void *ptr){ return IS_ERR(ptr); }



// dynamically allocate and free per-cpu variables
unsigned int rust_helper_num_online_cpus(void){ return num_online_cpus(); }
long long *rust_helper_alloc_percpu_longlong(void){ return alloc_percpu(long long); }
void rust_helper_free_percpu_longlong(long long *p){ free_percpu(p); }
int rust_helper_get_cpu(void){ return get_cpu(); }
void rust_helper_put_cpu(void){ put_cpu(); }
long long *rust_helper_per_cpu_ptr(long long *p, int cpu){ return per_cpu_ptr(p, cpu); }


// Page
void *rust_helper_kmap(struct page *page){ return kmap(page); }
void rust_helper_kunmap(struct page *page){ return kunmap(page); }
void *rust_helper_kmap_atomic(struct page *page){ return kmap_atomic(page); }
void rust_helper_kunmap_atomic(void *address){ kunmap_atomic(address); }


// block device
void rust_helper_bio_advance_iter_single(const struct bio *bio,
                                         struct bvec_iter *iter,
                                         unsigned int bytes) {
    bio_advance_iter_single(bio, iter, bytes);
}
void *rust_helper_blk_mq_rq_to_pdu(struct request *rq){ return blk_mq_rq_to_pdu(rq); }
struct request *rust_helper_blk_mq_rq_from_pdu(void *pdu) { return blk_mq_rq_from_pdu(pdu);}

//bool rust_helper_slab_is_available(void) { return slab_is_available(); }


// radix_tree
void rust_helper_init_radix_tree(struct xarray *tree, gfp_t gfp_mask)
{
    INIT_RADIX_TREE(tree, gfp_mask);
}

void **rust_helper_radix_tree_iter_init(struct radix_tree_iter *iter,
                                        unsigned long start)
{
    return radix_tree_iter_init(iter, start);
}
void **rust_helper_radix_tree_next_slot(void **slot,
                                        struct radix_tree_iter *iter,
                                        unsigned flags)
{
    return radix_tree_next_slot(slot, iter, flags);
}


// folio
void rust_helper_folio_get(struct folio *folio)
{
    folio_get(folio);
}
void rust_helper_folio_put(struct folio *folio)
{
    folio_put(folio);
}
struct folio *rust_helper_folio_alloc(gfp_t gfp, unsigned int order)
{
    return folio_alloc(gfp, order);
}
struct page *rust_helper_folio_page(struct folio *folio, size_t n)
{
    return folio_page(folio, n);
}
loff_t rust_helper_folio_pos(struct folio *folio)
{
    return folio_pos(folio);
}

size_t rust_helper_folio_size(struct folio *folio)
{
    return folio_size(folio);
}

void rust_helper_folio_lock(struct folio *folio)
{
    folio_lock(folio);
}


bool rust_helper_folio_test_uptodate(struct folio *folio)
{
    return folio_test_uptodate(folio);
}
void rust_helper_folio_mark_uptodate(struct folio *folio)
{
    folio_mark_uptodate(folio);
}
bool rust_helper_folio_test_highmem(struct folio *folio)
{
    return folio_test_highmem(folio);
}
void rust_helper_flush_dcache_folio(struct folio *folio)
{
    flush_dcache_folio(folio);
}

void *rust_helper_kmap_local_folio(struct folio *folio, size_t offset)
{
    return kmap_local_folio(folio, offset);
}
void rust_helper_kunmap_local(const void *vaddr)
{
    kunmap_local(vaddr);
}
struct folio *rust_helper_read_mapping_folio(struct address_space *mapping,
                                             pgoff_t index, struct file *file)
{
    return read_mapping_folio(mapping, index, file);
}


// fs
sector_t rust_helper_bdev_nr_sectors(struct block_device *bdev)
{
    return bdev_nr_sectors(bdev);
}
struct dentry *rust_helper_dget(struct dentry *dentry)
{
    return dget(dentry);
}
loff_t rust_helper_i_size_read(const struct inode *inode)
{
    return i_size_read(inode);
}

void *rust_helper_alloc_inode_sb(struct super_block *sb,
                                 struct kmem_cache *cache, gfp_t gfp)
{
    return alloc_inode_sb(sb, cache, gfp);
}

void rust_helper_inode_lock_shared(struct inode *inode)
{
    inode_lock_shared(inode);
}
void rust_helper_inode_unlock_shared(struct inode *inode)
{
    inode_unlock_shared(inode);
}
void rust_helper_mapping_set_large_folios(struct address_space *mapping)
{
    mapping_set_large_folios(mapping);
}

unsigned int rust_helper_MKDEV(unsigned int major, unsigned int minor)
{
    return MKDEV(major, minor);
}

void rust_helper_i_uid_write(struct inode *inode, uid_t uid)
{
    i_uid_write(inode, uid);
}

void rust_helper_i_gid_write(struct inode *inode, gid_t gid)
{
    i_gid_write(inode, gid);
}
void rust_helper_set_delayed_call(struct delayed_call *call,
                                  void (*fn)(void *), void *arg)
{
    set_delayed_call(call, fn, arg);
}
struct file *rust_helper_get_file(struct file *f)
{
    return get_file(f);
}
unsigned int rust_helper_memalloc_nofs_save(void)
{
    return memalloc_nofs_save();
}

void rust_helper_memalloc_nofs_restore(unsigned int flags)
{
    memalloc_nofs_restore(flags);
}