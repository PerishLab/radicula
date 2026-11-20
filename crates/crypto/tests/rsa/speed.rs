use std::time::Instant;

use radicula_crypto::rsa::{Hash, Pss};

use crate::support::{hex, load, make};

struct Sample {
    modulus: String,
    exponent: String,
    message: Vec<u8>,
    signature: Vec<u8>,
}

fn sample(path: &str) -> Sample {
    let root = load(path);
    let group = &root["testGroups"][0];
    let test = group["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|test| test["result"] == "valid" && !test["msg"].as_str().unwrap().is_empty())
        .unwrap();
    let key = &group["publicKey"];
    Sample {
        modulus: key["modulus"].as_str().unwrap().into(),
        exponent: key["publicExponent"].as_str().unwrap().into(),
        message: hex(test["msg"].as_str().unwrap()),
        signature: hex(test["sig"].as_str().unwrap()),
    }
}

fn rate(rounds: usize, mut work: impl FnMut()) -> f64 {
    work();
    let start = Instant::now();
    for _ in 0..rounds {
        work();
    }
    rounds as f64 / start.elapsed().as_secs_f64()
}

#[test]
#[ignore = "prints throughput; run with --release -- --ignored --nocapture"]
fn throughput() {
    for bits in [2048, 3072, 4096] {
        let item = sample(&format!("pkcs1/rsa_signature_{bits}_sha256_test.json"));
        let key = make(&item.modulus, &item.exponent).unwrap();
        let pkcs1 = rate(2000, || {
            assert!(
                key.pkcs1(Hash::Sha256, &item.message, &item.signature)
                    .is_ok()
            );
        });
        println!("pkcs1 sha256 {bits} bit, e = 65537: {pkcs1:.0} verifies/s");
        let setup = rate(200, || {
            std::hint::black_box(make(&item.modulus, &item.exponent).unwrap());
        });
        println!("key setup {bits} bit: {:.0} us", 1e6 / setup);
        let item = sample(&format!("pss/rsa_pss_{bits}_sha256_mgf1_32_test.json"));
        let key = make(&item.modulus, &item.exponent).unwrap();
        let scheme = Pss {
            hash: Hash::Sha256,
            mgf: Hash::Sha256,
            salt: 32,
        };
        let pss = rate(2000, || {
            assert!(key.pss(scheme, &item.message, &item.signature).is_ok());
        });
        println!("pss sha256 {bits} bit, e = 65537: {pss:.0} verifies/s");
    }
}
