use radicula_crypto::Error;
use radicula_crypto::rsa::{Hash, Key, Pss};

use crate::support::make;
use crate::vector::cavp::records;

#[test]
fn errors() {
    assert_eq!(
        Error::Key.to_string(),
        "public key is outside the range the construction defines"
    );
    assert_ne!(Error::Key, Error::Auth);
    assert_ne!(Hash::Sha256, Hash::Sha384);
    assert_ne!(Hash::Sha384, Hash::Sha512);
}

#[test]
fn copies() {
    let scheme = Pss {
        hash: Hash::Sha256,
        mgf: Hash::Sha512,
        salt: 7,
    };
    let other = scheme;
    assert_eq!(scheme, other);
    assert_ne!(scheme, Pss { salt: 8, ..scheme });
    assert_ne!(
        scheme,
        Pss {
            mgf: Hash::Sha256,
            ..scheme
        }
    );
    assert_ne!(
        scheme,
        Pss {
            hash: Hash::Sha384,
            ..scheme
        }
    );
}

#[test]
fn cloned() {
    let row = records("SigVer15_186-3.rsp")
        .into_iter()
        .find(|row| row.bits == 2048 && row.hash == "SHA256" && row.pass)
        .unwrap();
    let key: Key = make(&row.modulus, &row.exponent).unwrap();
    let twin = key.clone();
    assert_eq!(twin.bits(), key.bits());
    for public in [&key, &twin] {
        assert_eq!(
            public.pkcs1(Hash::Sha256, &row.message, &row.signature),
            Ok(())
        );
        assert_eq!(
            public.pkcs1(Hash::Sha384, &row.message, &row.signature),
            Err(Error::Auth)
        );
        assert_eq!(
            public.pkcs1(Hash::Sha512, &row.message, &row.signature),
            Err(Error::Auth)
        );
    }
}

#[test]
fn failures() {
    let row = records("SigVer15_186-3.rsp")
        .into_iter()
        .find(|row| row.bits == 2048 && row.hash == "SHA256" && row.pass)
        .unwrap();
    let key = make(&row.modulus, &row.exponent).unwrap();
    let mut message = row.message.clone();
    message.push(0);
    assert_eq!(
        key.pkcs1(Hash::Sha256, &message, &row.signature),
        Err(Error::Auth)
    );
    let scheme = Pss {
        hash: Hash::Sha256,
        mgf: Hash::Sha256,
        salt: 32,
    };
    assert_eq!(
        key.pss(scheme, &row.message, &row.signature),
        Err(Error::Auth)
    );
}
