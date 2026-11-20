use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::{Sha384, Sha512};

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
fn big() {
    let mut hash = Sha512::new();
    hash.update(&[0xa5; 200]);
    assert!(cleared(hash));
    assert!(cleared(Sha512::new()));
}

#[test]
fn small() {
    let mut hash = Sha384::new();
    hash.update(&[0xa5; 200]);
    assert!(cleared(hash));
    assert!(cleared(Sha384::new()));
}

#[test]
fn layout() {
    assert_eq!(size_of::<Sha512>(), 8 * 8 + 128 + 8 + 8);
    assert_eq!(size_of::<Sha384>(), 8 * 8 + 128 + 8 + 8);
}
