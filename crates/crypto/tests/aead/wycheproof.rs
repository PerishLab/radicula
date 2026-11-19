use radicula_crypto::{Chacha20poly1305, Error};

use crate::support::{array, field, load};

#[test]
fn aead() {
    let root = load("chacha20_poly1305_test.json");
    let (mut valid, mut invalid, mut sizes) = (0, 0, 0);
    for group in root["testGroups"].as_array().unwrap() {
        assert_eq!(group["tagSize"], 128);
        for case in group["tests"].as_array().unwrap() {
            let id = case["tcId"].as_u64().unwrap();
            let nonce = field(case, "iv");
            if nonce.len() != 12 {
                assert_eq!(case["result"], "invalid", "case {id}");
                sizes += 1;
                continue;
            }
            let cipher = Chacha20poly1305::new(&array(&field(case, "key")));
            let nonce: [u8; 12] = array(&nonce);
            let (aad, tag) = (field(case, "aad"), field(case, "tag"));
            let (plain, sealed) = (field(case, "msg"), field(case, "ct"));
            if case["result"] == "valid" {
                valid += 1;
                let mut data = plain.clone();
                let made = cipher.seal(&nonce, &aad, &mut data).unwrap();
                assert_eq!((&data, &made[..]), (&sealed, &tag[..]), "case {id}");
                let mut data = sealed.clone();
                cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
                assert_eq!(data, plain, "case {id} open");
            } else {
                invalid += 1;
                let mut data = sealed.clone();
                let result = cipher.open(&nonce, &aad, &mut data, &tag);
                assert_eq!(result, Err(Error::Auth), "case {id}");
                assert_eq!(data, sealed, "case {id} untouched");
            }
        }
    }
    assert_eq!((valid, invalid, sizes), (256, 60, 9));
}
