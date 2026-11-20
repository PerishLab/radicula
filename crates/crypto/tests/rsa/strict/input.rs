use radicula_crypto::Error;
use radicula_crypto::rsa::{Hash, Pss};

use crate::support::{hex, raw};

use super::build::{MESSAGE, frame, info, reject, setup};

#[test]
fn lengths() {
    let (secret, key, size) = setup();
    let signature = raw(&secret, &frame(size, &info(Hash::Sha256, MESSAGE)));
    assert_eq!(key.pkcs1(Hash::Sha256, MESSAGE, &signature), Ok(()));
    let mut longer = signature.clone();
    longer.push(0);
    let mut leading = vec![0u8];
    leading.extend(&signature);
    let mut twice = signature.clone();
    twice.extend(&signature);
    let cases: [(&str, Vec<u8>); 6] = [
        ("empty", vec![]),
        ("one byte short", signature[1..].to_vec()),
        ("last byte dropped", signature[..size - 1].to_vec()),
        ("one byte long", longer),
        ("leading zero byte", leading),
        ("twice as long", twice),
    ];
    for (name, bytes) in cases {
        reject(&key, Hash::Sha256, &bytes, name);
    }
}

#[test]
fn range() {
    let (secret, key, size) = setup();
    let modulus = hex(&secret.modulus);
    let mut above = modulus.clone();
    above[size - 1] = above[size - 1].wrapping_add(1);
    let mut topped = modulus.clone();
    topped[0] = 0xff;
    let mut zero = vec![0u8; size];
    let mut one = vec![0u8; size];
    one[size - 1] = 1;
    zero[0] = 0;
    let cases: [(&str, Vec<u8>); 6] = [
        ("equal to n", modulus.clone()),
        ("n plus one", above),
        ("all ff", vec![0xff; size]),
        ("top of range", topped),
        ("zero", zero),
        ("one", one),
    ];
    for (name, bytes) in cases {
        reject(&key, Hash::Sha256, &bytes, name);
        let scheme = Pss {
            hash: Hash::Sha256,
            mgf: Hash::Sha256,
            salt: 32,
        };
        assert_eq!(
            key.pss(scheme, MESSAGE, &bytes),
            Err(Error::Auth),
            "{name} pss"
        );
    }
}
