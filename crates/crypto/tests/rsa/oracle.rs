use radicula_crypto::rsa::{Hash, Key, Pss};
use rsa::{BigUint, Pkcs1v15Sign, RsaPublicKey};
use serde_json::Value;

use crate::support::{Rng, digest, files, hex, load, make, named};
use crate::vector::cavp::{Record, records};

pub fn reference(modulus: &str, exponent: &str) -> RsaPublicKey {
    let number = |text: &str| BigUint::from_bytes_be(&hex(text));
    RsaPublicKey::new(number(modulus), number(exponent)).unwrap()
}

pub fn plain(hash: Hash) -> Pkcs1v15Sign {
    match hash {
        Hash::Sha256 => Pkcs1v15Sign::new::<sha2::Sha256>(),
        Hash::Sha384 => Pkcs1v15Sign::new::<sha2::Sha384>(),
        Hash::Sha512 => Pkcs1v15Sign::new::<sha2::Sha512>(),
    }
}

pub fn salted(hash: Hash, salt: usize) -> rsa::Pss {
    match hash {
        Hash::Sha256 => rsa::Pss::new_with_salt::<sha2::Sha256>(salt),
        Hash::Sha384 => rsa::Pss::new_with_salt::<sha2::Sha384>(salt),
        Hash::Sha512 => rsa::Pss::new_with_salt::<sha2::Sha512>(salt),
    }
}

pub struct Pair {
    pub ours: Key,
    theirs: RsaPublicKey,
}

impl Pair {
    pub fn new(modulus: &str, exponent: &str) -> Self {
        Pair {
            ours: make(modulus, exponent).unwrap(),
            theirs: reference(modulus, exponent),
        }
    }

    pub fn pkcs1(&self, hash: Hash, message: &[u8], signature: &[u8]) -> (bool, bool) {
        let ours = self.ours.pkcs1(hash, message, signature).is_ok();
        let theirs = self
            .theirs
            .verify(plain(hash), &digest(hash, message), signature)
            .is_ok();
        (ours, theirs)
    }

    pub fn pss(&self, hash: Hash, salt: usize, message: &[u8], signature: &[u8]) -> (bool, bool) {
        let scheme = Pss {
            hash,
            mgf: hash,
            salt,
        };
        let ours = self.ours.pss(scheme, message, signature).is_ok();
        let theirs = self
            .theirs
            .verify(salted(hash, salt), &digest(hash, message), signature)
            .is_ok();
        (ours, theirs)
    }
}

fn flip(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if !out.is_empty() {
        let at = rng.below(out.len());
        out[at] ^= 1 << rng.below(8);
    }
    out
}

#[derive(Default)]
struct Tally {
    compared: usize,
    accepted: usize,
}

fn couple(group: &Value) -> Pair {
    let key = &group["publicKey"];
    Pair::new(
        key["modulus"].as_str().unwrap(),
        key["publicExponent"].as_str().unwrap(),
    )
}

fn plainone(pair: &Pair, hash: Hash, test: &Value, rng: &mut Rng) -> Tally {
    let message = hex(test["msg"].as_str().unwrap());
    let signature = hex(test["sig"].as_str().unwrap());
    let (ours, theirs) = pair.pkcs1(hash, &message, &signature);
    assert_eq!(ours, theirs, "tcId {}", test["tcId"]);
    let mut tally = Tally {
        compared: 1,
        accepted: usize::from(ours),
    };
    if test["result"] == "valid" {
        let changed = flip(rng, &signature);
        assert_eq!(pair.pkcs1(hash, &message, &changed), (false, false));
        let mut longer = message.clone();
        longer.push(rng.next() as u8);
        assert_eq!(pair.pkcs1(hash, &longer, &signature), (false, false));
        tally.compared += 2;
    }
    tally
}

fn plaingroup(group: &Value, rng: &mut Rng) -> Tally {
    let pair = couple(group);
    let hash = named(group["sha"].as_str().unwrap()).unwrap();
    let mut tally = Tally::default();
    for test in group["tests"].as_array().unwrap() {
        let each = plainone(&pair, hash, test, rng);
        tally.compared += each.compared;
        tally.accepted += each.accepted;
    }
    tally
}

#[test]
fn wycheproof() {
    let mut rng = Rng::new(2026);
    let mut total = Tally::default();
    for name in files("pkcs1").iter().filter(|name| !name.contains("8192")) {
        let root = load(&format!("pkcs1/{name}"));
        for group in root["testGroups"].as_array().unwrap() {
            let each = plaingroup(group, &mut rng);
            total.compared += each.compared;
            total.accepted += each.accepted;
        }
    }
    assert_eq!(total.compared, 2330 + 2 * 68);
    assert_eq!(total.accepted, 68);
}

fn saltone(pair: &Pair, scheme: Pss, test: &Value, rng: &mut Rng) -> Tally {
    let message = hex(test["msg"].as_str().unwrap());
    let signature = hex(test["sig"].as_str().unwrap());
    let (ours, theirs) = pair.pss(scheme.hash, scheme.salt, &message, &signature);
    assert_eq!(ours, theirs, "tcId {}", test["tcId"]);
    let mut tally = Tally {
        compared: 1,
        accepted: usize::from(ours),
    };
    if test["result"] == "valid" && rng.below(4) == 0 {
        let changed = flip(rng, &signature);
        assert_eq!(
            pair.pss(scheme.hash, scheme.salt, &message, &changed),
            (false, false)
        );
        assert_eq!(
            pair.pss(scheme.hash, scheme.salt + 1, &message, &signature),
            (false, false)
        );
        tally.compared += 2;
    }
    tally
}

fn saltgroup(group: &Value, rng: &mut Rng) -> Tally {
    let hash = named(group["sha"].as_str().unwrap());
    let mgf = named(group["mgfSha"].as_str().unwrap());
    let (Some(hash), Some(mgf)) = (hash, mgf) else {
        return Tally::default();
    };
    if hash != mgf {
        return Tally::default();
    }
    let pair = couple(group);
    let scheme = Pss {
        hash,
        mgf,
        salt: group["sLen"].as_u64().unwrap() as usize,
    };
    let mut tally = Tally::default();
    for test in group["tests"].as_array().unwrap() {
        let each = saltone(&pair, scheme, test, rng);
        tally.compared += each.compared;
        tally.accepted += each.accepted;
    }
    tally
}

#[test]
fn pss() {
    let mut rng = Rng::new(2027);
    let mut total = Tally::default();
    for name in files("pss") {
        let root = load(&format!("pss/{name}"));
        for group in root["testGroups"].as_array().unwrap() {
            let each = saltgroup(group, &mut rng);
            total.compared += each.compared;
            total.accepted += each.accepted;
        }
    }
    assert!(total.compared >= 1119 - 96 - 54);
    assert_eq!(total.accepted, 758 - 36 - 14 + 14);
}

fn oneline(row: &Record, rng: &mut Rng) {
    let hash = named(&row.hash).unwrap();
    let pair = Pair::new(&row.modulus, &row.exponent);
    let changed = flip(rng, &row.signature);
    let verdicts = match &row.salt {
        None => [
            pair.pkcs1(hash, &row.message, &row.signature),
            pair.pkcs1(hash, &row.message, &changed),
        ],
        Some(salt) => [
            pair.pss(hash, salt.len(), &row.message, &row.signature),
            pair.pss(hash, salt.len(), &row.message, &changed),
        ],
    };
    assert_eq!(verdicts[0].0, row.pass, "{} {}", row.bits, row.hash);
    assert_eq!(verdicts[0].0, verdicts[0].1);
    assert_eq!(verdicts[1], (false, false));
}

#[test]
fn cavp() {
    let mut rng = Rng::new(2028);
    let mut compared = 0;
    for name in ["SigVer15_186-3.rsp", "SigVerPSS_186-3.rsp"] {
        for row in records(name) {
            if row.bits >= 2048 && named(&row.hash).is_some() {
                oneline(&row, &mut rng);
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 216);
}
