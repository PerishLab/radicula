use std::collections::BTreeMap;

use radicula_crypto::Error;
use radicula_crypto::p256::ecdsa;

use crate::support::{ORDER, array, der, field, hex, load, padded, subtract, uncompressed};

struct Record {
    message: Vec<u8>,
    key: [u8; 65],
    first: [u8; 32],
    second: [u8; 32],
    pass: bool,
}

fn records() -> Vec<Record> {
    let text = std::fs::read_to_string("vectors/p256/SigVer.rsp").unwrap();
    let mut out = Vec::new();
    for block in text.split("\n\n").filter(|block| block.contains("Qx =")) {
        let line = |label: &str| {
            block
                .lines()
                .find_map(|line| line.strip_prefix(label))
                .map(|rest| rest.trim_start_matches([' ', '=']).trim().to_string())
                .unwrap()
        };
        out.push(Record {
            message: hex(&line("Msg")),
            key: uncompressed(&hex(&line("Qx")), &hex(&line("Qy"))),
            first: padded(&hex(&line("R"))),
            second: padded(&hex(&line("S"))),
            pass: line("Result").starts_with('P'),
        });
    }
    out
}

fn sample() -> Record {
    records().into_iter().find(|row| row.pass).unwrap()
}

fn flags(row: &serde_json::Value) -> Vec<String> {
    row["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag.as_str().unwrap().to_string())
        .collect()
}

fn count(row: &serde_json::Value, tally: &mut BTreeMap<String, usize>) {
    for flag in flags(row) {
        *tally.entry(flag).or_default() += 1;
    }
}

#[test]
fn nist() {
    let rows = records();
    let (mut passed, mut failed) = (0, 0);
    for row in &rows {
        let raw = [row.first, row.second].concat();
        let raw: [u8; 64] = array(&raw);
        let wire = der(&row.first, &row.second);
        let one = ecdsa::ieee(&row.key, &row.message, &raw);
        let two = ecdsa::verify(&row.key, &row.message, &wire);
        assert_eq!(one, two);
        if row.pass {
            assert_eq!(one, Ok(()));
            passed += 1;
        } else {
            assert!(one.is_err());
            failed += 1;
        }
    }
    assert_eq!((passed, failed), (3, 12));
}

#[test]
fn wycheproof() {
    let data = load("ecdsa_secp256r1_sha256_test.json");
    let (mut valid, mut invalid) = (0, 0);
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for group in data["testGroups"].as_array().unwrap() {
        let key = hex(group["publicKey"]["uncompressed"].as_str().unwrap());
        for row in group["tests"].as_array().unwrap() {
            let id = row["tcId"].as_u64().unwrap();
            let got = ecdsa::verify(&key, &field(row, "msg"), &field(row, "sig"));
            if row["result"] == "valid" {
                assert_eq!(got, Ok(()), "tc {id}");
                valid += 1;
            } else {
                assert!(got.is_err(), "tc {id}");
                invalid += 1;
                count(row, &mut tally);
            }
        }
    }
    assert_eq!((valid, invalid), (174, 310));
    let want = [
        ("ArithmeticError", 14),
        ("BerEncodedSignature", 7),
        ("IntegerOverflow", 5),
        ("InvalidEncoding", 92),
        ("InvalidSignature", 64),
        ("InvalidTypesInSignature", 63),
        ("MissingZero", 1),
        ("ModifiedInteger", 5),
        ("ModifiedSignature", 48),
        ("PointDuplication", 6),
        ("RangeCheck", 6),
    ];
    assert_eq!(tally.len(), want.len());
    for (flag, count) in want {
        assert_eq!(tally[flag], count, "{flag}");
    }
}

#[test]
fn ieee() {
    let data = load("ecdsa_secp256r1_sha256_p1363_test.json");
    let (mut valid, mut invalid, mut sized) = (0, 0, 0);
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for group in data["testGroups"].as_array().unwrap() {
        let key = hex(group["publicKey"]["uncompressed"].as_str().unwrap());
        for row in group["tests"].as_array().unwrap() {
            let id = row["tcId"].as_u64().unwrap();
            let signature = field(row, "sig");
            if row["result"] != "valid" {
                count(row, &mut tally);
            }
            let Ok(raw) = <[u8; 64]>::try_from(signature.as_slice()) else {
                assert_eq!(row["result"], "invalid");
                sized += 1;
                continue;
            };
            let got = ecdsa::ieee(&key, &field(row, "msg"), &raw);
            if row["result"] == "valid" {
                assert_eq!(got, Ok(()), "tc {id}");
                valid += 1;
            } else {
                assert!(got.is_err(), "tc {id}");
                invalid += 1;
            }
        }
    }
    assert_eq!((valid, invalid, sized), (173, 68, 21));
    let want = [
        ("ArithmeticError", 26),
        ("IntegerOverflow", 4),
        ("InvalidSignature", 49),
        ("ModifiedInteger", 1),
        ("PointDuplication", 6),
        ("RangeCheck", 4),
        ("SignatureSize", 12),
        ("SmallRandS", 8),
    ];
    assert_eq!(tally.len(), want.len());
    for (flag, count) in want {
        assert_eq!(tally[flag], count, "{flag}");
    }
}

#[test]
fn bounds() {
    let row = sample();
    let mut above = ORDER;
    above[31] += 1;
    for bad in [[0u8; 32], ORDER, above, [0xff; 32]] {
        for (first, second) in [(bad, row.second), (row.first, bad), (bad, bad)] {
            let raw: [u8; 64] = array(&[first, second].concat());
            assert_eq!(ecdsa::ieee(&row.key, &row.message, &raw), Err(Error::Auth));
            assert_eq!(
                ecdsa::verify(&row.key, &row.message, &der(&first, &second)),
                Err(Error::Auth)
            );
        }
    }
    let mut one = [0u8; 32];
    one[31] = 1;
    let below = subtract(&ORDER, &one);
    let raw: [u8; 64] = array(&[below, below].concat());
    assert_eq!(ecdsa::ieee(&row.key, &row.message, &raw), Err(Error::Auth));
}

#[test]
fn halves() {
    let row = sample();
    let flipped = subtract(&ORDER, &row.second);
    assert_eq!(
        ecdsa::verify(&row.key, &row.message, &der(&row.first, &row.second)),
        Ok(())
    );
    assert_eq!(
        ecdsa::verify(&row.key, &row.message, &der(&row.first, &flipped)),
        Ok(())
    );
    let raw: [u8; 64] = array(&[row.first, flipped].concat());
    assert_eq!(ecdsa::ieee(&row.key, &row.message, &raw), Ok(()));
    let negated = subtract(&ORDER, &row.first);
    assert!(ecdsa::verify(&row.key, &row.message, &der(&negated, &flipped)).is_err());
    assert!(ecdsa::verify(&row.key, &row.message, &der(&negated, &row.second)).is_err());
}

#[test]
fn encoding() {
    let row = sample();
    let good = der(&row.first, &row.second);
    assert_eq!(ecdsa::verify(&row.key, &row.message, &good), Ok(()));
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(Vec::new());
    cases.push(good[..good.len() - 1].to_vec());
    cases.push([&good[..], &[0u8]].concat());
    let mut longform = vec![0x30, 0x81, good[1]];
    longform.extend_from_slice(&good[2..]);
    cases.push(longform);
    let mut tag = good.clone();
    tag[0] = 0x31;
    cases.push(tag);
    let mut inner = good.clone();
    inner[2] = 3;
    cases.push(inner);
    let mut short = good.clone();
    short[1] -= 1;
    cases.push(short);
    let mut padding = vec![0x30, good[1] + 1, 2, good[3] + 1, 0];
    padding.extend_from_slice(&good[4..]);
    cases.push(padding);
    let mut negative = vec![0x30, 0x44, 2, 0x20];
    negative.extend_from_slice(&[0x80; 32]);
    negative.extend_from_slice(&[2, 0x20]);
    negative.extend_from_slice(&[0x01; 32]);
    cases.push(negative);
    cases.push(vec![0x30, 0x06, 2, 1, 0, 2, 1, 0]);
    cases.push(vec![0x30, 0x04, 2, 0, 2, 0]);
    cases.push(vec![0x30, 0x80]);
    cases.push(vec![0x30, 0x08, 2, 2, 0, 1, 2, 2, 0, 1]);
    cases.push(vec![0x30, 0x08, 2, 2, 0, 0x7f, 2, 2, 0, 0x7f]);
    cases.push(vec![0x30, 0x07, 2, 2, 0, 1, 2, 1, 1]);
    cases.push(vec![0x30, 0x07, 2, 1, 1, 2, 2, 0, 1]);
    cases.push(vec![0x30, 0x06, 2, 1, 0x80, 2, 1, 1]);
    for (index, case) in cases.iter().enumerate() {
        let want = if index == 9 {
            Error::Auth
        } else {
            Error::Format
        };
        assert_eq!(
            ecdsa::verify(&row.key, &row.message, case),
            Err(want),
            "case {index}"
        );
    }
}

#[test]
fn keys() {
    let row = sample();
    let good = der(&row.first, &row.second);
    let mut flipped = row.key;
    flipped[50] ^= 1;
    assert_eq!(
        ecdsa::verify(&flipped, &row.message, &good),
        Err(Error::Point)
    );
    assert_eq!(
        ecdsa::verify(&row.key[..64], &row.message, &good),
        Err(Error::Point)
    );
    assert_eq!(
        ecdsa::verify(&[0u8; 65], &row.message, &good),
        Err(Error::Point)
    );
    assert_eq!(ecdsa::verify(&[], &row.message, &good), Err(Error::Point));
    let mut message = row.message.clone();
    message.push(0);
    assert_eq!(ecdsa::verify(&row.key, &message, &good), Err(Error::Auth));
}
