use std::fs;

use radicula_crypto::Error;
use serde_json::Value;

use crate::support::{KDF384, KDF512, Kdf, MAC384, MAC512, Mac, hex};

fn groups(name: &str) -> Vec<Value> {
    let text = fs::read_to_string(format!("vectors/mac/{name}")).unwrap();
    let root: Value = serde_json::from_str(&text).unwrap();
    root["testGroups"].as_array().unwrap().clone()
}

fn field(case: &Value, name: &str) -> Vec<u8> {
    hex(case[name].as_str().unwrap())
}

fn mac(name: &str, algo: &Mac) {
    let (mut seen, mut valid) = (0, 0);
    for group in groups(name) {
        let size = group["tagSize"].as_u64().unwrap() as usize / 8;
        for case in group["tests"].as_array().unwrap() {
            seen += 1;
            let id = case["tcId"].as_u64().unwrap();
            let good = case["result"] == "valid";
            valid += usize::from(good);
            let tag = field(case, "tag");
            assert_eq!(tag.len(), size, "case {id}");
            let key = field(case, "key");
            let msg = field(case, "msg");
            let full = (algo.tag)(&key, &[&msg]);
            assert_eq!(full[..size] == tag[..], good, "case {id}");
            let whole = good && size == algo.size;
            assert_eq!((algo.check)(&key, &msg, &tag), whole, "case {id}");
        }
    }
    assert_eq!((seen, valid), (174, 66), "{name}");
}

fn kdf(name: &str, algo: &Kdf) {
    let (mut seen, mut valid) = (0, 0);
    for group in groups(name) {
        for case in group["tests"].as_array().unwrap() {
            seen += 1;
            let id = case["tcId"].as_u64().unwrap();
            let size = case["size"].as_u64().unwrap() as usize;
            let salt = field(case, "salt");
            let ikm = field(case, "ikm");
            let info = field(case, "info");
            let result = (algo.derive)(&salt, &ikm, &info, size);
            if case["result"] == "valid" {
                valid += 1;
                assert_eq!(result.unwrap(), field(case, "okm"), "case {id}");
                assert!(size <= algo.limit, "case {id}");
            } else {
                assert_eq!(result, Err(Error::Length), "case {id}");
                assert!(size > algo.limit, "case {id}");
            }
        }
    }
    assert_eq!((seen, valid), (83, 80), "{name}");
}

#[test]
fn mac384() {
    mac("hmac_sha384_test.json", &MAC384);
}

#[test]
fn mac512() {
    mac("hmac_sha512_test.json", &MAC512);
}

#[test]
fn kdf384() {
    kdf("hkdf_sha384_test.json", &KDF384);
}

#[test]
fn kdf512() {
    kdf("hkdf_sha512_test.json", &KDF512);
}
