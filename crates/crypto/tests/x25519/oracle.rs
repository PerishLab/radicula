use radicula_crypto::x25519;

use crate::support::{Rng, array};

const EDGES: [&str; 12] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "0900000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "0000000000000000000000000000000000000000000000000000000000000040",
];

fn reference(scalar: &[u8; 32], point: &[u8; 32]) -> [u8; 32] {
    x25519_dalek::x25519(*scalar, *point)
}

#[test]
fn random() {
    let mut rng = Rng::new(21);
    for round in 0..600 {
        let scalar: [u8; 32] = array(&rng.bytes(32));
        let point: [u8; 32] = array(&rng.bytes(32));
        assert_eq!(
            x25519::mul(&scalar, &point),
            reference(&scalar, &point),
            "round {round}"
        );
    }
}

#[test]
fn edges() {
    let mut rng = Rng::new(22);
    let points: Vec<[u8; 32]> = EDGES
        .iter()
        .map(|text| array(&crate::support::hex(text)))
        .collect();
    let mut scalars = vec![[0u8; 32], [0xff; 32], [0x80; 32], [0x7f; 32], [1; 32]];
    for _ in 0..4 {
        scalars.push(array(&rng.bytes(32)));
    }
    for scalar in &scalars {
        for point in &points {
            assert_eq!(x25519::mul(scalar, point), reference(scalar, point));
        }
    }
}

#[test]
fn bases() {
    let mut rng = Rng::new(23);
    for _ in 0..200 {
        let scalar: [u8; 32] = array(&rng.bytes(32));
        assert_eq!(x25519::base(&scalar), reference(&scalar, &x25519::BASE));
    }
}

#[test]
fn symmetry() {
    let mut rng = Rng::new(24);
    for _ in 0..100 {
        let first: [u8; 32] = array(&rng.bytes(32));
        let second: [u8; 32] = array(&rng.bytes(32));
        let (one, two) = (x25519::base(&first), x25519::base(&second));
        assert_eq!(x25519::mul(&first, &two), x25519::mul(&second, &one));
    }
}
