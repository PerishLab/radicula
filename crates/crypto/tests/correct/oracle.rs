use hkdf::Hkdf as Reference;
use hmac::{Hmac as Keyed, Mac};
use radicula_crypto::{Error, Hkdf, Hmac, Sha256};
use sha2::Digest;

use crate::support::Rng;

const EDGES: [usize; 16] = [
    0, 1, 31, 32, 33, 55, 56, 57, 63, 64, 65, 119, 120, 128, 129, 300,
];

fn tag(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = Keyed::<sha2::Sha256>::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().into()
}

#[test]
fn hash() {
    let mut rng = Rng::new(1);
    let lengths = (0..=300).chain([1000, 4096, 65537]);
    for length in lengths {
        let data = rng.bytes(length);
        let want = sha2::Sha256::digest(&data);
        assert_eq!(Sha256::digest(&data)[..], want[..], "length {length}");
        let cut = rng.below(length + 1);
        let mut hash = Sha256::new();
        hash.update(&data[..cut]);
        let mut twin = hash.clone();
        hash.update(&data[cut..]);
        assert_eq!(hash.finish()[..], want[..], "split {cut} of {length}");
        twin.update(&data[cut..]);
        assert_eq!(twin.finish()[..], want[..], "clone {cut} of {length}");
    }
    let data = rng.bytes(200);
    let mut hash = Sha256::new();
    for byte in &data {
        hash.update(&[*byte]);
    }
    assert_eq!(hash.finish()[..], sha2::Sha256::digest(&data)[..]);
}

#[test]
fn mac() {
    let mut rng = Rng::new(2);
    for size in 0..=200 {
        let key = rng.bytes(size);
        for length in EDGES {
            let data = rng.bytes(length);
            let want = tag(&key, &data);
            let cut = rng.below(length + 1);
            let mut mac = Hmac::new(&key);
            mac.update(&data[..cut]);
            mac.update(&data[cut..]);
            assert_eq!(mac.clone().finish(), want, "key {size} data {length}");
            assert!(mac.verify(&want), "key {size} data {length}");
        }
    }
}

#[test]
fn forged() {
    let mut rng = Rng::new(3);
    let key = rng.bytes(32);
    let data = rng.bytes(100);
    let want = tag(&key, &data);
    let mut mac = Hmac::new(&key);
    mac.update(&data);
    for bit in 0..256 {
        let mut bad = want;
        bad[bit / 8] ^= 1 << (bit % 8);
        assert!(!mac.clone().verify(&bad), "bit {bit}");
    }
    for size in (0..32).chain([33, 64]) {
        let mut other = want.to_vec();
        other.resize(size, 0);
        assert!(!mac.clone().verify(&other), "length {size}");
    }
    assert!(mac.clone().verify(&want));
    let mut other = Hmac::new(&rng.bytes(32));
    other.update(&data);
    assert!(!other.verify(&want));
}

fn derive(rng: &mut Rng, lengths: [usize; 3]) {
    let salt = rng.bytes(lengths[0]);
    let ikm = rng.bytes(lengths[1]);
    let info = rng.bytes(lengths[2]);
    let reference = Reference::<sha2::Sha256>::new(Some(&salt), &ikm);
    let mine = Hkdf::extract(&salt, &ikm);
    for size in [0, 1, 31, 32, 33, 64, 255, 1000, 8160] {
        let mut want = vec![0u8; size];
        reference.expand(&info, &mut want).unwrap();
        let mut got = vec![0xee; size];
        mine.expand(&info, &mut got).unwrap();
        assert_eq!(got, want, "{lengths:?} out {size}");
    }
}

#[test]
fn kdf() {
    let mut rng = Rng::new(4);
    for salt in [0, 1, 13, 32, 63, 64, 65, 100] {
        for ikm in [0, 1, 22, 32, 100] {
            for info in [0, 1, 10, 80, 300] {
                derive(&mut rng, [salt, ikm, info]);
            }
        }
    }
}

#[test]
fn prk() {
    let mut rng = Rng::new(5);
    for size in [32, 33, 64, 65, 100] {
        let prk = rng.bytes(size);
        let reference = Reference::<sha2::Sha256>::from_prk(&prk).unwrap();
        let mine = Hkdf::new(&prk).unwrap();
        let info = rng.bytes(20);
        let mut want = [0u8; 100];
        reference.expand(&info, &mut want).unwrap();
        let mut got = [0u8; 100];
        mine.expand(&info, &mut got).unwrap();
        assert_eq!(got, want, "prk {size}");
    }
    for size in [0, 1, 31] {
        assert_eq!(
            Hkdf::new(&rng.bytes(size)).err(),
            Some(Error::Length),
            "prk {size}"
        );
    }
}

#[test]
fn limit() {
    let mut rng = Rng::new(6);
    let mine = Hkdf::extract(&rng.bytes(16), &rng.bytes(16));
    assert_eq!(Hkdf::LIMIT, 8160);
    for size in [8161, 8192, 10000] {
        let mut out = vec![0xee; size];
        assert_eq!(mine.expand(b"info", &mut out), Err(Error::Length));
        assert!(out.iter().all(|byte| *byte == 0xee), "size {size}");
    }
    let reference = Reference::<sha2::Sha256>::new(None, b"key");
    assert!(reference.expand(b"info", &mut vec![0u8; 8161]).is_err());
}
