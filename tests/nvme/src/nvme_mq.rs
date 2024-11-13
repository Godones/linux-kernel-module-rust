use alloc::{boxed::Box, sync::Arc};
use core::{
    self,
    cell::SyncUnsafeCell,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering},
};

use kernel::{
    bindings,
    block::mq,
    device::RawDevice,
    error::{linux_err::*, KernelResult as Result},
    pr_info,
    types::{ArcBorrow, AtomicOptionalBoxedPtr, ForeignOwnable},
    BoxExt,
};
use kmacro::vtable;
use pinned_init::*;

use super::{
    nvme_defs::*, nvme_driver_defs::*, nvme_prp::*, nvme_queue::NvmeQueue, DeviceData, MappingData,
    NvmeCommand, NvmeNamespace, NvmeRequest,
};

pub(crate) struct AdminQueueOperations;

#[vtable]
impl mq::Operations for AdminQueueOperations {
    type RequestData = NvmeRequest;
    type RequestDataInit = impl PinInit<Self::RequestData>;
    type QueueData = Box<NvmeNamespace>;
    type HwData = Arc<NvmeQueue<Self>>;
    type TagSetData = Arc<DeviceData>;

    fn new_request_data(
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
    ) -> Self::RequestDataInit {
        // TODO: Can't have these clones inside `pin_init!`, why?
        let device = tagset_data.dev.clone();
        let dma_pool = tagset_data.dma_pool.clone();

        pin_init!(NvmeRequest {
            dma_addr: AtomicU64::new(!0),
            result: AtomicU32::new(0),
            status: AtomicU16::new(0),
            direction: AtomicU32::new(bindings::dma_data_direction_DMA_FROM_DEVICE),
            len: AtomicU32::new(0),
            dev: device,
            cmd: SyncUnsafeCell::new(NvmeCommand::default()),
            sg_count: AtomicU32::new(0),
            page_count: AtomicU32::new(0),
            first_dma: AtomicU64::new(0),
            mapping_data: AtomicOptionalBoxedPtr::new(None),
            dma_pool: dma_pool,
        })
    }

    fn queue_rq(
        hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
        rq: mq::Request<Self>,
        is_last: bool,
    ) -> Result {
        queue_rq(hw_data, queue_data, rq, is_last)
    }

    fn complete(rq: &mq::Request<Self>) {
        complete(rq)
    }

    fn commit_rqs(
        queue: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        _ns: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
    ) {
        queue.write_sq_db(true);
    }

    fn init_hctx(
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
        _hctx_idx: u32,
    ) -> Result<Self::HwData> {
        let queues = tagset_data.queues.lock();
        Ok(queues.admin.as_ref().ok_or(EINVAL)?.clone())
    }
}

pub(crate) struct IoQueueOperations;

#[vtable]
impl mq::Operations for IoQueueOperations {
    type RequestData = NvmeRequest;
    type RequestDataInit = impl PinInit<Self::RequestData>;
    type QueueData = Box<NvmeNamespace>;
    type HwData = Arc<NvmeQueue<Self>>;
    type TagSetData = Arc<DeviceData>;

    fn new_request_data(data: ArcBorrow<'_, DeviceData>) -> Self::RequestDataInit {
        let device = data.dev.clone();
        let dma_pool = data.dma_pool.clone();
        pin_init!(NvmeRequest {
            dma_addr: AtomicU64::new(!0),
            result: AtomicU32::new(0),
            status: AtomicU16::new(0),
            direction: AtomicU32::new(bindings::dma_data_direction_DMA_FROM_DEVICE),
            len: AtomicU32::new(0),
            dev: device,
            cmd: SyncUnsafeCell::new(NvmeCommand::default()),
            sg_count: AtomicU32::new(0),
            page_count: AtomicU32::new(0),
            first_dma: AtomicU64::new(0),
            mapping_data: AtomicOptionalBoxedPtr::new(None),
            dma_pool: dma_pool,
        })
    }

    fn init_hctx(
        tagset_data: ArcBorrow<'_, DeviceData>,
        hctx_idx: u32,
    ) -> Result<Arc<NvmeQueue<Self>>> {
        let queues = tagset_data.queues.lock();
        Ok(queues.io[hctx_idx as usize].clone())
    }

    fn queue_rq(
        io_queue: ArcBorrow<'_, NvmeQueue<Self>>,
        ns: &NvmeNamespace,
        rq: mq::Request<Self>,
        is_last: bool,
    ) -> Result {
        queue_rq(io_queue, ns, rq, is_last)
    }

    fn complete(rq: &mq::Request<Self>) {
        complete(rq)
    }

    fn commit_rqs(io_queue: ArcBorrow<'_, NvmeQueue<Self>>, _ns: &NvmeNamespace) {
        io_queue.write_sq_db(true);
    }

    fn poll(queue: ArcBorrow<'_, NvmeQueue<Self>>) -> bool {
        queue.process_completions()
    }

    fn map_queues(tag_set: &mq::TagSet<Self>) {
        // TODO: Build abstractions for these unsafe calls
        unsafe {
            let device_data: Self::TagSetData =
                Self::TagSetData::from_foreign((*tag_set.raw_tag_set()).driver_data);
            let num_maps = (*tag_set.raw_tag_set()).nr_maps;
            pr_info!("num_maps: {}\n", num_maps);
            let mut queue_offset: u32 = 0;
            let mut irq_offset: u32 = 1; //TODO: Unless we only have 1 vector
            for i in 0..num_maps {
                let queue_count = match i {
                    bindings::hctx_type_HCTX_TYPE_DEFAULT => device_data.irq_queue_count,
                    bindings::hctx_type_HCTX_TYPE_POLL => device_data.poll_queue_count,
                    _ => 0,
                };
                let map = &mut (&mut (*tag_set.raw_tag_set()).map)[i as usize];
                map.nr_queues = queue_count;
                if queue_count == 0 {
                    continue;
                }
                map.queue_offset = queue_offset;
                if i != bindings::hctx_type_HCTX_TYPE_POLL && irq_offset != 0 {
                    bindings::blk_mq_pci_map_queues(
                        map,
                        device_data.pci_dev.as_ptr(),
                        irq_offset as i32,
                    );
                } else {
                    bindings::blk_mq_map_queues(map);
                }
                queue_offset += queue_count;
                irq_offset += queue_count;
            }
        }
        pr_info!("Return from map queues");
    }
}

fn queue_rq<T>(
    io_queue: ArcBorrow<'_, NvmeQueue<T>>,
    ns: &NvmeNamespace,
    rq: mq::Request<T>,
    is_last: bool,
) -> Result
where
    T: mq::Operations<RequestData = NvmeRequest>,
{
    match rq.command() {
        bindings::req_op_REQ_OP_DRV_IN | bindings::req_op_REQ_OP_DRV_OUT => {
            rq.start();
            io_queue.submit_command(unsafe { &*rq.data_ref().cmd.get() }, is_last);
            Ok(())
        }
        bindings::req_op_REQ_OP_FLUSH => {
            rq.start();
            let mut cmd = NvmeCommand::new_flush(ns.id);
            cmd.common.command_id = rq.tag() as u16;
            io_queue.submit_command(&cmd, is_last);
            Ok(())
        }
        bindings::req_op_REQ_OP_WRITE | bindings::req_op_REQ_OP_READ => {
            let (direction, opcode) = if rq.command() == bindings::req_op_REQ_OP_READ {
                (
                    bindings::dma_data_direction_DMA_FROM_DEVICE,
                    NvmeOpcode::read,
                )
            } else {
                (
                    bindings::dma_data_direction_DMA_TO_DEVICE,
                    NvmeOpcode::write,
                )
            };
            let len = rq.payload_bytes();
            // TODO: Handle unwrap
            let offset = rq.bio().unwrap().raw_iter().bi_sector;
            let mut cmd = NvmeCommand {
                rw: NvmeRw {
                    opcode: opcode as _,
                    command_id: rq.tag() as u16,
                    nsid: ns.id.into(),
                    slba: (offset >> (ns.lba_shift - bindings::SECTOR_SHIFT)).into(),
                    length: ((len >> ns.lba_shift) as u16 - 1).into(),
                    ..NvmeRw::default()
                },
            };

            if rq.nr_phys_segments() == 1 {
                //let bv = rq.first_bvec();
                let bio = rq.bio().unwrap();
                let bv = bio.segment_iter().next().unwrap();
                if (bv.offset() % NVME_CTRL_PAGE_SIZE) + len as usize <= NVME_CTRL_PAGE_SIZE * 2 {
                    let dma_addr = unsafe {
                        bindings::dma_map_page_attrs(
                            io_queue.data.dev.ptr,
                            bv.page(),
                            bv.offset() as _,
                            len as _,
                            direction,
                            0,
                        )
                    };
                    if dma_addr == !0 {
                        return Err(ENOMEM);
                    }

                    rq.start();

                    cmd.rw.prp1 = dma_addr.into();
                    if len > (NVME_CTRL_PAGE_SIZE as u32) {
                        cmd.rw.prp2 = (dma_addr + (NVME_CTRL_PAGE_SIZE as u64)).into();
                    }

                    let pdu = rq.data_ref();
                    pdu.dma_addr.store(dma_addr, Ordering::Relaxed);
                    pdu.direction.store(direction, Ordering::Relaxed);
                    pdu.len.store(len, Ordering::Relaxed);

                    io_queue.submit_command(&cmd, is_last);
                    return Ok(());
                }
            }

            let mut md = Box::try_new_atomic(MappingData::default())?;
            let count = rq.map_sg(&mut md.sg)?;
            let dev = &io_queue.data.dev;
            dev.dma_map_sg(&mut md.sg[..count as usize], direction)?;
            let page_count = setup_prps(&io_queue.data, &mut cmd, &mut md, len)?;

            rq.start();

            let pdu = rq.data_ref();
            pdu.sg_count.store(count, Ordering::Relaxed);
            pdu.page_count.store(page_count, Ordering::Relaxed);
            pdu.first_dma
                .store(unsafe { cmd.common.prp2.into() }, Ordering::Relaxed);
            pdu.mapping_data.store(Some(md), Ordering::Relaxed);

            io_queue.submit_command(&cmd, is_last);
            Ok(())
        }

        _ => Err(EIO),
    }
}

fn complete<T>(rq: &mq::Request<T>)
where
    T: mq::Operations<RequestData = NvmeRequest>,
{
    match rq.command() {
        bindings::req_op_REQ_OP_DRV_IN
        | bindings::req_op_REQ_OP_DRV_OUT
        | bindings::req_op_REQ_OP_FLUSH => {
            // We just complete right away if flush completes.
            rq.end_ok();
            return;
        }
        _ => {}
    }

    let pdu = rq.data_ref();

    if let Some(mut md) = pdu.mapping_data.take(Ordering::Relaxed) {
        pdu.dev.dma_unmap_sg(
            &mut md.sg[..pdu.sg_count.load(Ordering::Relaxed) as usize],
            pdu.direction.load(Ordering::Relaxed),
        );
        free_prps(
            pdu.page_count.load(Ordering::Relaxed) as _,
            &md.pages,
            pdu.first_dma.load(Ordering::Relaxed),
            &pdu.dma_pool,
        );
    } else {
        // Unmap the page we mapped.
        unsafe {
            bindings::dma_unmap_page_attrs(
                pdu.dev.ptr,
                pdu.dma_addr.load(Ordering::Relaxed),
                pdu.len.load(Ordering::Relaxed) as _,
                pdu.direction.load(Ordering::Relaxed),
                0,
            )
        };
    }

    // On failure, complete the request immediately with an error.
    let status = pdu.status.load(Ordering::Relaxed);
    if status != 0 {
        pr_info!("Completing with error {:x}\n", status);
        rq.end_err(EIO);
        return;
    }

    rq.end_ok();
}