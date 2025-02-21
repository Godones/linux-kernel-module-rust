use alloc::{boxed::Box, format, sync::Arc};
use alloc::alloc::Global;
use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;
use core::{fmt::Debug, pin::Pin};
use core::alloc::Allocator;
use core::any::TypeId;
use core::ops::Deref;
use basic::{bindings, impl_has_timer, kernel::{
    block,
    block::{
        bio::Segment,
        mq,
        mq::{GenDisk, MqOperations, TagSet},
    },
    error,
    error::{linux_err, Error, KernelResult},
    mm::pages::Pages,
    radix_tree::RadixTree,
    sync::{SpinLock, UniqueArc},
    time,
    time::hrtimer::{RawTimer, TimerCallback},
    types::ForeignOwnable,
}, new_spinlock, println, println_color, SafePtr};
use interface::{empty_device::EmptyDeviceDomain, null_block::BlockArgs, DomainType};
use kmacro::vtable;
use pinned_init::{pin_data, pin_init, pinned_drop, InPlaceInit, InPlaceInitIn, PinInit, PinnedDrop};
use shared_heap::DVec;
use spin::Mutex;
use storage::{CustomStorge};

#[derive(Debug)]
enum IRQMode {
    None,
    Soft,
    Timer,
}

impl TryFrom<u8> for IRQMode {
    type Error = Error;

    fn try_from(value: u8) -> KernelResult<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Soft),
            2 => Ok(Self::Timer),
            _ => Err(error::linux_err::EINVAL),
        }
    }
}

pub struct NullBlkDomain {
    disk: Arc<Mutex<GenDisk<NullBlkDevice, CustomStorge>>, CustomStorge>,
    args: BlockArgs,
}

pub struct MultiDomainTest {
    empty_blk: Arc<dyn EmptyDeviceDomain>,
    buf: DVec<u8>,
}

impl Debug for NullBlkDomain {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NullBlkDomain")
    }
}

impl NullBlkDomain {
    pub fn init(args: &BlockArgs) -> KernelResult<Self> {
        println!("Rust null_blk loaded");
        // TODO: Major device number?
        let tagset = storage::get_or_insert_in(
            "nullb:tagset",
            || {
                let v = UniqueArc::try_pin_init_in(TagSet::try_new_no_alloc(1, (), 256, 1),CustomStorge).unwrap();
                let v= Arc::from(v);
                v
            },
        );
        // let tagset = UniqueArc::try_pin_init(TagSet::try_new_no_alloc(1, (), 256, 1))?.into();
        // let disk = Arc::new(Mutex::new(add_disk(tagset, args)?));
        let disk = storage::get_or_insert(
            "nullb:disk",
            || {
                let disk = add_disk(tagset, args).unwrap();
                Mutex::new(disk)
            },
        );

        #[cfg(any(feature = "multi_domain_no", feature = "multi_domain"))]
        init_multi_domain()?;
        Ok(Self {
            disk,
            args: args.clone(),
        })
    }
    pub fn tag_set_with_queue_data(&self) -> KernelResult<(SafePtr, SafePtr)> {
        let disk = self.disk.lock();
        Ok((disk.tagset_ptr(), disk.queue_data_ptr()))
    }

    pub fn set_gen_disk(&self, gen_disk: SafePtr) -> KernelResult<()> {
        let mut disk = self.disk.lock();
        disk.set_gen_disk(gen_disk);

        disk.set_name(format_args!("drnullb{}", 0))?;
        disk.set_capacity(self.args.param_capacity_mib << 11);
        disk.set_queue_logical_block_size(4096);
        disk.set_queue_physical_block_size(4096);
        disk.set_rotational(false);
        Ok(())
    }
}

impl Drop for NullBlkDomain {
    fn drop(&mut self) {
        let disk = storage::remove::<Mutex<GenDisk<NullBlkDevice, CustomStorge>>>("nullb:disk")
            .expect("cannot remove disk");
        drop(disk);

        let tag_set = storage::remove::<TagSet<NullBlkDevice>>("nullb:tagset").expect("cannot remove tagset");
        drop(tag_set);
        println!("Dropping NullBlkDomain");
    }
}

fn add_disk<A:Allocator>(
    tagset: Arc<TagSet<NullBlkDevice>,A>,
    args: &BlockArgs,
) -> KernelResult<GenDisk<NullBlkDevice,A>> {
    let tree = RadixTree::new_in()?;
    let mode = args.param_irq_mode.try_into()?;
    let alloc_page = Mutex::new(Vec::new_in(CustomStorge));
    let queue_data = Box::pin_init_in(pin_init!(
    QueueData {
        tree <- new_spinlock!(tree, "rnullb:mem"),
        completion_time_nsec: args.param_completion_time_nsec,
        irq_mode: mode,
        alloc_page,
        memory_backed: args.param_memory_backed,
    }),CustomStorge)?;
    let disk = GenDisk::new_no_alloc(tagset, queue_data);
    Ok(disk)
}


pub struct NullBlkDevice;
pub type Tree = RadixTree<Arc<Pages<0>,CustomStorge>, CustomStorge>;
pub type PageList = Vec<usize, CustomStorge>;

#[pin_data(PinnedDrop)]
pub struct QueueData {
    #[pin]
    tree: SpinLock<Tree>,
    completion_time_nsec: u64,
    irq_mode: IRQMode,
    memory_backed: bool,
    alloc_page:Mutex<PageList>,
}

#[pinned_drop]
impl PinnedDrop for QueueData{
    fn drop(self: Pin<&mut Self>) {
        let list=  self.alloc_page.lock();
        for idx in list.iter(){
            let key = format!("rnullb:page{}", idx);
            // log::warn!("drop page:{}", key);
            let page = storage::remove::<Pages<0>>(&key).expect("cannot remove page");
            drop(page);
        }
    }
}

impl NullBlkDevice {
    #[inline(always)]
    fn write(tree: &mut Tree, list: &mut PageList,sector: usize, segment: &Segment<'_>) -> KernelResult {
        let idx = sector >> 3; // TODO: PAGE_SECTOR_SHIFT
        let page = if let Some(page) = tree.get(idx as u64) {
            page
        } else {
            let key = format!("rnullb:page{}", idx);
            let page = storage::get_or_insert(
                &key,
                || {
                    let page = Pages::<0>::new().unwrap();
                    page
                },
            );
            tree.try_insert(idx as u64, page)?;
            list.push(idx);
            tree.get(idx as u64).unwrap()
        };
        segment.copy_to_page_atomic(&page)?;
        Ok(())
    }

    #[inline(always)]
    fn read(tree: &mut Tree, sector: usize, segment: &mut Segment<'_>) -> KernelResult {
        #[cfg(any(feature = "multi_domain_no", feature = "multi_domain"))]
        multi_domain_run();

        let idx = sector >> 3; // TODO: PAGE_SECTOR_SHIFT
        if let Some(page) = tree.get(idx as u64) {
            let page = page.deref();
            segment.copy_from_page_atomic(page)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn transfer(
        command: block::req_op,
        tree: &mut Tree,
        list: &mut PageList,
        sector: usize,
        segment: &mut Segment<'_>,
    ) -> KernelResult {
        match command {
            block::req_op_REQ_OP_WRITE => Self::write(tree, list,sector, segment)?,
            block::req_op_REQ_OP_READ => Self::read(tree, sector, segment)?,
            _ => (),
        }
        Ok(())
    }
}

#[pin_data]
pub struct Pdu {
    #[pin]
    timer: time::hrtimer::Timer<Self>,
}

impl TimerCallback for Pdu {
    type Receiver<'a> = Pin<&'a mut Self>;

    fn run<'a>(this: Self::Receiver<'a>) {
        mq::Request::<NullBlkDevice>::request_from_pdu(this).end_ok();
    }
}

impl_has_timer! {
    impl HasTimer<Self> for Pdu { self.timer }
}

#[vtable]
impl MqOperations for NullBlkDevice {
    type RequestData = Pdu;
    type RequestDataInit = impl PinInit<Pdu>;
    type QueueData = Pin<Box<QueueData,CustomStorge>>;
    type HwData = ();
    type TagSetData = ();
    type DomainType = ();

    fn new_request_data(
        _tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
    ) -> Self::RequestDataInit {
        pin_init!( Pdu {
            timer <- time::hrtimer::Timer::new(),
        })
    }

    #[inline(never)]
    fn queue_rq(
        _hw_data: (),
        queue_data: &QueueData,
        rq: mq::Request<Self>,
        _is_last: bool,
    ) -> KernelResult {
        rq.start();
        if queue_data.memory_backed {
            let mut tree = queue_data.tree.lock_irqsave();
            let mut list = queue_data.alloc_page.lock();
            let mut sector = rq.sector();
            for bio in rq.bio_iter() {
                for mut segment in bio.segment_iter() {
                    Self::transfer(rq.command(), &mut tree,&mut list, sector, &mut segment)?;
                    sector += segment.len() >> 9; // TODO: SECTOR_SHIFT
                }
            }
        }

        match queue_data.irq_mode {
            IRQMode::None => rq.end_ok(),
            IRQMode::Soft => rq.complete(),
            IRQMode::Timer => rq.data().schedule(queue_data.completion_time_nsec),
        }

        Ok(())
    }

    fn commit_rqs(
        _hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        _queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
    ) {
    }

    fn complete(rq: &mq::Request<Self>) {
        rq.end_ok();
    }

    fn init_hctx(
        _tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
        _hctx_idx: u32,
    ) -> KernelResult<Self::HwData> {
        Ok(())
    }
}

pub static MULTI_DOMAIN: spin::Mutex<Option<MultiDomainTest>> = spin::Mutex::new(None);

#[cfg(feature = "multi_domain")]
fn init_multi_domain() -> KernelResult {
    let buf = DVec::new(0, 4096);
    let empty_blk = basic::get_domain("empty_device").ok_or(linux_err::EINVAL)?;
    let empty_blk = match empty_blk {
        DomainType::EmptyDeviceDomain(empty_blk) => empty_blk,
        _ => return Err(linux_err::EINVAL),
    };
    let multi_domain = MultiDomainTest { empty_blk, buf };
    let mut lock = MULTI_DOMAIN.lock();
    lock.replace(multi_domain);
    println!("multi_domain init success");
    Ok(())
}

#[cfg(feature = "multi_domain_no")]
fn init_multi_domain() -> KernelResult {
    use null::NullDeviceDomainImpl;
    let buf = DVec::new(8, 4096);
    let empty_blk = Arc::new(NullDeviceDomainImpl::new());
    let multi_domain = MultiDomainTest { empty_blk, buf };
    let mut lock = MULTI_DOMAIN.lock();
    lock.replace(multi_domain);
    println!("multi_domain_no init success");
    Ok(())
}

const CROSS_NUM: usize = 1;

#[cfg(any(feature = "multi_domain_no", feature = "multi_domain"))]
fn multi_domain_run() {
    let lock = MULTI_DOMAIN.lock();
    let domain = lock.as_ref().unwrap();
    let buf = &domain.buf;
    let empty_blk = &domain.empty_blk;
    for _ in 0..CROSS_NUM {
        let _res = empty_blk.write(buf);
    }
}