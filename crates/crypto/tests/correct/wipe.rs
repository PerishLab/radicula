use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::{Hkdf, Hmac, Sha256};

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
fn hash() {
    let mut hash = Sha256::new();
    hash.update(&[0xa5; 100]);
    assert!(cleared(hash));
    assert!(cleared(Sha256::new()));
}

#[test]
fn mac() {
    let mut mac = Hmac::new(&[0xa5; 40]);
    mac.update(&[0x5a; 70]);
    assert!(cleared(mac));
    assert!(cleared(Hmac::new(&[0xa5; 100])));
}

#[test]
fn kdf() {
    assert!(cleared(Hkdf::extract(&[0xa5; 20], &[0x5a; 20])));
    assert!(cleared(Hkdf::new(&[0xa5; 32]).unwrap()));
}
