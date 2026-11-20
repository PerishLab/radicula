use std::collections::BTreeMap;

use radicula_crypto::Error;
use radicula_crypto::rsa::Pss;
use serde_json::Value;

use crate::support::{files, hex, load, make, named};

#[derive(Default)]
struct Tally {
    run: usize,
    valid: usize,
    invalid: usize,
    acceptable: usize,
    skipped: usize,
    flags: BTreeMap<String, usize>,
}

impl Tally {
    fn note(&mut self, test: &Value) {
        self.run += 1;
        match test["result"].as_str().unwrap() {
            "valid" => self.valid += 1,
            "invalid" => self.invalid += 1,
            _ => self.acceptable += 1,
        }
        let flags = test["flags"].as_array().unwrap();
        if flags.is_empty() {
            *self.flags.entry("(none)".into()).or_default() += 1;
        }
        for flag in flags {
            *self.flags.entry(flag.as_str().unwrap().into()).or_default() += 1;
        }
    }

    fn list(&self) -> Vec<(&str, usize)> {
        self.flags
            .iter()
            .map(|(name, count)| (name.as_str(), *count))
            .collect()
    }
}

fn expect(test: &Value, outcome: Result<(), Error>) {
    let name = format!("tcId {} {}", test["tcId"], test["comment"]);
    match test["result"].as_str().unwrap() {
        "valid" => assert_eq!(outcome, Ok(()), "{name}"),
        _ => assert_eq!(outcome, Err(Error::Auth), "{name}"),
    }
}

#[test]
fn pkcs1() {
    let names = files("pkcs1");
    assert_eq!(names.len(), 10);
    let mut tally = Tally::default();
    for name in names.iter().filter(|name| !name.contains("8192")) {
        let root = load(&format!("pkcs1/{name}"));
        let mut count = 0;
        for group in root["testGroups"].as_array().unwrap() {
            let key = group["publicKey"].clone();
            let public = make(
                key["modulus"].as_str().unwrap(),
                key["publicExponent"].as_str().unwrap(),
            )
            .unwrap();
            assert_eq!(public.bits(), group["keySize"].as_u64().unwrap() as usize);
            let hash = named(group["sha"].as_str().unwrap()).unwrap();
            for test in group["tests"].as_array().unwrap() {
                let outcome = public.pkcs1(
                    hash,
                    &hex(test["msg"].as_str().unwrap()),
                    &hex(test["sig"].as_str().unwrap()),
                );
                expect(test, outcome);
                tally.note(test);
                count += 1;
            }
        }
        assert_eq!(
            count,
            root["numberOfTests"].as_u64().unwrap() as usize,
            "{name}"
        );
    }
    assert_eq!(tally.run, 2330);
    assert_eq!(tally.valid, 68);
    assert_eq!(tally.invalid, 2253);
    assert_eq!(tally.acceptable, 9);
    assert_eq!(
        tally.list(),
        [
            ("(none)", 63),
            ("BerEncodedPadding", 126),
            ("EdgeCaseSignature", 1),
            ("InvalidAsnInPadding", 1065),
            ("InvalidPadding", 45),
            ("InvalidSignature", 72),
            ("MissingNull", 9),
            ("ModifiedPadding", 675),
            ("NoHash", 18),
            ("ShortPadding", 9),
            ("SignatureMalleability", 27),
            ("SmallPublicKey", 5),
            ("SmallSignature", 4),
            ("WrongHash", 183),
            ("WrongPrimitive", 33),
        ]
    );
}

#[test]
fn oversized() {
    let root = load("pkcs1/rsa_signature_8192_sha256_test.json");
    let mut count = 0;
    for group in root["testGroups"].as_array().unwrap() {
        let key = &group["publicKey"];
        let outcome = make(
            key["modulus"].as_str().unwrap(),
            key["publicExponent"].as_str().unwrap(),
        );
        assert_eq!(outcome.err(), Some(Error::Key));
        count += group["tests"].as_array().unwrap().len();
    }
    assert_eq!(count, 258);
}

#[test]
fn pss() {
    let names = files("pss");
    assert_eq!(names.len(), 9);
    let mut tally = Tally::default();
    for name in &names {
        let root = load(&format!("pss/{name}"));
        let mut count = 0;
        for group in root["testGroups"].as_array().unwrap() {
            let tests = group["tests"].as_array().unwrap();
            count += tests.len();
            let hash = named(group["sha"].as_str().unwrap());
            let mgf = named(group["mgfSha"].as_str().unwrap());
            let (Some(hash), Some(mgf)) = (hash, mgf) else {
                tally.skipped += tests.len();
                continue;
            };
            assert_eq!(group["mgf"], "MGF1");
            let key = &group["publicKey"];
            let public = make(
                key["modulus"].as_str().unwrap(),
                key["publicExponent"].as_str().unwrap(),
            )
            .unwrap();
            let scheme = Pss {
                hash,
                mgf,
                salt: group["sLen"].as_u64().unwrap() as usize,
            };
            for test in tests {
                let outcome = public.pss(
                    scheme,
                    &hex(test["msg"].as_str().unwrap()),
                    &hex(test["sig"].as_str().unwrap()),
                );
                expect(test, outcome);
                tally.note(test);
            }
        }
        assert_eq!(
            count,
            root["numberOfTests"].as_u64().unwrap() as usize,
            "{name}"
        );
    }
    assert_eq!(tally.run, 1119);
    assert_eq!(tally.valid, 758);
    assert_eq!(tally.invalid, 361);
    assert_eq!(tally.acceptable, 0);
    assert_eq!(tally.skipped, 96);
    assert_eq!(
        tally.list(),
        [
            ("DistinctHash", 36),
            ("ModifiedSignature", 363),
            ("Normal", 56),
            ("ParameterTest", 54),
            ("SpecialCaseHash", 634),
            ("WrongPrimitive", 12),
        ]
    );
}
