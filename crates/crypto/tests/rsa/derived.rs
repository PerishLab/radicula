use radicula_crypto::Error;
use radicula_crypto::rsa::{Hash, Pss};
use serde_json::Value;

use crate::oracle::{Pair, plain, salted};
use crate::support::{Rng, Secret, digest, hex, load, named, pick, stream};
use crate::vector::cavp::records;

fn pairing(pair: &Pair, secret: &Secret, hash: Hash, rng: &mut Rng) {
    let size = digest(hash, b"").len();
    let extra = rng.below(300);
    let message = rng.bytes(1 + extra);
    let hashed = digest(hash, &message);
    let mut crypt = stream(rng.next());
    let signature = secret
        .key
        .sign_with_rng(&mut crypt, plain(hash), &hashed)
        .unwrap();
    assert_eq!(pair.pkcs1(hash, &message, &signature), (true, true));
    let mut other = message.clone();
    other.push(0);
    assert_eq!(pair.pkcs1(hash, &other, &signature), (false, false));
    let signature = secret
        .key
        .sign_with_rng(&mut crypt, salted(hash, size), &hashed)
        .unwrap();
    assert_eq!(pair.pss(hash, size, &message, &signature), (true, true));
    assert_eq!(pair.pss(hash, size, &other, &signature), (false, false));
    assert_eq!(
        pair.pss(hash, size + 1, &message, &signature),
        (false, false)
    );
}

#[test]
fn signed() {
    let mut rng = Rng::new(2029);
    for bits in [2048, 3072] {
        let secret = pick(bits);
        let pair = Pair::new(&secret.modulus, &secret.exponent);
        for hash in [Hash::Sha256, Hash::Sha384, Hash::Sha512] {
            pairing(&pair, &secret, hash, &mut rng);
        }
    }
}

fn generated(entry: &Value) -> usize {
    let pair = Pair::new(entry["n"].as_str().unwrap(), entry["e"].as_str().unwrap());
    let mut compared = 0;
    for case in entry["cases"].as_array().unwrap() {
        let hash = named(case["hash"].as_str().unwrap()).unwrap();
        let message = hex(case["msg"].as_str().unwrap());
        let signature = hex(case["sig"].as_str().unwrap());
        let good = case["result"] == "valid";
        if case["scheme"] == "pkcs1" {
            assert_eq!(
                pair.pkcs1(hash, &message, &signature),
                (good, good),
                "{case}"
            );
            compared += 1;
        } else if case["note"] == "extra leading byte" {
            let salt = case["salt"].as_u64().unwrap() as usize;
            let scheme = Pss {
                hash,
                mgf: hash,
                salt,
            };
            assert_eq!(
                pair.ours.pss(scheme, &message, &signature),
                Err(Error::Auth)
            );
        } else if case["mgf"] == case["hash"] {
            let salt = case["salt"].as_u64().unwrap() as usize;
            assert_eq!(
                pair.pss(hash, salt, &message, &signature),
                (good, good),
                "{case}"
            );
            compared += 1;
        }
    }
    compared
}

#[test]
fn odd() {
    let root = load("generated.json");
    let compared: usize = root["keys"].as_array().unwrap().iter().map(generated).sum();
    assert_eq!(compared, 2 * 93 + 3 * 93);
}

#[test]
fn moduli() {
    let mut rng = Rng::new(2030);
    let mut compared = 0;
    for name in ["SigVer15_186-3.rsp", "SigVerPSS_186-3.rsp"] {
        for row in records(name) {
            if row.bits < 2048 || !row.pass || named(&row.hash).is_none() {
                continue;
            }
            let mut modulus = hex(&row.modulus);
            let last = modulus.len() - 1;
            let at = 1 + rng.below(last - 1);
            modulus[at] ^= 1 << rng.below(8);
            let text: String = modulus.iter().map(|byte| format!("{byte:02x}")).collect();
            let pair = Pair::new(&text, &row.exponent);
            let hash = named(&row.hash).unwrap();
            let verdict = match &row.salt {
                None => pair.pkcs1(hash, &row.message, &row.signature),
                Some(salt) => pair.pss(hash, salt.len(), &row.message, &row.signature),
            };
            assert_eq!(verdict, (false, false), "{name} {}", row.hash);
            compared += 1;
        }
    }
    assert_eq!(compared, 36);
}
