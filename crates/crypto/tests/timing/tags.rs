use std::hint::black_box;

use radicula_crypto::{Chacha20poly1305, Poly1305};

use crate::paths::PAIRS;
use crate::probe::{Fed, Mix, POOL, Pools, clean, serial};

pub const KEY: [u8; 32] = [0x42; 32];
pub const NONCE: [u8; 12] = [0x24; 12];
pub const AAD: [u8; 13] = [0x11; 13];
pub const PLAIN: [u8; 32] = [0x33; 32];
pub const OTHERS: [(&str, usize, usize); 2] = [("first-last", 0, 1), ("first-middle", 0, 2)];

pub fn tags(base: [u8; 16], seed: u64) -> [Vec<[u8; 16]>; 3] {
    let mut mix = Mix::new(seed);
    let mut pools = [Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..POOL {
        let mut first = base;
        let mut last = base;
        let mut middle = base;
        first[0] ^= 1 + (mix.byte() % 255);
        last[15] ^= 1 + (mix.byte() % 255);
        middle[8] ^= 1 + (mix.byte() % 255);
        pools[0].push(first);
        pools[1].push(last);
        pools[2].push(middle);
    }
    pools
}

pub fn equalled(base: [u8; 16], seed: u64) -> [Vec<[u8; 16]>; 3] {
    let [first, last, _] = tags(base, seed);
    [vec![base; POOL], first, last]
}

pub fn sealed() -> ([u8; 32], [u8; 16]) {
    let mut data = PLAIN;
    let tag = Chacha20poly1305::new(&KEY)
        .seal(&NONCE, &AAD, &mut data)
        .unwrap();
    (data, tag)
}

pub fn authentic() -> [u8; 16] {
    let mut poly = Poly1305::new(&KEY);
    poly.update(b"message");
    poly.finish()
}

#[test]
fn poly() {
    let _held = serial();
    let base = authentic();
    let items = equalled(base, 71);
    for (name, low, high) in PAIRS {
        let pools = Pools::new(items.clone(), low, high);
        let work = |tag: &[u8; 16]| {
            let mut poly = Poly1305::new(&KEY);
            poly.update(b"message");
            black_box(poly.verify(tag));
        };
        clean(
            &format!("poly {name}"),
            30_000,
            &mut Fed::new(pools, base, work),
        );
    }
}

#[test]
fn open() {
    let _held = serial();
    let (cipher, tag) = sealed();
    let items = tags(tag, 72);
    for (name, low, high) in OTHERS {
        let pools = Pools::new(items.clone(), low, high);
        let aead = Chacha20poly1305::new(&KEY);
        let mut data = cipher;
        let work = |given: &[u8; 16]| {
            black_box(aead.open(&NONCE, &AAD, &mut data, given).is_err());
        };
        clean(
            &format!("open {name}"),
            30_000,
            &mut Fed::new(pools, tag, work),
        );
    }
}
