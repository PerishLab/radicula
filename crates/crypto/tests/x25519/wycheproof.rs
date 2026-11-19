use radicula_crypto::Error;
use radicula_crypto::x25519;

use crate::support::{array, field, load};

#[test]
fn xdh() {
    let root = load("x25519_test.json");
    let mut counts = [0usize; 4];
    for group in root["testGroups"].as_array().unwrap() {
        assert_eq!(group["curve"], "curve25519");
        for case in group["tests"].as_array().unwrap() {
            counts[0] += 1;
            let id = case["tcId"].as_u64().unwrap();
            let (scalar, point) = (
                array(&field(case, "private")),
                array(&field(case, "public")),
            );
            let shared: [u8; 32] = array(&field(case, "shared"));
            assert_eq!(x25519::mul(&scalar, &point), shared, "case {id} mul");
            let zero = shared == [0u8; 32];
            let result = x25519::agree(&scalar, &point);
            if zero {
                counts[3] += 1;
                assert_eq!(result, Err(Error::Zero), "case {id} agree");
            } else {
                assert_eq!(result, Ok(shared), "case {id} agree");
            }
            match case["result"].as_str().unwrap() {
                "valid" => counts[1] += 1,
                "acceptable" => counts[2] += 1,
                other => panic!("case {id}: unexpected result {other}"),
            }
        }
    }
    assert_eq!(counts, [518, 264, 254, 31]);
}
