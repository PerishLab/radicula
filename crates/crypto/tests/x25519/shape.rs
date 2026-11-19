use radicula_crypto::Error;
use radicula_crypto::x25519::{self, BASE, SIZE};

use crate::support::{Rng, array, hex};

const SMALL: [&str; 7] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

#[test]
fn display() {
    assert_eq!(Error::Zero.to_string(), "shared secret is all zero");
    assert_ne!(Error::Zero, Error::Auth);
}

#[test]
fn sizes() {
    assert_eq!(SIZE, 32);
    assert_eq!(BASE[0], 9);
    assert!(BASE[1..].iter().all(|byte| *byte == 0));
}

#[test]
fn small() {
    let mut rng = Rng::new(31);
    for text in SMALL {
        let point: [u8; 32] = array(&hex(text));
        for _ in 0..3 {
            let scalar: [u8; 32] = array(&rng.bytes(32));
            assert_eq!(x25519::mul(&scalar, &point), [0u8; 32], "{text}");
            assert_eq!(x25519::agree(&scalar, &point), Err(Error::Zero), "{text}");
        }
    }
}

#[test]
fn clamp() {
    let mut rng = Rng::new(32);
    for _ in 0..20 {
        let scalar: [u8; 32] = array(&rng.bytes(32));
        let mut twin = scalar;
        twin[0] ^= 7;
        twin[31] ^= 0xc0;
        let point: [u8; 32] = array(&rng.bytes(32));
        assert_eq!(x25519::mul(&scalar, &point), x25519::mul(&twin, &point));
        let mut high = point;
        high[31] ^= 0x80;
        assert_eq!(x25519::mul(&scalar, &point), x25519::mul(&scalar, &high));
        twin[0] ^= 8;
        assert_ne!(x25519::mul(&scalar, &point), x25519::mul(&twin, &point));
    }
}

#[test]
fn accepts() {
    let mut rng = Rng::new(33);
    let scalar: [u8; 32] = array(&rng.bytes(32));
    let public = x25519::base(&scalar);
    let other: [u8; 32] = array(&rng.bytes(32));
    let shared = x25519::agree(&other, &public).unwrap();
    assert_ne!(shared, [0u8; 32]);
    assert_eq!(shared, x25519::mul(&other, &public));
}
