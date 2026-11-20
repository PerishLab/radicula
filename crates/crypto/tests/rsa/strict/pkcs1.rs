use radicula_crypto::Error;
use radicula_crypto::rsa::Hash;

use crate::support::{Rng, digest, hex, prefix, raw};

use super::build::{MESSAGE, frame, info, reject, setup};

#[test]
fn baseline() {
    let (secret, key, size) = setup();
    let signature = raw(&secret, &frame(size, &info(Hash::Sha256, MESSAGE)));
    assert_eq!(key.pkcs1(Hash::Sha256, MESSAGE, &signature), Ok(()));
    let exact = hex("3031300d060960864801650304020105000420");
    assert_eq!(prefix(Hash::Sha256), exact);
    for hash in [Hash::Sha384, Hash::Sha512] {
        let signature = raw(&secret, &frame(size, &info(hash, MESSAGE)));
        assert_eq!(key.pkcs1(hash, MESSAGE, &signature), Ok(()));
    }
}

#[test]
fn padding() {
    let (secret, key, size) = setup();
    let tail = info(Hash::Sha256, MESSAGE);
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut short = vec![0u8, 1];
    short.extend([0xff; 7]);
    short.push(0);
    short.extend(&tail);
    short.resize(size, 0);
    cases.push(("seven bytes of padding", short));
    let mut broken = frame(size, &tail);
    broken[100] = 0xfe;
    cases.push(("padding byte not ff", broken));
    let mut broken = frame(size, &tail);
    broken[size - tail.len() - 1] = 0xff;
    cases.push(("separator missing", broken));
    for (name, kind) in [("type 00", 0u8), ("type 02", 2), ("type ff", 0xff)] {
        let mut broken = frame(size, &tail);
        broken[1] = kind;
        cases.push((name, broken));
    }
    let mut broken = frame(size, &tail);
    broken[0] = 1;
    cases.push(("leading byte 01", broken));
    let mut broken = frame(size, &tail);
    broken[size - 1] ^= 1;
    cases.push(("digest bit flipped", broken));
    let mut broken = frame(size, &tail);
    broken[size - tail.len() - 1] = 1;
    cases.push(("separator 01", broken));
    for (name, em) in cases {
        let signature = raw(&secret, &em);
        reject(&key, Hash::Sha256, &signature, name);
    }
}

#[test]
fn digestinfo() {
    let (secret, key, size) = setup();
    let hash = Hash::Sha256;
    let digest = digest(hash, MESSAGE);
    let plain = prefix(hash);
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut absent = plain.clone();
    absent.truncate(absent.len() - 4);
    absent[1] -= 2;
    absent[3] -= 2;
    absent.extend([4, 32]);
    cases.push(("no NULL", absent));
    let mut twice = plain.clone();
    twice.truncate(twice.len() - 2);
    twice.splice(17..17, [5, 0]);
    twice[1] += 2;
    twice[3] += 2;
    cases.push(("two NULLs", twice));
    let mut other = plain.clone();
    other[14] = 2;
    cases.push(("oid of sha384", other));
    let mut other = plain.clone();
    other[14] = 9;
    cases.push(("unknown oid", other));
    let mut length = plain.clone();
    length[plain.len() - 1] = 0x21;
    cases.push(("length byte 21", length));
    let mut length = plain.clone();
    length[plain.len() - 1] = 0x1f;
    cases.push(("length byte 1f", length));
    let mut outer = plain.clone();
    outer[1] += 1;
    cases.push(("outer length too big", outer));
    let mut inner = plain.clone();
    inner[3] += 1;
    cases.push(("inner length too big", inner));
    let mut long = plain.clone();
    long.splice(1..2, [0x81, 0x31]);
    cases.push(("long form length", long));
    let mut null = plain.clone();
    let at = null.len() - 3;
    null[at] = 1;
    cases.push(("null replaced", null));
    for (name, head) in cases {
        let mut tail = head;
        tail.extend(&digest);
        let signature = raw(&secret, &frame(size, &tail));
        reject(&key, hash, &signature, name);
    }
}

#[test]
fn trailing() {
    let (secret, key, size) = setup();
    let tail = info(Hash::Sha256, MESSAGE);
    for extra in [vec![0u8], vec![0xff], vec![0; 8], vec![1, 2, 3, 4, 5, 6, 7]] {
        let mut longer = tail.clone();
        longer.extend(&extra);
        let signature = raw(&secret, &frame(size, &longer));
        reject(
            &key,
            Hash::Sha256,
            &signature,
            &format!("trailing {}", extra.len()),
        );
    }
    let mut garbage = vec![0u8, 1, 0xff, 0];
    garbage.extend(&tail);
    garbage.resize(size, 0xaa);
    reject(
        &key,
        Hash::Sha256,
        &raw(&secret, &garbage),
        "short padding with garbage",
    );
}

#[test]
fn hashes() {
    let (secret, key, size) = setup();
    let other = frame(size, &info(Hash::Sha384, MESSAGE));
    let signature = raw(&secret, &other);
    assert_eq!(key.pkcs1(Hash::Sha384, MESSAGE, &signature), Ok(()));
    reject(&key, Hash::Sha256, &signature, "sha384 signature as sha256");
    reject(&key, Hash::Sha512, &signature, "sha384 signature as sha512");
    let signature = raw(
        &secret,
        &frame(size, &info(Hash::Sha256, b"another message")),
    );
    reject(&key, Hash::Sha256, &signature, "other message");
    let mut mixed = prefix(Hash::Sha256);
    mixed.extend(&digest(Hash::Sha384, MESSAGE)[..32]);
    reject(
        &key,
        Hash::Sha256,
        &raw(&secret, &frame(size, &mixed)),
        "truncated sha384",
    );
}

#[test]
fn random() {
    let (secret, key, _) = setup();
    let mut rng = Rng::new(77);
    for round in 0..8 {
        let message = rng.bytes(round * 37);
        let em = frame(256, &info(Hash::Sha256, &message));
        let signature = raw(&secret, &em);
        assert_eq!(key.pkcs1(Hash::Sha256, &message, &signature), Ok(()));
        let mut changed = signature;
        let at = rng.below(256);
        changed[at] ^= 1 << rng.below(8);
        assert_eq!(
            key.pkcs1(Hash::Sha256, &message, &changed),
            Err(Error::Auth)
        );
    }
}
