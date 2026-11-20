use radicula_crypto::Error;
use radicula_crypto::p256::{Point, ecdh};

use crate::support::{ORDER, Rng, field, hex, load, padded, scalar, subtract, uncompressed};

fn judge(row: &serde_json::Value, counts: &mut [usize; 3]) {
    let secret = padded(&field(row, "private"));
    let id = row["tcId"].as_u64().unwrap();
    let got = ecdh::agree(&secret, &field(row, "public"));
    match row["result"].as_str().unwrap() {
        "valid" => {
            assert_eq!(got.unwrap().to_vec(), field(row, "shared"), "tc {id}");
            counts[0] += 1;
        }
        "invalid" => {
            assert_eq!(got, Err(Error::Point), "tc {id}");
            counts[1] += 1;
        }
        _ => {
            assert_eq!(got, Err(Error::Point), "tc {id}");
            counts[2] += 1;
        }
    }
}

#[test]
fn wycheproof() {
    let data = load("ecdh_secp256r1_ecpoint_test.json");
    let mut counts = [0usize; 3];
    for group in data["testGroups"].as_array().unwrap() {
        for row in group["tests"].as_array().unwrap() {
            judge(row, &mut counts);
        }
    }
    assert_eq!(counts, [330, 24, 1]);
}

#[test]
fn range() {
    let peer = Point::base().encode().unwrap();
    let mut one = [0u8; 32];
    one[31] = 1;
    let below = subtract(&ORDER, &one);
    let mut above = ORDER;
    above[31] += 1;
    for bad in [[0u8; 32], ORDER, above, [0xff; 32]] {
        assert_eq!(ecdh::agree(&bad, &peer), Err(Error::Scalar));
        assert_eq!(ecdh::public(&bad), Err(Error::Scalar));
    }
    assert!(ecdh::agree(&one, &peer).is_ok());
    assert!(ecdh::agree(&below, &peer).is_ok());
    assert_eq!(ecdh::public(&one).unwrap(), peer);
    assert_eq!(
        ecdh::agree(&one, &peer).unwrap().to_vec(),
        peer[1..33].to_vec()
    );
}

#[test]
fn symmetry() {
    let mut rng = Rng::new(11);
    for _ in 0..6 {
        let (left, right) = (scalar(&mut rng), scalar(&mut rng));
        let first = ecdh::agree(&left, &ecdh::public(&right).unwrap()).unwrap();
        let second = ecdh::agree(&right, &ecdh::public(&left).unwrap()).unwrap();
        assert_eq!(first, second);
    }
}

#[test]
fn peers() {
    let secret = [7u8; 32];
    let peer = Point::base().encode().unwrap();
    assert_eq!(ecdh::agree(&secret, &[]), Err(Error::Point));
    assert_eq!(ecdh::agree(&secret, &peer[..64]), Err(Error::Point));
    let mut extra = peer.to_vec();
    extra.push(0);
    assert_eq!(ecdh::agree(&secret, &extra), Err(Error::Point));
    assert_eq!(ecdh::agree(&secret, &[0u8; 65]), Err(Error::Point));
    let mut flipped = peer;
    flipped[40] ^= 1;
    assert_eq!(ecdh::agree(&secret, &flipped), Err(Error::Point));
    let alias = uncompressed(
        &[0u8; 32],
        &hex("66485c780e2f83d72433bd5d84a06bb6541c2af31dae871728bf856a174f93f4"),
    );
    assert!(ecdh::agree(&secret, &alias).is_ok());
}
