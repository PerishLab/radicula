use std::fs;

use radicula_crypto::{Error, Hkdf, Hmac};
use serde_json::Value;

use crate::support::hex;

fn groups(name: &str) -> Vec<Value> {
    let text = fs::read_to_string(format!("vectors/mac/{name}")).unwrap();
    let root: Value = serde_json::from_str(&text).unwrap();
    root["testGroups"].as_array().unwrap().clone()
}

fn field(case: &Value, name: &str) -> Vec<u8> {
    hex(case[name].as_str().unwrap())
}

#[test]
fn mac() {
    let mut seen = 0;
    for group in groups("hmac_sha256_test.json") {
        let size = group["tagSize"].as_u64().unwrap() as usize / 8;
        for case in group["tests"].as_array().unwrap() {
            seen += 1;
            let id = case["tcId"].as_u64().unwrap();
            let valid = case["result"] == "valid";
            let tag = field(case, "tag");
            assert_eq!(tag.len(), size, "case {id}");
            let mut mac = Hmac::new(&field(case, "key"));
            mac.update(&field(case, "msg"));
            let full = mac.clone().finish();
            assert_eq!(full[..size] == tag[..], valid, "case {id}");
            assert_eq!(mac.verify(&tag), valid && size == Hmac::SIZE, "case {id}");
        }
    }
    assert_eq!(seen, 174);
}

#[test]
fn kdf() {
    let mut seen = 0;
    for group in groups("hkdf_sha256_test.json") {
        for case in group["tests"].as_array().unwrap() {
            seen += 1;
            let id = case["tcId"].as_u64().unwrap();
            let size = case["size"].as_u64().unwrap() as usize;
            let kdf = Hkdf::extract(&field(case, "salt"), &field(case, "ikm"));
            let mut out = vec![0u8; size];
            let result = kdf.expand(&field(case, "info"), &mut out);
            if case["result"] == "valid" {
                result.unwrap();
                assert_eq!(out, field(case, "okm"), "case {id}");
            } else {
                assert_eq!(result, Err(Error::Length), "case {id}");
                assert!(out.iter().all(|byte| *byte == 0), "case {id}");
            }
        }
    }
    assert_eq!(seen, 86);
}
