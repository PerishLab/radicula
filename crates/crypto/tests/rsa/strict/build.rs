use radicula_crypto::Error;
use radicula_crypto::rsa::{Hash, Key, Pss};

use crate::support::{Secret, digest, make, pick, prefix, raw};

pub const MESSAGE: &[u8] = b"strict verification message";

pub fn setup() -> (Secret, Key, usize) {
    let secret = pick(2048);
    let key = make(&secret.modulus, &secret.exponent).unwrap();
    (secret, key, 256)
}

pub fn frame(size: usize, tail: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8, 1];
    out.extend(vec![0xff; size - 3 - tail.len()]);
    out.push(0);
    out.extend(tail);
    out
}

pub fn info(hash: Hash, message: &[u8]) -> Vec<u8> {
    let mut tail = prefix(hash);
    tail.extend(digest(hash, message));
    tail
}

pub fn reject(key: &Key, hash: Hash, signature: &[u8], name: &str) {
    assert_eq!(
        key.pkcs1(hash, MESSAGE, signature),
        Err(Error::Auth),
        "{name}"
    );
}

pub struct Frame {
    pub hash: Hash,
    pub mgf: Hash,
    pub salt: Vec<u8>,
    pub last: u8,
    pub clear: fn(&mut [u8]),
    pub mask: fn(&mut [u8]),
}

pub fn plain(_: &mut [u8]) {}

pub fn stream(hash: Hash, seed: &[u8], length: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut counter = 0u32;
    while out.len() < length {
        let mut block = seed.to_vec();
        block.extend(counter.to_be_bytes());
        out.extend(digest(hash, &block));
        counter += 1;
    }
    out.truncate(length);
    out
}

pub fn encode(frame: &Frame, message: &[u8], size: usize, spare: usize) -> Vec<u8> {
    let hlen = digest(frame.hash, b"").len();
    let mut check = vec![0u8; 8];
    check.extend(digest(frame.hash, message));
    check.extend(&frame.salt);
    let h = digest(frame.hash, &check);
    let split = size - hlen - 1;
    let mut block = vec![0u8; split - frame.salt.len() - 1];
    block.push(1);
    block.extend(&frame.salt);
    (frame.clear)(&mut block);
    for (slot, byte) in block.iter_mut().zip(stream(frame.mgf, &h, split)) {
        *slot ^= byte;
    }
    block[0] &= 0xff >> spare;
    (frame.mask)(&mut block);
    block.extend(&h);
    block.push(frame.last);
    block
}

pub fn standard(salt: usize) -> Frame {
    Frame {
        hash: Hash::Sha256,
        mgf: Hash::Sha256,
        salt: vec![0x5a; salt],
        last: 0xbc,
        clear: plain,
        mask: plain,
    }
}

pub fn pss(secret: &Secret, frame: &Frame) -> Vec<u8> {
    raw(secret, &encode(frame, MESSAGE, 256, 1))
}

pub fn scheme(salt: usize) -> Pss {
    Pss {
        hash: Hash::Sha256,
        mgf: Hash::Sha256,
        salt,
    }
}
