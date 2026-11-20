use std::hint::black_box;

use radicula_crypto::p256::{Point, ecdh};
use radicula_crypto::x25519;

use crate::probe::{Fed, Mix, POOL, Pools, clean, serial};

pub const POINT: [u8; 32] = [
    0xe6, 0xdb, 0x68, 0x67, 0x58, 0x30, 0x30, 0xdb, 0x35, 0x94, 0xc1, 0xa4, 0x24, 0xb1, 0x5f, 0x7c,
    0x72, 0x66, 0x24, 0xec, 0x26, 0xb3, 0x35, 0x3b, 0x10, 0xa9, 0x03, 0xa6, 0xd0, 0xab, 0x1c, 0x4c,
];
const SAMPLES: usize = if cfg!(debug_assertions) {
    3_000
} else {
    30_000
};
pub const PAIRS: [(&str, usize, usize); 2] = [("first-second", 0, 1), ("first-third", 0, 2)];

fn random(mix: &mut Mix) -> [u8; 32] {
    std::array::from_fn(|_| mix.byte())
}

pub fn scalars(seed: u64, first: [u8; 32]) -> [Vec<[u8; 32]>; 3] {
    let mut mix = Mix::new(seed);
    let second: Vec<[u8; 32]> = (0..POOL).map(|_| random(&mut mix)).collect();
    [vec![first; POOL], second, vec![[0u8; 32]; POOL]]
}

pub const FIRSTS: [(&str, [u8; 32]); 2] = [("fixed", [0x5au8; 32]), ("ones", [0xffu8; 32])];

#[test]
fn scalar() {
    let _held = serial();
    for (label, first) in FIRSTS {
        for (name, low, high) in PAIRS {
            let pools = Pools::new(scalars(101, first), low, high);
            let work = |key: &[u8; 32]| {
                black_box(x25519::mul(key, &POINT));
            };
            clean(
                &format!("{label} {name}"),
                SAMPLES,
                &mut Fed::new(pools, first, work),
            );
        }
    }
}

const CURVE: usize = if cfg!(debug_assertions) { 400 } else { 4_000 };
pub const DENSE: [u8; 32] = [0x7fu8; 32];
pub const SPARSE: [u8; 32] = {
    let mut out = [0u8; 32];
    out[31] = 1;
    out
};
pub const CURVES: [(&str, [u8; 32]); 2] = [("fixed", [0x5au8; 32]), ("dense", DENSE)];

pub fn orders(seed: u64, first: [u8; 32]) -> [Vec<[u8; 32]>; 3] {
    let mut mix = Mix::new(seed);
    let second = (0..POOL)
        .map(|_| {
            let mut out = random(&mut mix);
            out[0] %= 0xff;
            out
        })
        .collect();
    [vec![first; POOL], second, vec![SPARSE; POOL]]
}

pub fn peer() -> [u8; 65] {
    Point::base().mul(&[0x33u8; 32]).encode().unwrap()
}

#[test]
fn agree() {
    let _held = serial();
    let peer = peer();
    for (label, first) in CURVES {
        for (name, low, high) in PAIRS {
            let pools = Pools::new(orders(121, first), low, high);
            let work = |key: &[u8; 32]| {
                black_box(ecdh::agree(key, &peer).unwrap());
            };
            clean(
                &format!("agree {label} {name}"),
                CURVE,
                &mut Fed::new(pools, first, work),
            );
        }
    }
}
