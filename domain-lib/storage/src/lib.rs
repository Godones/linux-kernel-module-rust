#![feature(allocator_api)]
#![feature(downcast_unchecked)]
#![no_std]
#![no_main]
extern crate alloc;
use alloc::{boxed::Box, sync::Arc};
use core::{alloc::Allocator, any::Any};

use spin::Once;
pub trait SendAllocator: Allocator + Send + Sync {}
type ArcValueType = Arc<dyn Any + Send + Sync, DataStorageHeap>;

pub trait DomainDataStorage: Send + Sync {
    fn insert(&self, key: &str, value: ArcValueType) -> Option<ArcValueType>;
    fn get(&self, key: &str) -> Option<ArcValueType>;
    fn remove(&self, key: &str) -> Option<ArcValueType>;
}

pub trait StorageBuilder {
    fn build() -> &'static dyn SendAllocator;
}

pub type DataStorageHeap = &'static dyn SendAllocator;
static DATA_ALLOCATOR: Once<DataStorageHeap> = Once::new();

pub fn init_data_allocator(allocator: &'static dyn SendAllocator) {
    DATA_ALLOCATOR.call_once(|| allocator);
    log::error!("init data allocator success");
}

impl StorageBuilder for DataStorageHeap {
    fn build() -> &'static dyn SendAllocator {
        *DATA_ALLOCATOR.get().unwrap()
    }
}

#[allow(dead_code)]
pub struct StorageArg {
    pub allocator: DataStorageHeap,
    pub storage: Box<dyn DomainDataStorage>,
}

impl StorageArg {
    pub fn new(allocator: DataStorageHeap, storage: Box<dyn DomainDataStorage>) -> Self {
        Self { allocator, storage }
    }
}

#[cfg(feature = "impl")]
mod __private {
    use alloc::{boxed::Box, sync::Arc};
    use core::any::Any;

    use spin::Once;

    use crate::{DataStorageHeap, DomainDataStorage, StorageBuilder};

    pub fn insert_data<T: Any + Send + Sync>(
        key: &str,
        value: T,
    ) -> Option<Arc<T, DataStorageHeap>> {
        let arc = Arc::new_in(value, DataStorageHeap::build());
        let old = DATABASE.get().unwrap().insert(key, arc);
        let old_arc = match old {
            Some(arc) => Some(unsafe { arc.downcast_unchecked::<T>() }),
            None => None,
        };
        old_arc
    }

    pub fn get_data<T: Any + Send + Sync>(key: &str) -> Option<Arc<T, DataStorageHeap>> {
        let res = DATABASE
            .get()
            .unwrap()
            .get(key)
            .and_then(|arc_wrapper| unsafe {
                let res = arc_wrapper.downcast_unchecked::<T>();
                Some(res)
            });
        res
    }

    pub fn get_or_insert_with_data<T: Any + Send + Sync, F: FnOnce() -> T>(
        key: &str,
        f: F,
    ) -> Arc<T, DataStorageHeap> {
        let arc = get_data::<T>(&key);
        match arc {
            Some(arc) => arc,
            None => {
                let value = f();
                let arc = Arc::new_in(value, DataStorageHeap::build());
                DATABASE.get().unwrap().insert(key, arc.clone());
                arc
            }
        }
    }

    pub fn remove_data<T: Any + Send + Sync>(key: &str) -> Option<Arc<T, DataStorageHeap>> {
        let res = DATABASE
            .get()
            .unwrap()
            .remove(key)
            .and_then(|value| unsafe {
                let strong_count = Arc::strong_count(&value);
                log::error!("remove_data: {:?}, ref count: {}", key, strong_count);
                assert!(strong_count >= 2);
                let value = value.downcast_unchecked::<T>();
                let value = if strong_count != 2 {
                    // decrement strong count to 2
                    let raw = Arc::into_raw(value);
                    for _ in 0..(strong_count - 2) {
                        Arc::decrement_strong_count_in(raw, DataStorageHeap::build());
                    }
                    Arc::from_raw_in(raw, DataStorageHeap::build())
                } else {
                    value
                };
                let strong_count = Arc::strong_count(&value);
                log::error!("remove_data: {:?}, ref count: {}", key, strong_count);
                Some(value)
            });
        res
    }

    static DATABASE: Once<Box<dyn DomainDataStorage>> = Once::new();

    pub fn init_database(database: Box<dyn DomainDataStorage>) {
        DATABASE.call_once(|| database);
        log::error!("init database success");
    }
}

#[cfg(feature = "impl")]
pub use __private::*;
