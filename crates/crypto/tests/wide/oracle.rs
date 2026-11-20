use hkdf::Hkdf as Reference;
use hmac::{Hmac as Keyed, Mac as Tagger};
use radicula_crypto::Error;
use sha2::{Sha384, Sha512};

use crate::support::{KDF384, KDF512, Kdf, MAC384, MAC512, Mac, Rng};

const EDGES: [usize; 22] = [
    0, 1, 47, 48, 49, 63, 64, 65, 111, 112, 113, 127, 128, 129, 143, 144, 255, 256, 257, 383, 384,
    600,
];
const SIZES: [usize; 14] = [
    0, 1, 47, 48, 49, 64, 127, 128, 129, 255, 1000, 8160, 12240, 16320,
];

type Sample = Option<Vec<u8>>;

struct Oracle {
    tag: fn(&[u8], &[u8]) -> Vec<u8>,
    expand: fn(&[u8], &[u8], &[u8], usize) -> Sample,
    prk: fn(&[u8], &[u8], usize) -> Sample,
}

macro_rules! oracle {
    ($name:ident, $hash:ty) => {
        const $name: Oracle = Oracle {
            tag: |key, data| {
                let mut mac = Keyed::<$hash>::new_from_slice(key).unwrap();
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            },
            expand: |salt, ikm, info, size| {
                let kdf = Reference::<$hash>::new(Some(salt), ikm);
                let mut out = vec![0u8; size];
                kdf.expand(info, &mut out).ok()?;
                Some(out)
            },
            prk: |prk, info, size| {
                let kdf = Reference::<$hash>::from_prk(prk).unwrap();
                let mut out = vec![0u8; size];
                kdf.expand(info, &mut out).ok()?;
                Some(out)
            },
        };
    };
}

oracle!(TAG384, Sha384);
oracle!(TAG512, Sha512);

fn mac(algo: &Mac, oracle: &Oracle, seed: u64) {
    let mut rng = Rng::new(seed);
    for size in (0..=300).chain([1000]) {
        let key = rng.bytes(size);
        for length in EDGES {
            let data = rng.bytes(length);
            let want = (oracle.tag)(&key, &data);
            let cut = rng.below(length + 1);
            let pieces: [&[u8]; 3] = [&data[..cut / 2], &data[cut / 2..cut], &data[cut..]];
            assert_eq!((algo.tag)(&key, &pieces), want, "key {size} data {length}");
            assert!((algo.check)(&key, &data, &want), "key {size} data {length}");
        }
    }
}

fn forged(algo: &Mac, oracle: &Oracle, seed: u64) {
    let mut rng = Rng::new(seed);
    let key = rng.bytes(algo.size);
    let data = rng.bytes(100);
    let want = (oracle.tag)(&key, &data);
    for bit in 0..algo.size * 8 {
        let mut bad = want.clone();
        bad[bit / 8] ^= 1 << (bit % 8);
        assert!(!(algo.check)(&key, &data, &bad), "bit {bit}");
    }
    for size in (0..algo.size).chain([algo.size + 1, 2 * algo.size]) {
        let mut other = want.clone();
        other.resize(size, 0);
        assert!(!(algo.check)(&key, &data, &other), "length {size}");
    }
    assert!((algo.check)(&key, &data, &want));
    assert!(!(algo.check)(&rng.bytes(algo.size), &data, &want));
}

fn derive(algo: &Kdf, oracle: &Oracle, rng: &mut Rng, lengths: [usize; 3]) {
    let salt = rng.bytes(lengths[0]);
    let ikm = rng.bytes(lengths[1]);
    let info = rng.bytes(lengths[2]);
    for size in SIZES {
        let want = (oracle.expand)(&salt, &ikm, &info, size);
        let got = (algo.derive)(&salt, &ikm, &info, size);
        assert_eq!(got.ok(), want, "{lengths:?} out {size}");
    }
}

fn kdf(algo: &Kdf, oracle: &Oracle, seed: u64) {
    let mut rng = Rng::new(seed);
    for salt in [0, 1, 13, 48, 64, 127, 128, 129, 200] {
        for ikm in [0, 1, 22, 48, 64, 200] {
            for info in [0, 1, 10, 80, 300] {
                derive(algo, oracle, &mut rng, [salt, ikm, info]);
            }
        }
    }
}

fn prk(algo: &Kdf, oracle: &Oracle, seed: u64) {
    let mut rng = Rng::new(seed);
    for size in [algo.size, algo.size + 1, 100, 128, 129, 200] {
        let prk = rng.bytes(size);
        let info = rng.bytes(20);
        for length in [0, 1, 100, 1000, algo.limit] {
            let want = (oracle.prk)(&prk, &info, length);
            assert_eq!((algo.direct)(&prk, &info, length).ok(), want, "prk {size}");
        }
    }
    for size in [0, 1, algo.size - 1] {
        let short = rng.bytes(size);
        assert_eq!((algo.direct)(&short, b"info", 10), Err(Error::Length));
    }
}

#[test]
fn mac384() {
    mac(&MAC384, &TAG384, 2);
}

#[test]
fn mac512() {
    mac(&MAC512, &TAG512, 3);
}

#[test]
fn forged384() {
    forged(&MAC384, &TAG384, 4);
}

#[test]
fn forged512() {
    forged(&MAC512, &TAG512, 5);
}

#[test]
fn kdf384() {
    kdf(&KDF384, &TAG384, 6);
}

#[test]
fn kdf512() {
    kdf(&KDF512, &TAG512, 7);
}

#[test]
fn prk384() {
    prk(&KDF384, &TAG384, 8);
}

#[test]
fn prk512() {
    prk(&KDF512, &TAG512, 9);
}
