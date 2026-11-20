use std::mem::{ManuallyDrop, size_of};
use std::slice;

use radicula_crypto::{Hkdf384, Hkdf512, Hmac384, Hmac512, Sha384, Sha512};

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
fn mac384() {
    let mut mac = Hmac384::new(&[0xa5; 40]);
    mac.update(&[0x5a; 200]);
    assert!(cleared(mac));
    assert!(cleared(Hmac384::new(&[0xa5; 300])));
}

#[test]
fn mac512() {
    let mut mac = Hmac512::new(&[0xa5; 40]);
    mac.update(&[0x5a; 200]);
    assert!(cleared(mac));
    assert!(cleared(Hmac512::new(&[0xa5; 300])));
}

#[test]
fn kdf384() {
    assert!(cleared(Hkdf384::extract(&[0xa5; 20], &[0x5a; 20])));
    assert!(cleared(Hkdf384::new(&[0xa5; 48]).unwrap()));
}

#[test]
fn kdf512() {
    assert!(cleared(Hkdf512::extract(&[0xa5; 20], &[0x5a; 20])));
    assert!(cleared(Hkdf512::new(&[0xa5; 64]).unwrap()));
}

#[test]
fn layout() {
    assert_eq!(size_of::<Hmac384>(), 2 * size_of::<Sha384>());
    assert_eq!(size_of::<Hmac512>(), 2 * size_of::<Sha512>());
    assert_eq!(size_of::<Hkdf384>(), size_of::<Hmac384>());
    assert_eq!(size_of::<Hkdf512>(), size_of::<Hmac512>());
}
