use radicula_crypto::Error;
use radicula_crypto::p256::Point;

use crate::support::{hex, padded, uncompressed};

const GX: &str = "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296";
const GY: &str = "4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5";
const PRIME: &str = "ffffffff00000001000000000000000000000000ffffffffffffffffffffffff";

fn generator() -> [u8; 65] {
    uncompressed(&hex(GX), &hex(GY))
}

#[test]
fn roundtrip() {
    assert_eq!(Point::base().encode().unwrap(), generator());
    let point = Point::decode(&generator()).unwrap();
    assert_eq!(point.encode().unwrap(), generator());
    assert_eq!(Point::SIZE, 65);
}

#[test]
fn shapes() {
    let good = generator();
    for length in [0usize, 1, 2, 33, 64, 66, 97] {
        let mut bytes = good.to_vec();
        bytes.resize(length, 0);
        assert_eq!(
            Point::decode(&bytes).err(),
            Some(Error::Point),
            "length {length}"
        );
    }
    for prefix in [0u8, 1, 2, 3, 5, 6, 7, 0xff] {
        let mut bytes = good;
        bytes[0] = prefix;
        assert_eq!(
            Point::decode(&bytes).err(),
            Some(Error::Point),
            "prefix {prefix}"
        );
    }
}

#[test]
fn identity() {
    assert_eq!(Point::decode(&[0u8]).err(), Some(Error::Point));
    assert_eq!(Point::decode(&[0u8; 65]).err(), Some(Error::Point));
    let mut zero = [0u8; 65];
    zero[0] = 4;
    assert_eq!(Point::decode(&zero).err(), Some(Error::Point));
}

#[test]
fn offcurve() {
    let mut bytes = generator();
    for index in [1usize, 16, 32, 33, 48, 64] {
        for bit in 0..8 {
            let mut changed = bytes;
            changed[index] ^= 1 << bit;
            assert_eq!(
                Point::decode(&changed).err(),
                Some(Error::Point),
                "{index}:{bit}"
            );
        }
    }
    bytes[64] = bytes[64].wrapping_add(1);
    assert_eq!(Point::decode(&bytes).err(), Some(Error::Point));
}

#[test]
fn unreduced() {
    let prime = hex(PRIME);
    assert_eq!(
        Point::decode(&uncompressed(&prime, &hex(GY))).err(),
        Some(Error::Point)
    );
    assert_eq!(
        Point::decode(&uncompressed(&hex(GX), &prime)).err(),
        Some(Error::Point)
    );
    assert_eq!(
        Point::decode(&uncompressed(&[0xff; 32], &[0xff; 32])).err(),
        Some(Error::Point)
    );
    let alias = hex("66485c780e2f83d72433bd5d84a06bb6541c2af31dae871728bf856a174f93f4");
    assert_eq!(
        Point::decode(&uncompressed(&prime, &alias)).err(),
        Some(Error::Point)
    );
    assert!(Point::decode(&uncompressed(&[0u8; 32], &alias)).is_ok());
}

#[test]
fn zeros() {
    assert_eq!(
        Point::decode(&uncompressed(&[0u8; 32], &[0u8; 32])).err(),
        Some(Error::Point)
    );
    let mut one = [0u8; 32];
    one[31] = 1;
    assert_eq!(
        Point::decode(&uncompressed(&one, &[0u8; 32])).err(),
        Some(Error::Point)
    );
    assert_eq!(
        Point::decode(&uncompressed(&[0u8; 32], &one)).err(),
        Some(Error::Point)
    );
}

fn fit(text: &str) -> Option<[u8; 32]> {
    let even = if text.len() % 2 == 1 {
        format!("0{text}")
    } else {
        text.to_string()
    };
    let bytes = hex(&even);
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    (bytes.len() - start <= 32).then(|| padded(&bytes))
}

#[test]
fn knownanswer() {
    let text = std::fs::read_to_string("vectors/p256/PKV.rsp").unwrap();
    let (mut passed, mut failed) = (0, 0);
    for block in text.split("\n\n").filter(|block| block.contains("Qx =")) {
        let line = |label: &str| {
            block
                .lines()
                .find_map(|line| line.strip_prefix(label))
                .map(|rest| rest.trim_start_matches([' ', '=']).trim().to_string())
                .unwrap()
        };
        let verdict = line("Result");
        let (Some(x), Some(y)) = (fit(&line("Qx")), fit(&line("Qy"))) else {
            assert!(verdict.starts_with('F'), "wide {verdict}");
            failed += 1;
            continue;
        };
        let key = uncompressed(&x, &y);
        match Point::decode(&key) {
            Ok(point) => {
                assert!(verdict.starts_with('P'), "accepted {verdict}");
                assert_eq!(point.encode().unwrap(), key);
                passed += 1;
            }
            Err(error) => {
                assert!(verdict.starts_with('F'), "rejected {verdict}");
                assert_eq!(error, Error::Point);
                failed += 1;
            }
        }
    }
    assert_eq!((passed, failed), (4, 8));
}

#[test]
fn smally() {
    let x = hex("d7325d7646cd60d80a92738ceb345f844cffaf35841022cab176f692de8de1d7");
    let mut five = [0u8; 32];
    five[31] = 5;
    let key = uncompressed(&x, &five);
    assert_eq!(Point::decode(&key).unwrap().encode().unwrap(), key);
    let shifted = hex("ffffffff00000001000000000000000000000001000000000000000000000004");
    assert_eq!(
        Point::decode(&uncompressed(&x, &shifted)).err(),
        Some(Error::Point)
    );
}
