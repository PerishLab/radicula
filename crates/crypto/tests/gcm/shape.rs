use radicula_crypto::{Aes, Aesgcm, Error};

use crate::support::{Rng, array};

#[test]
fn flips() {
    let mut rng = Rng::new(21);
    for size in [16usize, 32] {
        let key = rng.bytes(size);
        let cipher = Aesgcm::new(&key).unwrap();
        let nonce: [u8; 12] = array(&rng.bytes(12));
        let aad = rng.bytes(21);
        let plain = rng.bytes(70);
        let mut sealed = plain.clone();
        let tag = cipher.seal(&nonce, &aad, &mut sealed).unwrap();
        for index in 0..sealed.len() {
            let mut data = sealed.clone();
            data[index] ^= 1 << (index % 8);
            let result = cipher.open(&nonce, &aad, &mut data, &tag);
            assert_eq!(result, Err(Error::Auth), "cipher {index}");
            data[index] ^= 1 << (index % 8);
            assert_eq!(data, sealed, "cipher {index} untouched");
        }
        for index in 0..aad.len() {
            let mut bent = aad.clone();
            bent[index] ^= 1 << (index % 8);
            let mut data = sealed.clone();
            let result = cipher.open(&nonce, &bent, &mut data, &tag);
            assert_eq!(result, Err(Error::Auth), "aad {index}");
            assert_eq!(data, sealed, "aad {index} untouched");
        }
        for index in 0..tag.len() {
            let mut bent = tag;
            bent[index] ^= 1 << (index % 8);
            let mut data = sealed.clone();
            let result = cipher.open(&nonce, &aad, &mut data, &bent);
            assert_eq!(result, Err(Error::Auth), "tag {index}");
            assert_eq!(data, sealed, "tag {index} untouched");
        }
        let mut other = nonce;
        other[11] ^= 1;
        let mut data = sealed.clone();
        assert_eq!(cipher.open(&other, &aad, &mut data, &tag), Err(Error::Auth));
        let mut data = sealed.clone();
        cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
        assert_eq!(data, plain);
    }
}

#[test]
fn tags() {
    let cipher = Aesgcm::new(&[3u8; 16]).unwrap();
    let nonce = [4u8; 12];
    let mut data = [9u8; 20];
    let tag = cipher.seal(&nonce, b"aad", &mut data).unwrap();
    let sealed = data;
    for size in [0usize, 1, 4, 8, 12, 15, 17, 32] {
        let mut padded = tag.to_vec();
        padded.resize(size.max(tag.len()), 0);
        let wrong = if size < 16 {
            &tag[..size]
        } else {
            &padded[..size]
        };
        let mut copy = sealed;
        let result = cipher.open(&nonce, b"aad", &mut copy, wrong);
        assert_eq!(result, Err(Error::Auth), "size {size}");
        assert_eq!(copy, sealed, "size {size} untouched");
    }
}

#[test]
fn keys() {
    for size in [0usize, 8, 15, 17, 24, 31, 33, 48] {
        assert_eq!(
            Aesgcm::new(&vec![1u8; size]).err(),
            Some(Error::Length),
            "{size}"
        );
    }
}

#[test]
fn lanes() {
    let mut rng = Rng::new(22);
    let key = rng.bytes(32);
    let nonce: [u8; 12] = array(&rng.bytes(12));
    let cipher = Aesgcm::new(&key).unwrap();
    let block = Aes::new(&key).unwrap();
    let mut long = vec![0u8; 300];
    cipher.seal(&nonce, &[], &mut long).unwrap();
    for (index, chunk) in long.chunks(16).enumerate() {
        let mut counter = [0u8; 16];
        counter[..12].copy_from_slice(&nonce);
        counter[12..].copy_from_slice(&(index as u32 + 2).to_be_bytes());
        block.encrypt(&mut counter);
        assert_eq!(chunk, &counter[..chunk.len()], "block {index}");
    }
}

#[test]
fn roundtrip() {
    let mut rng = Rng::new(23);
    let cipher = Aesgcm::new(&rng.bytes(16)).unwrap();
    for size in 0..200usize {
        let nonce: [u8; 12] = array(&rng.bytes(12));
        let plain = rng.bytes(size);
        let aad = rng.bytes(size % 37);
        let mut data = plain.clone();
        let tag = cipher.seal(&nonce, &aad, &mut data).unwrap();
        assert_eq!(data.len(), plain.len());
        if size > 0 {
            assert_ne!(data, plain);
        }
        cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
        assert_eq!(data, plain, "size {size}");
    }
}

#[test]
fn sizes() {
    assert_eq!((Aes::BLOCK, Aesgcm::NONCE, Aesgcm::TAG), (16, 12, 16));
}
