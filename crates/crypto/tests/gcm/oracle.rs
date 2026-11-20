use aes::Aes128;
use aes::Aes256;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes_gcm::aead::AeadInPlace;
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use ghash::GHash;
use ghash::universal_hash::UniversalHash;
use radicula_crypto::{Aes, Aesgcm, Ghash};

use crate::support::{Rng, array};

fn expected(key: &[u8], block: &[u8; 16]) -> [u8; 16] {
    let mut out = GenericArray::clone_from_slice(block);
    if key.len() == 16 {
        Aes128::new(GenericArray::from_slice(key)).encrypt_block(&mut out);
    } else {
        Aes256::new(GenericArray::from_slice(key)).encrypt_block(&mut out);
    }
    out.into()
}

#[test]
fn block() {
    let mut rng = Rng::new(11);
    for round in 0..600 {
        let key = match round {
            0 => vec![0u8; 16],
            1 => vec![0xff; 16],
            2 => vec![0u8; 32],
            3 => vec![0xff; 32],
            _ => rng.bytes(if round % 2 == 0 { 16 } else { 32 }),
        };
        let mut block: [u8; 16] = match round {
            0 | 2 => [0u8; 16],
            1 | 3 => [0xff; 16],
            _ => array(&rng.bytes(16)),
        };
        let want = expected(&key, &block);
        Aes::new(&key).unwrap().encrypt(&mut block);
        assert_eq!(block, want, "round {round}");
    }
}

fn reference(key: &[u8; 16], pieces: &[&[u8]]) -> [u8; 16] {
    let mut hash = GHash::new(GenericArray::from_slice(key));
    for piece in pieces {
        hash.update_padded(piece);
    }
    hash.finalize().into()
}

#[test]
fn hash() {
    let mut rng = Rng::new(12);
    for round in 0..600 {
        let key: [u8; 16] = match round {
            0 => [0u8; 16],
            1 => [0xff; 16],
            _ => array(&rng.bytes(16)),
        };
        let size = if round < 2 { 80 } else { rng.below(301) };
        let data = match round {
            0 => vec![0u8; 80],
            1 => vec![0xff; 80],
            _ => rng.bytes(size),
        };
        let want = reference(&key, &[&data]);
        let mut whole = Ghash::new(&key);
        whole.update(&data);
        assert_eq!(whole.finish(), want, "round {round}");
        let mut split = Ghash::new(&key);
        let cut = rng.below(data.len() + 1);
        split.update(&data[..cut]);
        split.update(&data[cut..]);
        assert_eq!(split.finish(), want, "round {round} split");
    }
}

#[test]
fn pads() {
    let mut rng = Rng::new(13);
    for round in 0..300 {
        let key: [u8; 16] = array(&rng.bytes(16));
        let (one, two) = (rng.below(70), rng.below(70));
        let (first, second) = (rng.bytes(one), rng.bytes(two));
        let want = reference(&key, &[&first, &second]);
        let mut hash = Ghash::new(&key);
        hash.update(&first);
        hash.pad();
        hash.update(&second);
        assert_eq!(hash.finish(), want, "round {round}");
    }
}

fn sealed(key: &[u8], nonce: &[u8; 12], aad: &[u8], data: &mut [u8]) -> [u8; 16] {
    let nonce = GenericArray::from_slice(nonce);
    if key.len() == 16 {
        let cipher = Aes128Gcm::new(GenericArray::from_slice(key));
        cipher
            .encrypt_in_place_detached(nonce, aad, data)
            .unwrap()
            .into()
    } else {
        let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
        cipher
            .encrypt_in_place_detached(nonce, aad, data)
            .unwrap()
            .into()
    }
}

#[test]
fn aead() {
    let mut rng = Rng::new(14);
    for round in 0..1500 {
        let key = rng.bytes(if round % 2 == 0 { 16 } else { 32 });
        let nonce: [u8; 12] = array(&rng.bytes(12));
        let (extra, size) = (rng.below(80), rng.below(301));
        let aad = rng.bytes(match round % 5 {
            0 => 0,
            1 => 16,
            2 => 1,
            _ => extra,
        });
        let plain = rng.bytes(if round < 305 { round } else { size });
        let mut want = plain.clone();
        let tag = sealed(&key, &nonce, &aad, &mut want);
        let cipher = Aesgcm::new(&key).unwrap();
        let mut data = plain.clone();
        let made = cipher.seal(&nonce, &aad, &mut data).unwrap();
        assert_eq!((&data, made), (&want, tag), "round {round}");
        cipher.open(&nonce, &aad, &mut data, &made).unwrap();
        assert_eq!(data, plain, "round {round} open");
    }
}
