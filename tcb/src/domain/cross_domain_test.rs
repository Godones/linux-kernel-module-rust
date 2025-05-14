use alloc::sync::Arc;

use interface::{empty_device::EmptyDeviceDomain, DomainType, DomainTypeRaw};
use shared_heap::DBox;

use crate::domain_helper::query_domain;

const ITER: usize = 10_000_000;

pub fn get_rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn test_cross_domain() {
    let empty_device = query_domain("empty_device").unwrap();
    let domain = match empty_device {
        DomainType::EmptyDeviceDomain(empty_device) => empty_device,
        _ => {
            pr_err!("Failed to get empty device domain");
            return;
        }
    };
    let start = get_rdtsc();
    println!("test_cross_domain");
    for _ in 0..ITER {
        let res = domain.no_arg();
        if res.is_err() {
            println!("no_arg error: {:?}", res);
            break;
        }
    }
    let elapse = get_rdtsc() - start;
    println!(
        "domain.no_arg: avg: {}, total: {}, iter: {}",
        elapse as f64 / ITER as f64,
        elapse,
        ITER
    );
    // let start = get_rdtsc();
    // for _ in 0..ITER {
    //     let res = domain.one_arg(1);
    //     if res.is_err() {
    //         println!("one_arg error: {:?}", res);
    //         break;
    //     }
    // }
    // let elapse = get_rdtsc() - start;
    // println!(
    //     "domain.one_arg: avg: {}, total: {}, iter: {}",
    //     elapse as f64 / ITER as f64,
    //     elapse,
    //     ITER
    // );
    // // assert_eq!(domain.one_arg(12321).unwrap(), 12321 + 1);

    // let start = get_rdtsc();
    // let mut x = DBox::new(0usize);
    // for _ in 0..ITER {
    //     let res= domain.one_darg(x);
    //     if res.is_err() {
    //         println!("one_rref error: {:?}", res);
    //         break;
    //     }
    //     x = res.unwrap();
    // }
    // let elapse = get_rdtsc() - start;
    // println!(
    //     "domain.one_rref: avg: {}, total: {}, iter: {}",
    //     elapse as f64 / ITER as f64,
    //     elapse,
    //     ITER
    // );
    // assert_eq!(*domain.one_darg(x).unwrap(), ITER + 1);
}
