use alloc::{boxed::Box, sync::Arc};
use core::{fmt::Debug, pin::Pin};

use basic::{
    console::*,
    impl_has_timer,
    kernel::{
        block,
        block::{
            bio::Segment,
            mq,
            mq::{GenDisk, MqOperations, TagSet},
        },
        error,
        error::{Error, KernelResult},
        mm::pages::Pages,
        radix_tree::RadixTree,
        sync::{Mutex, SpinLock, UniqueArc},
        time,
        time::hrtimer::{RawTimer, TimerCallback},
        types::ForeignOwnable,
    },
    new_mutex, new_spinlock, SafePtr,
};
use interface::null_block::BlockArgs;
use kmacro::vtable;
use pinned_init::{pin_data, pin_init, InPlaceInit, PinInit};

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
    disk: Pin<Box<Mutex<GenDisk<NullBlkDevice>>>>,
    args: BlockArgs,
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
        let tagset = UniqueArc::try_pin_init(TagSet::try_new_no_alloc(1, (), 256, 1))?.into();

        let disk = Box::pin_init(new_mutex!(add_disk(tagset, args)?, "nullb:disk"))?;
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
        println!("Dropping NullBlkDomain");
    }
}

pub struct NullBlkDevice;
type Tree = RadixTree<Box<Pages<0>>>;

#[pin_data]
pub struct QueueData {
    #[pin]
    tree: SpinLock<Tree>,
    completion_time_nsec: u64,
    irq_mode: IRQMode,
    memory_backed: bool,
}

fn add_disk(
    tagset: Arc<TagSet<NullBlkDevice>>,
    args: &BlockArgs,
) -> KernelResult<GenDisk<NullBlkDevice>> {
    let tree = RadixTree::new()?;
    let mode = args.param_irq_mode.try_into()?;
    let queue_data = Box::pin_init(pin_init!(
    QueueData {
        tree <- new_spinlock!(tree, "rnullb:mem"),
        completion_time_nsec: args.param_completion_time_nsec,
        irq_mode: mode,
        memory_backed: args.param_memory_backed,
    }))?;
    let disk = GenDisk::new_no_alloc(tagset, queue_data);
    Ok(disk)
}

impl NullBlkDevice {
    #[inline(always)]
    fn write(tree: &mut Tree, sector: usize, segment: &Segment<'_>) -> KernelResult {
        let idx = sector >> 3; // TODO: PAGE_SECTOR_SHIFT
        let mut page = if let Some(page) = tree.get_mut(idx as u64) {
            page
        } else {
            tree.try_insert(idx as u64, Box::try_new(Pages::new()?)?)?;
            tree.get_mut(idx as u64).unwrap()
        };

        segment.copy_to_page_atomic(&mut page)?;

        Ok(())
    }

    #[inline(always)]
    fn read(tree: &mut Tree, sector: usize, segment: &mut Segment<'_>) -> KernelResult {
        let idx = sector >> 3; // TODO: PAGE_SECTOR_SHIFT
        if let Some(page) = tree.get(idx as u64) {
            segment.copy_from_page_atomic(page)?;
        }

        Ok(())
    }

    #[inline(never)]
    fn transfer(
        command: block::req_op,
        tree: &mut Tree,
        sector: usize,
        segment: &mut Segment<'_>,
    ) -> KernelResult {
        match command {
            block::req_op_REQ_OP_WRITE => Self::write(tree, sector, segment)?,
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
    type QueueData = Pin<Box<QueueData>>;
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

            let mut sector = rq.sector();
            for bio in rq.bio_iter() {
                for mut segment in bio.segment_iter() {
                    Self::transfer(rq.command(), &mut tree, sector, &mut segment)?;
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
