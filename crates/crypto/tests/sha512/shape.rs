use radicula_crypto::{Sha384, Sha512};

#[test]
fn sizes() {
    assert_eq!(Sha384::SIZE, 48);
    assert_eq!(Sha384::BLOCK, 128);
    assert_eq!(Sha512::SIZE, 64);
    assert_eq!(Sha512::BLOCK, 128);
    assert_eq!(Sha384::digest(b"x").len(), Sha384::SIZE);
    assert_eq!(Sha512::digest(b"x").len(), Sha512::SIZE);
}

#[test]
fn fresh() {
    let mut made = Sha512::default();
    let mut fresh = Sha512::new();
    made.update(b"abc");
    fresh.update(b"abc");
    assert_eq!(made.finish(), fresh.finish());
    let mut made = Sha384::default();
    let mut fresh = Sha384::new();
    made.update(b"abc");
    fresh.update(b"abc");
    assert_eq!(made.finish(), fresh.finish());
}

#[test]
fn whole() {
    let data = [0x42u8; 300];
    let mut hash = Sha512::new();
    hash.update(&data);
    assert_eq!(hash.finish(), Sha512::digest(&data));
    let mut hash = Sha384::new();
    hash.update(&data);
    assert_eq!(hash.finish(), Sha384::digest(&data));
}

#[test]
fn split() {
    let mut first = Sha512::new();
    first.update(&[1u8; 100]);
    let mut second = first.clone();
    first.update(&[2u8; 50]);
    second.update(&[3u8; 50]);
    let mut base = [1u8; 150];
    base[100..].fill(2);
    assert_eq!(first.finish(), Sha512::digest(&base));
    base[100..].fill(3);
    assert_eq!(second.finish(), Sha512::digest(&base));
}

#[test]
fn clones() {
    let mut first = Sha384::new();
    first.update(&[1u8; 130]);
    let mut second = first.clone();
    first.update(&[2u8; 3]);
    second.update(&[3u8; 3]);
    let mut base = [1u8; 133];
    base[130..].fill(2);
    assert_eq!(first.finish(), Sha384::digest(&base));
    base[130..].fill(3);
    assert_eq!(second.finish(), Sha384::digest(&base));
}

#[test]
fn separate() {
    assert_ne!(Sha384::digest(b"abc")[..], Sha512::digest(b"abc")[..48]);
}

#[test]
fn empty() {
    let mut hash = Sha512::new();
    hash.update(b"");
    hash.update(b"abc");
    hash.update(b"");
    assert_eq!(hash.finish(), Sha512::digest(b"abc"));
}
