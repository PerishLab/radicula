use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305 as Reference, Key, Nonce};
use radicula_crypto::{Chacha20, Chacha20poly1305, Poly1305};

use crate::support::{Rng, array};

const EDGES: [usize; 14] = [0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 300];

fn reference(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], data: &mut [u8]) -> [u8; 16] {
    let cipher = Reference::new(Key::from_slice(key));
    let tag = cipher
        .encrypt_in_place_detached(Nonce::from_slice(nonce), aad, data)
        .unwrap();
    tag.into()
}

#[test]
fn aead() {
    let mut rng = Rng::new(7);
    for size in 0..=300 {
        for length in EDGES {
            let key: [u8; 32] = array(&rng.bytes(32));
            let nonce: [u8; 12] = array(&rng.bytes(12));
            let (aad, plain) = (rng.bytes(length), rng.bytes(size));
            let mut want = plain.clone();
            let tag = reference(&key, &nonce, &aad, &mut want);
            let cipher = Chacha20poly1305::new(&key);
            let mut data = plain.clone();
            let made = cipher.seal(&nonce, &aad, &mut data).unwrap();
            assert_eq!((&data, made), (&want, tag), "plain {size} aad {length}");
            cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
            assert_eq!(data, plain, "plain {size} aad {length} open");
        }
    }
}

#[test]
fn stream() {
    let mut rng = Rng::new(8);
    for size in 0..=300 {
        let key: [u8; 32] = array(&rng.bytes(32));
        let nonce: [u8; 12] = array(&rng.bytes(12));
        let counter = if size % 3 == 0 {
            rng.next() as u32
        } else {
            size as u32
        };
        let plain = rng.bytes(size);
        let mut want = plain.clone();
        let mut twin = chacha20::ChaCha20::new(&key.into(), &nonce.into());
        twin.seek(u64::from(counter) * 64);
        twin.apply_keystream(&mut want);
        let mut data = plain.clone();
        let mut stream = Chacha20::new(&key, &nonce, counter);
        let cut = rng.below(size + 1);
        stream.apply(&mut data[..cut]).unwrap();
        stream.apply(&mut data[cut..]).unwrap();
        assert_eq!(data, want, "size {size} counter {counter}");
    }
}

#[test]
fn mac() {
    let mut rng = Rng::new(9);
    for size in 0..=300 {
        let key: [u8; 32] = array(&rng.bytes(32));
        let message = rng.bytes(size);
        let tag: [u8; 16] = poly1305::Poly1305::new(&key.into())
            .compute_unpadded(&message)
            .into();
        let mut poly = Poly1305::new(&key);
        poly.update(&message);
        assert_eq!(poly.finish(), tag, "size {size}");
    }
}

#[test]
fn extremes() {
    let words = [0u8, 0xff, 0x80, 0x7f, 0x01, 0xfe];
    for fill in words {
        for keyfill in words {
            let key = [keyfill; 32];
            let message = vec![fill; 16 * 6 + 5];
            let tag: [u8; 16] = poly1305::Poly1305::new(&key.into())
                .compute_unpadded(&message)
                .into();
            let mut poly = Poly1305::new(&key);
            poly.update(&message);
            assert_eq!(poly.finish(), tag, "fill {fill:#x} key {keyfill:#x}");
        }
    }
}
