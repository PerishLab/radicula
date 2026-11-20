use radicula_crypto::Error;
use radicula_crypto::rsa::{Hash, Pss};

use crate::support::{hex, raw};

use super::build::{Frame, MESSAGE, encode, pss, scheme, setup, standard};

#[test]
fn encoding() {
    let (secret, key, _) = setup();
    let signature = pss(&secret, &standard(32));
    assert_eq!(key.pss(scheme(32), MESSAGE, &signature), Ok(()));
    let signature = pss(&secret, &standard(0));
    assert_eq!(key.pss(scheme(0), MESSAGE, &signature), Ok(()));
    let signature = pss(
        &secret,
        &Frame {
            mgf: Hash::Sha512,
            ..standard(20)
        },
    );
    let wide = Pss {
        mgf: Hash::Sha512,
        ..scheme(20)
    };
    assert_eq!(key.pss(wide, MESSAGE, &signature), Ok(()));
    assert_eq!(key.pss(scheme(20), MESSAGE, &signature), Err(Error::Auth));
}

#[test]
fn broken() {
    let (secret, key, _) = setup();
    let cases: Vec<(&str, Frame)> = vec![
        (
            "trailer bd",
            Frame {
                last: 0xbd,
                ..standard(32)
            },
        ),
        (
            "trailer 00",
            Frame {
                last: 0x00,
                ..standard(32)
            },
        ),
        (
            "trailer cc",
            Frame {
                last: 0xcc,
                ..standard(32)
            },
        ),
        (
            "separator gone",
            Frame {
                clear: |db| db[db.len() - 33] = 0,
                ..standard(32)
            },
        ),
        (
            "separator early",
            Frame {
                clear: |db| {
                    let at = db.len() - 33;
                    db[at - 1] = 1;
                    db[at] = 0
                },
                ..standard(32)
            },
        ),
        (
            "separator late",
            Frame {
                clear: |db| {
                    let at = db.len() - 33;
                    db[at] = 0;
                    db[at + 1] = 1
                },
                ..standard(32)
            },
        ),
        (
            "nonzero before separator",
            Frame {
                clear: |db| db[3] = 0x55,
                ..standard(32)
            },
        ),
        (
            "nonzero first byte",
            Frame {
                clear: |db| db[0] = 0x01,
                ..standard(32)
            },
        ),
        (
            "salt flipped in block",
            Frame {
                clear: |db| {
                    let at = db.len() - 1;
                    db[at] ^= 1
                },
                ..standard(32)
            },
        ),
        (
            "hash flipped",
            Frame {
                last: 0xbc,
                mask: |db| db[10] ^= 4,
                ..standard(32)
            },
        ),
    ];
    for (name, frame) in cases {
        let signature = pss(&secret, &frame);
        assert_eq!(
            key.pss(scheme(32), MESSAGE, &signature),
            Err(Error::Auth),
            "{name}"
        );
    }
}

#[test]
fn spare() {
    let (secret, key, _) = setup();
    let modulus = hex(&secret.modulus);
    let mut found = 0;
    for tweak in 0..64u8 {
        let frame = Frame {
            salt: vec![tweak; 32],
            mask: |db| db[0] |= 0x80,
            ..standard(32)
        };
        let encoded = encode(&frame, MESSAGE, 256, 1);
        if encoded >= modulus {
            continue;
        }
        let signature = raw(&secret, &encoded);
        assert_eq!(key.pss(scheme(32), MESSAGE, &signature), Err(Error::Auth));
        found += 1;
    }
    assert!(found > 0);
}

#[test]
fn parameters() {
    let (secret, key, _) = setup();
    let signature = pss(&secret, &standard(32));
    for salt in [
        0,
        1,
        20,
        31,
        33,
        64,
        222,
        223,
        224,
        256,
        300,
        usize::MAX,
        usize::MAX - 1,
    ] {
        assert_eq!(
            key.pss(scheme(salt), MESSAGE, &signature),
            Err(Error::Auth),
            "salt {salt}"
        );
    }
    let others = [
        Pss {
            hash: Hash::Sha384,
            ..scheme(32)
        },
        Pss {
            hash: Hash::Sha512,
            ..scheme(32)
        },
        Pss {
            mgf: Hash::Sha384,
            ..scheme(32)
        },
        Pss {
            mgf: Hash::Sha512,
            ..scheme(32)
        },
    ];
    for other in others {
        assert_eq!(
            key.pss(other, MESSAGE, &signature),
            Err(Error::Auth),
            "{other:?}"
        );
    }
    assert_eq!(
        key.pss(scheme(32), b"another", &signature),
        Err(Error::Auth)
    );
    assert_eq!(key.pss(scheme(32), MESSAGE, &signature), Ok(()));
    let mut changed = signature.clone();
    changed[40] ^= 0x10;
    assert_eq!(key.pss(scheme(32), MESSAGE, &changed), Err(Error::Auth));
}
