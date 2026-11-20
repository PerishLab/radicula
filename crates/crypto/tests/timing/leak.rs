use std::hint::black_box;

use radicula_crypto::p256::ecdh;
use radicula_crypto::x25519;
use radicula_crypto::{Chacha20poly1305, Poly1305};

use crate::control::early;
use crate::paths::{PAIRS, STEPS};
use crate::probe::{Fed, Pools, leaks, serial};
use crate::scalar::{CURVES, FIRSTS, PAIRS as CLASSES, POINT, orders, peer, scalars};
use crate::tags::{AAD, KEY, NONCE, OTHERS, PLAIN, authentic, equalled, tags};

#[test]
fn poly() {
    let _held = serial();
    let base = authentic();
    let items = equalled(base, 91);
    for (name, low, high) in PAIRS {
        let pools = Pools::new(items.clone(), low, high);
        let work = |tag: &[u8; 16]| {
            let mut poly = Poly1305::new(&KEY);
            poly.update(b"message");
            black_box(early(&poly.finish(), black_box(tag)));
        };
        leaks(
            &format!("poly {name}"),
            30_000,
            &mut Fed::new(pools, base, work),
        );
    }
}

#[test]
fn open() {
    let _held = serial();
    let (_, tag) = crate::tags::sealed();
    let items = tags(tag, 92);
    let (name, low, high) = OTHERS[0];
    let pools = Pools::new(items, low, high);
    let aead = Chacha20poly1305::new(&KEY);
    let work = |given: &[u8; 16]| {
        let mut data = PLAIN;
        let want = aead.seal(&NONCE, &AAD, &mut data).unwrap();
        for _ in 0..STEPS {
            black_box(early(&want, black_box(given)));
        }
    };
    leaks(
        &format!("open {name}"),
        30_000,
        &mut Fed::new(pools, tag, work),
    );
}

#[test]
fn scalar() {
    let _held = serial();
    let (label, first) = FIRSTS[1];
    let (name, low, high) = CLASSES[1];
    let pools = Pools::new(scalars(111, first), low, high);
    let work = |key: &[u8; 32]| {
        black_box(x25519::mul(key, &POINT));
        if key[1] & 1 == 1 {
            black_box(x25519::mul(key, &POINT));
        }
    };
    leaks(
        &format!("scalar {label} {name}"),
        1_000,
        &mut Fed::new(pools, first, work),
    );
}

#[test]
fn agree() {
    let _held = serial();
    let (label, first) = CURVES[0];
    let (name, low, high) = CLASSES[0];
    let peer = peer();
    let pools = Pools::new(orders(131, first), low, high);
    let work = |key: &[u8; 32]| {
        black_box(ecdh::agree(key, &peer).unwrap());
        if key[0] != first[0] {
            black_box(ecdh::agree(key, &peer).unwrap());
        }
    };
    leaks(
        &format!("agree {label} {name}"),
        300,
        &mut Fed::new(pools, first, work),
    );
}
