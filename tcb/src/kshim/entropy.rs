use alloc::sync::Arc;

use interface::logger::LogDomain;
use kernel::{
    buf::KernelSlicePtrWriter,
    error::KernelResult,
    random,
    sync::{CpuId, LongLongPerCpu},
    sysctl::SysctlStorage,
};
use rref::RRefVec;

pub struct EntropySource {
    log_domain: Arc<dyn LogDomain>,
    counter: LongLongPerCpu,
}

impl EntropySource {
    pub fn new(log_domain: Arc<dyn LogDomain>) -> Self {
        Self {
            log_domain,
            counter: LongLongPerCpu::new(),
        }
    }
}

impl SysctlStorage for EntropySource {
    fn store_value(&self, data: &[u8]) -> (usize, KernelResult<()>) {
        // println!("EntropySource::store_value {:?}", data);
        let str = core::str::from_utf8(data).unwrap();
        CpuId::read(|id| {
            println!("[core: {}]EntropySource::store_value: {}", id, str);
        });
        self.counter.get_with(|counter| {
            println!("counter: {}", *counter);
            *counter += 1;
        });

        let str = core::str::from_utf8(data).unwrap();
        let log_message = RRefVec::from_slice(str.as_bytes());
        let _r = self
            .log_domain
            .log(interface::logger::Level::Info, &log_message);
        random::add_randomness(data);
        (data.len(), Ok(()))
    }

    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, KernelResult<()>) {
        let mut storage = alloc::vec![0; data.len()];
        if let Err(e) = random::getrandom(&mut storage) {
            return (0, Err(e));
        }
        (storage.len(), data.write(&storage))
    }
}
