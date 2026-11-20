use radicula_crypto::Error;
use radicula_crypto::rsa::{Key, Pss};
use serde_json::Value;

use crate::support::{hex, load, make, named};

fn outcome(key: &Key, case: &Value) -> Result<(), Error> {
    let hash = named(case["hash"].as_str().unwrap()).unwrap();
    let mgf = named(case["mgf"].as_str().unwrap()).unwrap();
    let message = hex(case["msg"].as_str().unwrap());
    let signature = hex(case["sig"].as_str().unwrap());
    if case["scheme"] == "pkcs1" {
        return key.pkcs1(hash, &message, &signature);
    }
    let salt = case["salt"].as_u64().unwrap() as usize;
    key.pss(Pss { hash, mgf, salt }, &message, &signature)
}

#[test]
fn bleichenbacher() {
    let root = load("generated.json");
    let list = root["forgeries"].as_array().unwrap();
    assert_eq!(list.len(), 8);
    for entry in list {
        let key = make(entry["n"].as_str().unwrap(), entry["e"].as_str().unwrap()).unwrap();
        let hash = named(entry["hash"].as_str().unwrap()).unwrap();
        let message = hex(entry["msg"].as_str().unwrap());
        let signature = hex(entry["sig"].as_str().unwrap());
        assert_eq!(
            signature.len() * 8,
            entry["bits"].as_u64().unwrap() as usize
        );
        assert_eq!(
            key.pkcs1(hash, &message, &signature),
            Err(Error::Auth),
            "{entry}"
        );
    }
}

#[test]
fn generated() {
    let root = load("generated.json");
    let keys = root["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 5);
    let mut valid = 0;
    let mut invalid = 0;
    for entry in keys {
        let key = make(entry["n"].as_str().unwrap(), entry["e"].as_str().unwrap()).unwrap();
        let bits = entry["bits"].as_u64().unwrap() as usize;
        assert_eq!(key.bits(), bits);
        let odd = entry["kind"] == "odd";
        assert_eq!((bits - usize::from(odd)) % 8, 0);
        for case in entry["cases"].as_array().unwrap() {
            let good = case["result"] == "valid";
            valid += usize::from(good);
            invalid += usize::from(!good);
            let want = if good { Ok(()) } else { Err(Error::Auth) };
            assert_eq!(outcome(&key, case), want, "{case}");
        }
    }
    assert_eq!(valid, 5 * 48);
    assert_eq!(invalid, 2 * 144 + 3 * 141);
}
