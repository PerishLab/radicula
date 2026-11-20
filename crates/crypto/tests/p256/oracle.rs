use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{ProjectivePoint, PublicKey, SecretKey};
use radicula_crypto::p256::{Point, ecdh, ecdsa};

use crate::support::{ORDER, Rng, array, scalar, subtract};

fn encode(key: &PublicKey) -> [u8; 65] {
    array(key.to_encoded_point(false).as_bytes())
}

#[test]
fn multiply() {
    let mut rng = Rng::new(21);
    for _ in 0..24 {
        let bytes = scalar(&mut rng);
        let secret = SecretKey::from_slice(&bytes).unwrap();
        let want = encode(&secret.public_key());
        assert_eq!(Point::base().mul(&bytes).encode().unwrap(), want);
        assert_eq!(ecdh::public(&bytes).unwrap(), want);
    }
    let two = (ProjectivePoint::GENERATOR + ProjectivePoint::GENERATOR).to_affine();
    let two = PublicKey::from_affine(two).unwrap();
    assert_eq!(Point::base().double().encode().unwrap(), encode(&two));
}

#[test]
fn agree() {
    let mut rng = Rng::new(22);
    for _ in 0..16 {
        let (left, right) = (scalar(&mut rng), scalar(&mut rng));
        let mine = SecretKey::from_slice(&left).unwrap();
        let theirs = SecretKey::from_slice(&right).unwrap().public_key();
        let want = p256::ecdh::diffie_hellman(mine.to_nonzero_scalar(), theirs.as_affine());
        let got = ecdh::agree(&left, &encode(&theirs)).unwrap();
        assert_eq!(got.as_slice(), want.raw_secret_bytes().as_slice());
    }
}

fn flip(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let at = rng.below(out.len());
    out[at] ^= 1 << rng.below(8);
    out
}

#[test]
fn verify() {
    let mut rng = Rng::new(23);
    for round in 0..24 {
        let bytes = scalar(&mut rng);
        let signer = SigningKey::from_slice(&bytes).unwrap();
        let checker = VerifyingKey::from(&signer);
        let key = encode(&PublicKey::from(&checker));
        let message = rng.bytes(round * 7);
        let signature: Signature = signer.sign(&message);
        let wire = signature.to_der();
        let raw: [u8; 64] = array(signature.to_bytes().as_slice());
        assert_eq!(ecdsa::verify(&key, &message, wire.as_bytes()), Ok(()));
        assert_eq!(ecdsa::ieee(&key, &message, &raw), Ok(()));
        if !message.is_empty() {
            let other = flip(&message, &mut rng);
            assert!(ecdsa::verify(&key, &other, wire.as_bytes()).is_err());
            assert!(checker.verify(&other, &signature).is_err());
        }
        let broken = flip(&raw, &mut rng);
        let broken: [u8; 64] = array(&broken);
        let ours = ecdsa::ieee(&key, &message, &broken).is_ok();
        let theirs = Signature::from_slice(&broken)
            .map(|parsed| checker.verify(&message, &parsed).is_ok())
            .unwrap_or(false);
        assert_eq!(ours, theirs);
        assert!(!ours);
        let moved = flip(&key, &mut rng);
        if moved[0] == 4 {
            let theirs = PublicKey::from_sec1_bytes(&moved).is_ok();
            assert_eq!(Point::decode(&moved).is_ok(), theirs);
        }
        assert!(ecdsa::verify(&moved, &message, wire.as_bytes()).is_err());
    }
}

fn spread(words: [u64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, word) in words.iter().enumerate() {
        out[24 - 8 * index..32 - 8 * index].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[test]
fn limbs() {
    let peer = Point::base().mul(&[0x21; 32]).encode().unwrap();
    let theirs = PublicKey::from_sec1_bytes(&peer).unwrap();
    let patterns = [
        [1, 0, 0, 0],
        [0, 1, 0, 0],
        [0, 0, 1, 0],
        [0, 0, 0, 1],
        [1, 1, 0, 0],
        [1, 0, 1, 0],
        [1, 0, 0, 1],
        [0, 1, 1, 0],
        [0, 1, 0, 1],
        [0, 0, 1, 1],
        [1, 2, 0, 3],
        [3, 1, 2, 0],
        [5, 5, 5, 5],
        [u64::MAX, 0, 0, 0],
        [0, u64::MAX, 0, 0],
        [0, 0, u64::MAX, 0],
        [0, 0, 0, 0x7fff_ffff_ffff_ffff],
    ];
    let mut one = [0u8; 32];
    one[31] = 1;
    let mut cases: Vec<[u8; 32]> = patterns.iter().map(|words| spread(*words)).collect();
    cases.push(subtract(&ORDER, &one));
    cases.push(subtract(&ORDER, &spread([2, 0, 0, 0])));
    for bytes in cases {
        let secret = SecretKey::from_slice(&bytes).unwrap();
        assert_eq!(ecdh::public(&bytes).unwrap(), encode(&secret.public_key()));
        let want = p256::ecdh::diffie_hellman(secret.to_nonzero_scalar(), theirs.as_affine());
        let got = ecdh::agree(&bytes, &peer).unwrap();
        assert_eq!(
            got.as_slice(),
            want.raw_secret_bytes().as_slice(),
            "{bytes:?}"
        );
    }
}
