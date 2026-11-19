use radicula_crypto::{Error, Hkdf, Hmac, Sha256};

#[test]
fn display() {
    let text = Error::Length.to_string();
    assert!(text.contains("length"), "{text}");
    let boxed: Box<dyn std::error::Error> = Box::new(Error::Length);
    assert_eq!(boxed.to_string(), text);
}

#[test]
fn default() {
    let mut fresh = Sha256::new();
    let mut made = Sha256::default();
    fresh.update(b"abc");
    made.update(b"abc");
    assert_eq!(fresh.finish(), made.finish());
}

#[test]
fn sizes() {
    assert_eq!(Sha256::SIZE, 32);
    assert_eq!(Sha256::BLOCK, 64);
    assert_eq!(Hmac::SIZE, 32);
    assert_eq!(Hkdf::LIMIT, 255 * 32);
}
