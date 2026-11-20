use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::p256::Point;

fn bytes<T>(slot: &ManuallyDrop<T>) -> Vec<u8> {
    let start = (slot as *const ManuallyDrop<T>).cast::<u8>();
    unsafe { slice::from_raw_parts(start, size_of::<T>()).to_vec() }
}

fn cleared<T>(value: T) -> bool {
    let mut slot = ManuallyDrop::new(value);
    assert!(bytes(&slot).iter().any(|byte| *byte != 0));
    unsafe { ManuallyDrop::drop(&mut slot) };
    bytes(&slot).iter().all(|byte| *byte == 0)
}

#[test]
fn point() {
    assert!(cleared(Point::base()));
    assert!(cleared(Point::base().double()));
    assert!(cleared(Point::base().mul(&[0xa5; 32])));
}
