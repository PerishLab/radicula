use radicula_crypto::Error;
use radicula_crypto::rsa::Key;

fn full(bits: usize) -> Vec<u8> {
    let mut out = vec![0xff; bits / 8];
    if !bits.is_multiple_of(8) {
        out.insert(0, 0xff >> (8 - bits % 8));
    }
    out
}

fn with(bytes: &[u8], at: usize, value: u8) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[at] = value;
    out
}

const E: [u8; 3] = [1, 0, 1];

#[test]
fn modulus() {
    let good = full(2048);
    assert!(Key::new(&good, &E).is_ok());
    let cases: [(&str, Vec<u8>); 12] = [
        ("empty", vec![]),
        ("zero", vec![0; 300]),
        ("even", with(&good, 255, 0xfe)),
        ("even 4096", with(&full(4096), 511, 0x00)),
        ("2040 bits", full(2040)),
        ("2047 bits", full(2047)),
        ("1024 bits", full(1024)),
        ("4097 bits", full(4097)),
        ("8192 bits", full(8192)),
        ("513 bytes", vec![0xff; 513]),
        ("one", vec![1]),
        ("short odd", vec![0x7f, 0xff]),
    ];
    for (name, bytes) in cases {
        assert_eq!(Key::new(&bytes, &E).err(), Some(Error::Key), "{name}");
    }
}

#[test]
fn sizes() {
    for bits in [2048, 2049, 2055, 2056, 3000, 3072, 4095, 4096] {
        let key = Key::new(&full(bits), &E).unwrap();
        assert_eq!(key.bits(), bits);
    }
    assert_eq!(Key::MIN, 2048);
    assert_eq!(Key::MAX, 4096);
}

#[test]
fn leading() {
    let plain = full(2048);
    let mut padded = vec![0u8; 5];
    padded.extend(&plain);
    assert_eq!(Key::new(&padded, &E).unwrap().bits(), 2048);
    let mut long = vec![0u8; 600];
    long.extend(&plain);
    assert_eq!(Key::new(&long, &[0, 0, 0, 1, 0, 1]).unwrap().bits(), 2048);
    assert!(Key::new(&plain, &[0, 0, 0, 0, 3]).is_ok());
}

#[test]
fn exponent() {
    let modulus = full(2048);
    let bad: [(&str, &[u8]); 14] = [
        ("empty", &[]),
        ("zero", &[0]),
        ("zeros", &[0, 0, 0]),
        ("one", &[1]),
        ("two", &[2]),
        ("four", &[4]),
        ("even", &[1, 0, 0]),
        ("2^32 + 1", &[1, 0, 0, 0, 1]),
        ("2^32", &[1, 0, 0, 0, 0]),
        ("2^32 + 3", &[1, 0, 0, 0, 3]),
        ("2^32 + 255", &[1, 0, 0, 0, 0xff]),
        ("2^39 + 3", &[0x80, 0, 0, 0, 3]),
        ("wide", &[0xff; 8]),
        ("long", &[1, 0, 0, 0, 0, 0, 0, 0, 3]),
    ];
    for (name, bytes) in bad {
        assert_eq!(Key::new(&modulus, bytes).err(), Some(Error::Key), "{name}");
    }
    let good: [(&str, &[u8]); 7] = [
        ("three", &[3]),
        ("five", &[5]),
        ("65537", &[1, 0, 1]),
        ("65537 padded", &[0, 0, 1, 0, 1]),
        ("17", &[0, 17]),
        ("2^32 - 1", &[0xff; 4]),
        ("3 padded", &[0, 0, 0, 0, 0, 3]),
    ];
    for (name, bytes) in good {
        assert!(Key::new(&modulus, bytes).is_ok(), "{name}");
    }
}
