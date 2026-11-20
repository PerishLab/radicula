use radicula_crypto::{Aesgcm, Error};

use crate::support::{array, field, load};

#[test]
fn aead() {
    let root = load("gcm/aes_gcm_test.json");
    let mut count = [0usize; 4];
    for group in root["testGroups"].as_array().unwrap() {
        for case in group["tests"].as_array().unwrap() {
            let id = case["tcId"].as_u64().unwrap();
            let key = field(case, "key");
            let nonce = field(case, "iv");
            let (aad, tag) = (field(case, "aad"), field(case, "tag"));
            let (plain, sealed) = (field(case, "msg"), field(case, "ct"));
            if key.len() == 24 {
                assert_eq!(Aesgcm::new(&key).err(), Some(Error::Length), "case {id}");
                count[2] += 1;
                continue;
            }
            let cipher = Aesgcm::new(&key).unwrap();
            if nonce.len() != 12 || tag.len() != 16 {
                count[3] += 1;
                continue;
            }
            let nonce: [u8; 12] = array(&nonce);
            if case["result"] == "valid" {
                count[0] += 1;
                let mut data = plain.clone();
                let made = cipher.seal(&nonce, &aad, &mut data).unwrap();
                assert_eq!((&data, &made[..]), (&sealed, &tag[..]), "case {id}");
                let mut data = sealed.clone();
                cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
                assert_eq!(data, plain, "case {id} open");
            } else {
                count[1] += 1;
                let mut data = sealed.clone();
                let result = cipher.open(&nonce, &aad, &mut data, &tag);
                assert_eq!(result, Err(Error::Auth), "case {id}");
                assert_eq!(data, sealed, "case {id} untouched");
            }
        }
    }
    assert_eq!(count.iter().sum::<usize>(), 316);
    assert_eq!(count, [79, 54, 103, 80]);
}
