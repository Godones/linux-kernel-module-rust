#[allow(
    clippy::all,
    missing_docs,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    improper_ctypes,
    unreachable_pub,
    unsafe_op_in_unsafe_fn
)]
pub use kbind::*;

pub fn rust_helper_errname(_err: core::ffi::c_int) -> *const core::ffi::c_char {
    core::ptr::null()
}

extern "C" {
    #[link_name = "rust_helper_ERR_PTR"]
    pub fn ERR_PTR(err: core::ffi::c_long) -> *mut core::ffi::c_void;
    pub fn rust_helper_rcu_read_unlock();

    pub fn rust_helper_rcu_read_lock();
    pub fn rust_helper_synchronize_rcu();
    pub fn rust_helper_rcu_assign_pointer(
        rcu_data: *const CRcuData,
        new_ptr: *const core::ffi::c_void,
    );
    pub fn rust_helper_rcu_dereference(rcu_data: *const CRcuData) -> *const core::ffi::c_void;

    // srcu
    // void * rust_helper_srcu_dereference(struct rcudata *p,const struct srcu_struct *ssp) {
    #[link_name = "rust_helper_srcu_dereference"]
    pub fn srcu_dereference(
        p: *const CRcuData,
        ssp: *const srcu_struct,
    ) -> *const core::ffi::c_void;

    #[link_name = "rust_helper_spin_lock_init"]
    pub fn spin_lock_init(
        lock: *mut spinlock_t,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    );
    #[link_name = "rust_helper_spin_lock"]
    pub fn spin_lock(lock: *mut spinlock_t);
    #[link_name = "rust_helper_spin_unlock"]
    pub fn spin_unlock(lock: *mut spinlock_t);
    #[link_name = "rust_helper_spin_unlock_irqrestore"]
    pub fn spin_unlock_irqrestore(lock: *mut spinlock_t, flags: core::ffi::c_ulong);
    #[link_name = "rust_helper_spin_lock_irqsave"]
    pub fn spin_lock_irqsave(lock: *mut spinlock_t) -> core::ffi::c_ulong;

    #[link_name = "rust_helper_get_current"]
    pub fn get_current() -> *mut task_struct;
    #[link_name = "rust_helper_get_task_struct"]
    pub fn get_task_struct(t: *mut task_struct);
    #[link_name = "rust_helper_put_task_struct"]
    pub fn put_task_struct(t: *mut task_struct);
    #[link_name = "rust_helper_signal_pending"]
    pub fn signal_pending(t: *mut task_struct) -> core::ffi::c_int;

    // error
    #[link_name = "rust_helper_IS_ERR"]
    pub fn is_err(ptr: *const core::ffi::c_void) -> bool_;
    #[link_name = "rust_helper_PTR_ERR"]
    pub fn ptr_err(ptr: *const core::ffi::c_void) -> core::ffi::c_long;
    // error end

    // Per-cpu
    #[link_name = "rust_helper_num_online_cpus"]
    pub fn num_online_cpus() -> core::ffi::c_uint;
    #[link_name = "rust_helper_alloc_percpu_longlong"]
    pub fn alloc_percpu_longlong() -> *mut core::ffi::c_longlong;
    #[link_name = "rust_helper_free_percpu_longlong"]
    pub fn free_percpu_longlong(p: *mut core::ffi::c_longlong);
    #[link_name = "rust_helper_get_cpu"]
    pub fn get_cpu() -> core::ffi::c_int;
    #[link_name = "rust_helper_put_cpu"]
    pub fn put_cpu();
    #[link_name = "rust_helper_per_cpu_ptr"]
    pub fn per_cpu_ptr(
        p: *mut core::ffi::c_longlong,
        cpu: core::ffi::c_int,
    ) -> *mut core::ffi::c_longlong;
    // Per-cpu end

    // Page
    #[link_name = "rust_helper_kmap"]
    pub fn kmap(page: *mut page) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_kunmap"]
    pub fn kunmap(page: *mut page);
    #[link_name = "rust_helper_kmap_atomic"]
    pub fn kmap_atomic(page: *mut page) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_kunmap_atomic"]
    pub fn kunmap_atomic(address: *mut core::ffi::c_void);

    #[link_name="rust_helper_kmap_local_page"]
    pub fn kmap_local_page(page: *mut page) -> *mut core::ffi::c_void;

    // Page end

    // Block device
    #[link_name = "rust_helper_bio_advance_iter_single"]
    pub fn bio_advance_iter_single(bio: *const bio, iter: *mut bvec_iter, bytes: core::ffi::c_uint);
    #[link_name = "rust_helper_blk_mq_rq_to_pdu"]
    pub fn blk_mq_rq_to_pdu(rq: *mut request) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_blk_mq_rq_from_pdu"]
    pub fn blk_mq_rq_from_pdu(pdu: *mut core::ffi::c_void) -> *mut request;
    // Block device end

    // #[link_name="rust_helper_slab_is_available"]
    // pub fn slab_is_available() -> bool;

    // radix tree
    #[link_name = "rust_helper_init_radix_tree"]
    pub fn init_radix_tree(tree: *mut xarray, gfp_mask: gfp_t);
    #[link_name = "rust_helper_radix_tree_iter_init"]
    pub fn radix_tree_iter_init(
        iter: *mut radix_tree_iter,
        start: core::ffi::c_ulong,
    ) -> *mut *mut core::ffi::c_void;
    #[link_name = "rust_helper_radix_tree_next_slot"]
    pub fn radix_tree_next_slot(
        slot: *mut *mut core::ffi::c_void,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void;

    // folio
    #[link_name = "rust_helper_folio_get"]
    pub fn folio_get(folio: *mut folio);
    #[link_name = "rust_helper_folio_put"]
    pub fn folio_put(folio: *mut folio);
    #[link_name = "rust_helper_folio_alloc"]
    pub fn folio_alloc(gfp: gfp_t, order: core::ffi::c_uint) -> *mut folio;
    #[link_name = "rust_helper_folio_page"]
    pub fn folio_page(folio: *mut folio, n: usize) -> *mut page;
    #[link_name = "rust_helper_folio_pos"]
    pub fn folio_pos(folio: *mut folio) -> loff_t;
    #[link_name = "rust_helper_folio_size"]
    pub fn folio_size(folio: *mut folio) -> usize;

    #[link_name = "rust_helper_folio_lock"]
    pub fn folio_lock(folio: *mut folio);
    #[link_name = "rust_helper_folio_test_uptodate"]
    pub fn folio_test_uptodate(folio: *mut folio) -> bool_;
    #[link_name = "rust_helper_folio_mark_uptodate"]
    pub fn folio_mark_uptodate(folio: *mut folio);
    #[link_name = "rust_helper_folio_test_highmem"]
    pub fn folio_test_highmem(folio: *mut folio) -> bool_;
    #[link_name = "rust_helper_flush_dcache_folio"]
    pub fn flush_dcache_folio(folio: *mut folio);
    #[link_name = "rust_helper_kmap_local_folio"]
    pub fn kmap_local_folio(folio: *mut folio, offset: usize) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_kunmap_local"]
    pub fn kunmap_local(vaddr: *const core::ffi::c_void);
    #[link_name = "rust_helper_read_mapping_folio"]
    pub fn read_mapping_folio(
        mapping: *mut address_space,
        index: core::ffi::c_ulong,
        file: *mut file,
    ) -> *mut folio;

    // fs
    #[link_name = "rust_helper_bdev_nr_sectors"]
    pub fn bdev_nr_sectors(bdev: *mut block_device) -> sector_t;
    #[link_name = "rust_helper_dget"]
    pub fn dget(dentry: *mut dentry) -> *mut dentry;
    #[link_name = "rust_helper_i_size_read"]
    pub fn i_size_read(inode: *const inode) -> loff_t;
    #[link_name = "rust_helper_alloc_inode_sb"]
    pub fn alloc_inode_sb(
        sb: *mut super_block,
        cache: *mut kmem_cache,
        gfp: gfp_t,
    ) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_inode_lock_shared"]
    pub fn inode_lock_shared(inode: *mut inode);
    #[link_name = "rust_helper_inode_unlock_shared"]
    pub fn inode_unlock_shared(inode: *mut inode);
    #[link_name = "rust_helper_mapping_set_large_folios"]
    pub fn mapping_set_large_folios(mapping: *mut address_space);
    #[link_name = "rust_helper_MKDEV"]
    pub fn MKDEV(major: core::ffi::c_uint, minor: core::ffi::c_uint) -> core::ffi::c_uint;
    #[link_name = "rust_helper_i_uid_write"]
    pub fn i_uid_write(inode: *mut inode, uid: uid_t);
    #[link_name = "rust_helper_i_gid_write"]
    pub fn i_gid_write(inode: *mut inode, gid: gid_t);
    #[link_name = "rust_helper_set_delayed_call"]
    pub fn set_delayed_call(
        call: *mut delayed_call,
        fn_: ::core::option::Option<unsafe extern "C" fn(arg1: *mut core::ffi::c_void)>,
        arg: *mut core::ffi::c_void,
    );
    #[link_name = "rust_helper_get_file"]
    pub fn get_file(f: *mut file) -> *mut file;
    #[link_name = "rust_helper_memalloc_nofs_save"]
    pub fn memalloc_nofs_save() -> core::ffi::c_uint;
    #[link_name = "rust_helper_memalloc_nofs_restore"]
    pub fn memalloc_nofs_restore(flags: core::ffi::c_uint);

    //io mem
    #[link_name = "rust_helper_readb"]
    pub fn readb(addr: *const core::ffi::c_void) -> u8_;
    #[link_name = "rust_helper_readw"]
    pub fn readw(addr: *const core::ffi::c_void) -> u16_;
    #[link_name = "rust_helper_readl"]
    pub fn readl(addr: *const core::ffi::c_void) -> u32_;
    #[link_name = "rust_helper_readq"]
    pub fn readq(addr: *const core::ffi::c_void) -> u64_;
    #[link_name = "rust_helper_writeb"]
    pub fn writeb(value: u8_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writew"]
    pub fn writew(value: u16_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writel"]
    pub fn writel(value: u32_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writeq"]
    pub fn writeq(value: u64_, addr: *mut core::ffi::c_void);

    #[link_name = "rust_helper_readb_relaxed"]
    pub fn readb_relaxed(addr: *const core::ffi::c_void) -> u8_;
    #[link_name = "rust_helper_readw_relaxed"]
    pub fn readw_relaxed(addr: *const core::ffi::c_void) -> u16_;
    #[link_name = "rust_helper_readl_relaxed"]
    pub fn readl_relaxed(addr: *const core::ffi::c_void) -> u32_;
    #[link_name = "rust_helper_readq_relaxed"]
    pub fn readq_relaxed(addr: *const core::ffi::c_void) -> u64_;

    #[link_name = "rust_helper_writeb_relaxed"]
    pub fn writeb_relaxed(value: u8_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writew_relaxed"]
    pub fn writew_relaxed(value: u16_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writel_relaxed"]
    pub fn writel_relaxed(value: u32_, addr: *mut core::ffi::c_void);
    #[link_name = "rust_helper_writeq_relaxed"]
    pub fn writeq_relaxed(value: u64_, addr: *mut core::ffi::c_void);

    // blk
    #[link_name = "rust_helper_blk_mq_tag_to_rq"]
    pub fn blk_mq_tag_to_rq(tags: *mut blk_mq_tags, tag: core::ffi::c_uint) -> *mut request;
    #[link_name = "rust_helper_blk_rq_payload_bytes"]
    pub fn blk_rq_payload_bytes(rq: *mut request) -> core::ffi::c_uint;
    #[link_name = "rust_helper_blk_rq_nr_phys_segments"]
    pub fn blk_rq_nr_phys_segments(rq: *mut request) -> core::ffi::c_ushort;

    #[link_name = "rust_helper_dev_name"]
    pub fn dev_name(dev: *const device) -> *const core::ffi::c_char;

    // PCI
    #[link_name = "rust_helper_pci_set_drvdata"]
    pub fn pci_set_drvdata(pdev: *mut pci_dev, data: *mut core::ffi::c_void);
    #[link_name = "rust_helper_pci_get_drvdata"]
    pub fn pci_get_drvdata(pdev: *mut pci_dev) -> *mut core::ffi::c_void;

    #[link_name = "rust_helper_pci_resource_len"]
    pub fn pci_resource_len(pdev: *mut pci_dev, bar: core::ffi::c_int) -> core::ffi::c_ulong;
    #[link_name = "rust_helper_devm_add_action"]
    pub fn devm_add_action(
        dev: *mut device,
        action: Option<unsafe extern "C" fn(arg1: *mut core::ffi::c_void)>,
        data: *mut core::ffi::c_void,
    ) -> core::ffi::c_int;

    #[link_name = "rust_helper_mdelay"]
    pub fn mdelay(ms: u64);
    #[link_name = "rust_helper_num_possible_cpus"]
    pub fn num_possible_cpus() -> core::ffi::c_uint;


    // xarray
    #[link_name="rust_helper_xa_init_flags"]
    pub fn xa_init_flags(xa: *mut xarray, flags: gfp_t);
    #[link_name="rust_helper_xa_empty"]
    pub fn xa_empty(xa: *mut xarray) -> bool_;
    #[link_name="rust_helper_xa_alloc"]
    pub fn xa_alloc(
        xa: *mut xarray,
        id: *mut u32_,
        entry: *mut core::ffi::c_void,
        limit: xa_limit,
        gfp: gfp_t,
    ) -> core::ffi::c_int;
    #[link_name="rust_helper_xa_lock"]
    pub fn xa_lock(xa: *mut xarray);
    #[link_name="rust_helper_xa_unlock"]
    pub fn xa_unlock(xa: *mut xarray);
    #[link_name="rust_helper_xa_err"]
    pub fn xa_err(entry: *mut core::ffi::c_void) -> core::ffi::c_int;

}

#[repr(C)]
#[derive(Debug)]
pub struct CRcuData {
    pub data_ptr: *mut core::ffi::c_void,
}
