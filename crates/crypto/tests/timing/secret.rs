use std::hint::black_box;

use radicula_crypto::{Chacha20, Chacha20poly1305, Poly1305};

use crate::probe::{Fed, Mix, POOL, Pools, clean, serial};
use crate::tags::{AAD, KEY, NONCE};

const CLASSES: [(&str, usize, usize); 2] = [("fixed-random", 0, 1), ("random-zero", 1, 2)];

fn classes(seed: u64) -> [Vec<[u8; 32]>; 3] {
    let mut mix = Mix::new(seed);
    let fixed = [0x5au8; 32];
    let mut random = Vec::new();
    for _ in 0..POOL {
        random.push(std::array::from_fn(|_| mix.byte()));
    }
    [vec![fixed; POOL], random, vec![[0u8; 32]; POOL]]
}

#[test]
fn block() {
    let _held = serial();
    for (name, low, high) in CLASSES {
        let pools = Pools::new(classes(81), low, high);
        let work = |key: &[u8; 32]| {
            black_box(Chacha20::block(key, &NONCE, 1));
        };
        clean(
            &format!("block {name}"),
            20_000,
            &mut Fed::new(pools, KEY, work),
        );
    }
}

#[test]
fn mac() {
    let _held = serial();
    for (name, low, high) in CLASSES {
        let pools = Pools::new(classes(82), low, high);
        let work = |key: &[u8; 32]| {
            let mut poly = Poly1305::new(key);
            poly.update(&[0x55; 64]);
            black_box(poly.finish());
        };
        clean(
            &format!("mac {name}"),
            20_000,
            &mut Fed::new(pools, KEY, work),
        );
    }
}

#[test]
fn seal() {
    let _held = serial();
    for (name, low, high) in CLASSES {
        let pools = Pools::new(classes(83), low, high);
        let aead = Chacha20poly1305::new(&KEY);
        let work = |plain: &[u8; 32]| {
            let mut data = *plain;
            black_box(aead.seal(&NONCE, &AAD, &mut data).unwrap());
            black_box(data);
        };
        clean(
            &format!("seal {name}"),
            20_000,
            &mut Fed::new(pools, KEY, work),
        );
    }
}
