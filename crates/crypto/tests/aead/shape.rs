use radicula_crypto::{Chacha20, Chacha20poly1305, Error, Poly1305};

use crate::support::Rng;

const KEY: [u8; 32] = [7; 32];
const NONCE: [u8; 12] = [9; 12];

fn sealed(rng: &mut Rng, aad: usize, size: usize) -> (Vec<u8>, Vec<u8>, [u8; 16]) {
    let aad = rng.bytes(aad);
    let mut data = rng.bytes(size);
    let tag = Chacha20poly1305::new(&KEY)
        .seal(&NONCE, &aad, &mut data)
        .unwrap();
    (aad, data, tag)
}

fn rejected(aad: &[u8], data: &[u8], tag: &[u8]) -> bool {
    let mut copy = data.to_vec();
    let result = Chacha20poly1305::new(&KEY).open(&NONCE, aad, &mut copy, tag);
    result == Err(Error::Auth) && copy == data
}

#[test]
fn display() {
    assert_eq!(Error::Auth.to_string(), "authentication failed");
    assert_ne!(Error::Auth, Error::Length);
}

#[test]
fn sizes() {
    assert_eq!(Chacha20::KEY, 32);
    assert_eq!(Chacha20::NONCE, 12);
    assert_eq!(Chacha20::BLOCK, 64);
    assert_eq!(Poly1305::KEY, 32);
    assert_eq!(Poly1305::TAG, 16);
    assert_eq!(Chacha20poly1305::KEY, 32);
    assert_eq!(Chacha20poly1305::NONCE, 12);
    assert_eq!(Chacha20poly1305::TAG, 16);
}

#[test]
fn flips() {
    let mut rng = Rng::new(3);
    for (aad, size) in [(0, 0), (5, 1), (16, 16), (21, 70)] {
        let (aad, data, tag) = sealed(&mut rng, aad, size);
        for index in 0..data.len() {
            let mut bad = data.clone();
            bad[index] ^= 1 << (index % 8);
            assert!(rejected(&aad, &bad, &tag), "cipher {index}");
        }
        for index in 0..aad.len() {
            let mut bad = aad.clone();
            bad[index] ^= 1 << (index % 8);
            assert!(rejected(&bad, &data, &tag), "aad {index}");
        }
        for index in 0..16 {
            let mut bad = tag;
            bad[index] ^= 1 << (index % 8);
            assert!(rejected(&aad, &data, &bad), "tag {index}");
        }
    }
}

#[test]
fn tags() {
    let mut rng = Rng::new(4);
    let (aad, data, tag) = sealed(&mut rng, 4, 40);
    assert!(rejected(&aad, &data, &tag[..15]));
    assert!(rejected(&aad, &data, &[]));
    assert!(rejected(&aad, &data, &[&tag[..], &[0]].concat()));
    assert!(rejected(&aad, &data[..39], &tag));
    assert!(rejected(&aad[..3], &data, &tag));
    let mut copy = data.clone();
    let other = Chacha20poly1305::new(&[8; 32]);
    assert_eq!(other.open(&NONCE, &aad, &mut copy, &tag), Err(Error::Auth));
    let result = Chacha20poly1305::new(&KEY).open(&[1; 12], &aad, &mut copy, &tag);
    assert_eq!(result, Err(Error::Auth));
}

#[test]
fn verify() {
    let tag = {
        let mut poly = Poly1305::new(&[3; 32]);
        poly.update(b"message");
        poly.finish()
    };
    let check = |given: &[u8]| {
        let mut poly = Poly1305::new(&[3; 32]);
        poly.update(b"message");
        poly.verify(given)
    };
    assert!(check(&tag));
    assert!(!check(&tag[..15]));
    assert!(!check(&[&tag[..], &[0]].concat()));
    assert!(!check(&[]));
    for index in 0..16 {
        let mut bad = tag;
        bad[index] ^= 0x80;
        assert!(!check(&bad), "byte {index}");
    }
}

#[test]
fn exhausted() {
    let mut data = [0xaa; 65];
    let mut stream = Chacha20::new(&KEY, &NONCE, u32::MAX);
    assert_eq!(stream.apply(&mut data), Err(Error::Length));
    assert_eq!(data, [0xaa; 65]);
    stream.apply(&mut data[..10]).unwrap();
    stream.apply(&mut data[10..64]).unwrap();
    let before = data;
    assert_eq!(stream.apply(&mut data[..1]), Err(Error::Length));
    assert_eq!(data, before);
    stream.apply(&mut []).unwrap();
    let last = Chacha20::block(&KEY, &NONCE, u32::MAX);
    let mut zeros = [0u8; 64];
    Chacha20::new(&KEY, &NONCE, u32::MAX)
        .apply(&mut zeros)
        .unwrap();
    assert_eq!(zeros, last);
    let mut pair = [0u8; 129];
    assert_eq!(
        Chacha20::new(&KEY, &NONCE, u32::MAX - 1).apply(&mut pair),
        Err(Error::Length)
    );
    Chacha20::new(&KEY, &NONCE, u32::MAX - 1)
        .apply(&mut pair[..128])
        .unwrap();
}

#[test]
fn roundtrip() {
    let mut rng = Rng::new(5);
    for size in [0, 1, 63, 64, 65, 1000] {
        let plain = rng.bytes(size);
        let (aad, mut data, tag) = sealed(&mut rng, 9, 0);
        let mut copy = plain.clone();
        let cipher = Chacha20poly1305::new(&KEY);
        let made = cipher.seal(&NONCE, &aad, &mut copy).unwrap();
        assert_eq!(copy.len(), size);
        cipher.open(&NONCE, &aad, &mut copy, &made).unwrap();
        assert_eq!(copy, plain);
        data.clear();
        assert_eq!(tag.len(), 16);
    }
}
