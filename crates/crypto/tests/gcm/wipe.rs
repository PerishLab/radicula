use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::{Aes, Aesgcm, Ghash};

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
fn block() {
    assert!(cleared(Aes::new(&[0xa5; 16]).unwrap()));
    assert!(cleared(Aes::new(&[0x5a; 32]).unwrap()));
}

#[test]
fn hash() {
    let mut hash = Ghash::new(&[0xa5; 16]);
    hash.update(&[0x5a; 37]);
    assert!(cleared(hash));
    assert!(cleared(Ghash::new(&[0xa5; 16])));
}

#[test]
fn aead() {
    assert!(cleared(Aesgcm::new(&[0xa5; 16]).unwrap()));
    assert!(cleared(Aesgcm::new(&[0x5a; 32]).unwrap()));
}
