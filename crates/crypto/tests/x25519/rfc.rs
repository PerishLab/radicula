use radicula_crypto::x25519::{self, BASE};

use crate::support::{array, field, load};

fn rows(name: &str) -> Vec<serde_json::Value> {
    load("rfc7748.json")[name].as_array().unwrap().clone()
}

fn iterate(count: u64) -> [u8; 32] {
    let (mut scalar, mut point) = (BASE, BASE);
    for _ in 0..count {
        let next = x25519::mul(&scalar, &point);
        point = scalar;
        scalar = next;
    }
    scalar
}

fn expected(count: u64) -> Vec<u8> {
    let rows = rows("iter");
    let row = rows.iter().find(|row| row["count"] == count).unwrap();
    field(row, "output")
}

#[test]
fn mul() {
    let rows = rows("mul");
    assert_eq!(rows.len(), 2);
    for row in rows {
        let out = x25519::mul(
            &array(&field(&row, "scalar")),
            &array(&field(&row, "point")),
        );
        assert_eq!(out[..], field(&row, "output")[..], "{}", row["source"]);
    }
}

#[test]
fn once() {
    assert_eq!(iterate(1)[..], expected(1)[..]);
}

#[test]
fn thousand() {
    assert_eq!(iterate(1000)[..], expected(1000)[..]);
}

#[test]
#[ignore = "one million ladder runs; use cargo test --release -- --ignored"]
fn million() {
    assert_eq!(iterate(1_000_000)[..], expected(1_000_000)[..]);
}

#[test]
fn agree() {
    let row = &rows("dh")[0];
    let (alice, bob) = (array(&field(row, "alice")), array(&field(row, "bob")));
    let (known, other) = (field(row, "alicepublic"), field(row, "bobpublic"));
    assert_eq!(x25519::base(&alice)[..], known[..]);
    assert_eq!(x25519::base(&bob)[..], other[..]);
    let shared = field(row, "shared");
    assert_eq!(
        x25519::agree(&alice, &array(&other)).unwrap()[..],
        shared[..]
    );
    assert_eq!(x25519::agree(&bob, &array(&known)).unwrap()[..], shared[..]);
}
