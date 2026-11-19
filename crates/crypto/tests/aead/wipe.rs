use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::{Chacha20, Chacha20poly1305, Poly1305};

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
fn stream() {
    let mut stream = Chacha20::new(&[0xa5; 32], &[0x5a; 12], 3);
    stream.apply(&mut [0u8; 70]).unwrap();
    assert!(cleared(stream));
    assert!(cleared(Chacha20::new(&[1; 32], &[2; 12], 0)));
}

#[test]
fn mac() {
    let mut poly = Poly1305::new(&[0xa5; 32]);
    poly.update(&[0x5a; 37]);
    assert!(cleared(poly));
    assert!(cleared(Poly1305::new(&[0xa5; 32])));
}

#[test]
fn aead() {
    assert!(cleared(Chacha20poly1305::new(&[0xa5; 32])));
}
