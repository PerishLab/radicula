use std::hint::black_box;

use radicula_crypto::{Aes, Aesgcm, Ghash};

use crate::control::early;
use crate::paths::STEPS;
use crate::probe::{Fed, Mix, POOL, Pools, clean, leaks, serial};
use crate::tags::{AAD, NONCE, PLAIN, tags};

const SAMPLES: usize = 20_000;
const CLASSES: [(&str, usize, usize); 2] = [("fixed-random", 0, 1), ("random-zero", 1, 2)];
const FIXED: [u8; 32] = [0x5a; 32];

fn pools<const N: usize>(seed: u64) -> [Vec<[u8; N]>; 3] {
    let mut mix = Mix::new(seed);
    let random = (0..POOL)
        .map(|_| std::array::from_fn(|_| mix.byte()))
        .collect();
    let fixed = std::array::from_fn(|index| FIXED[index % 32]);
    [vec![fixed; POOL], random, vec![[0u8; N]; POOL]]
}

#[test]
fn keys() {
    let _held = serial();
    for size in [16usize, 32] {
        for (name, low, high) in CLASSES {
            let items = Pools::new(pools::<32>(201), low, high);
            let work = |key: &[u8; 32]| {
                let mut block = [0x33u8; 16];
                Aes::new(&key[..size]).unwrap().encrypt(&mut block);
                black_box(block);
            };
            clean(
                &format!("aes{} key {name}", size * 8),
                SAMPLES,
                &mut Fed::new(items, FIXED, work),
            );
        }
    }
}

#[test]
fn blocks() {
    let _held = serial();
    for size in [16usize, 32] {
        let cipher = Aes::new(&FIXED[..size]).unwrap();
        for (name, low, high) in CLASSES {
            let items = Pools::new(pools::<16>(202), low, high);
            let work = |block: &[u8; 16]| {
                let mut copy = *block;
                cipher.encrypt(&mut copy);
                black_box(copy);
            };
            clean(
                &format!("aes{} block {name}", size * 8),
                SAMPLES,
                &mut Fed::new(items, [0u8; 16], work),
            );
        }
    }
}

#[test]
fn ghashkey() {
    let _held = serial();
    for (name, low, high) in CLASSES {
        let items = Pools::new(pools::<16>(203), low, high);
        let work = |key: &[u8; 16]| {
            let mut hash = Ghash::new(key);
            hash.update(&[0x77u8; 96]);
            black_box(hash.finish());
        };
        clean(
            &format!("ghash key {name}"),
            SAMPLES,
            &mut Fed::new(items, [0u8; 16], work),
        );
    }
}

#[test]
fn ghashdata() {
    let _held = serial();
    for (name, low, high) in CLASSES {
        let items = Pools::new(pools::<96>(204), low, high);
        let work = |data: &[u8; 96]| {
            let mut hash = Ghash::new(&[0x6bu8; 16]);
            hash.update(data);
            black_box(hash.finish());
        };
        clean(
            &format!("ghash data {name}"),
            SAMPLES,
            &mut Fed::new(items, [0u8; 96], work),
        );
    }
}

fn sealed() -> ([u8; 32], [u8; 16]) {
    let mut data = PLAIN;
    let tag = Aesgcm::new(&FIXED[..16])
        .unwrap()
        .seal(&NONCE, &AAD, &mut data)
        .unwrap();
    (data, tag)
}

#[test]
fn open() {
    let _held = serial();
    let (cipher, tag) = sealed();
    let items = tags(tag, 205);
    let aead = Aesgcm::new(&FIXED[..16]).unwrap();
    for (name, low, high) in [("first-last", 0, 1), ("first-middle", 0, 2)] {
        let pools = Pools::new(items.clone(), low, high);
        let mut data = cipher;
        let work = |given: &[u8; 16]| {
            black_box(aead.open(&NONCE, &AAD, &mut data, given).is_err());
        };
        clean(
            &format!("gcm open {name}"),
            SAMPLES,
            &mut Fed::new(pools, tag, work),
        );
    }
}

#[test]
fn leakyopen() {
    let _held = serial();
    let (cipher, tag) = sealed();
    let items = tags(tag, 206);
    let aead = Aesgcm::new(&FIXED[..16]).unwrap();
    let pools = Pools::new(items, 0, 1);
    let mut data = cipher;
    let work = |given: &[u8; 16]| {
        black_box(aead.open(&NONCE, &AAD, &mut data, given).is_err());
        for _ in 0..STEPS {
            black_box(early(black_box(&tag), black_box(given)));
        }
    };
    leaks("gcm early open", SAMPLES, &mut Fed::new(pools, tag, work));
}

#[test]
fn branchkey() {
    let _held = serial();
    let items = Pools::new(pools::<32>(207), 1, 2);
    let work = |key: &[u8; 32]| {
        let mut block = [0x33u8; 16];
        let cipher = Aes::new(&key[..16]).unwrap();
        cipher.encrypt(&mut block);
        if key[0] == 0 {
            cipher.encrypt(&mut block);
        }
        black_box(block);
    };
    leaks("aes key branch", SAMPLES, &mut Fed::new(items, FIXED, work));
}

#[test]
fn branchdata() {
    let _held = serial();
    let items = Pools::new(pools::<96>(208), 1, 2);
    let work = |data: &[u8; 96]| {
        let mut hash = Ghash::new(&[0x6bu8; 16]);
        hash.update(data);
        if data[0] == 0 {
            hash.update(&[0u8; 16]);
        }
        black_box(hash.finish());
    };
    leaks(
        "ghash data branch",
        SAMPLES,
        &mut Fed::new(items, [0u8; 96], work),
    );
}
