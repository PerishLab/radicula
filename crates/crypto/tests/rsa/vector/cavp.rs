use std::collections::HashMap;
use std::fs;

use radicula_crypto::Error;
use radicula_crypto::rsa::Pss;

use crate::support::{hex, make, named};

pub struct Record {
    pub bits: usize,
    pub modulus: String,
    pub primes: [String; 2],
    pub exponent: String,
    pub secret: String,
    pub hash: String,
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
    pub salt: Option<Vec<u8>>,
    pub pass: bool,
}

fn lengths(text: &str) -> HashMap<(usize, String), usize> {
    let line = text
        .lines()
        .find(|line| line.starts_with("# Combinations selected:"))
        .unwrap();
    let mut found = HashMap::new();
    for part in line
        .trim_start_matches("# Combinations selected:")
        .split(";;")
    {
        let part = part.trim();
        let Some(rest) = part.strip_prefix("Mod Size ") else {
            continue;
        };
        let (size, hashes) = rest.split_once(" with ").unwrap();
        for hash in hashes.split("; ") {
            let (name, tail) = hash.split_once("(Salt len: ").unwrap();
            let length = tail
                .trim_end_matches(';')
                .trim_end_matches(')')
                .parse()
                .unwrap();
            found.insert((size.parse().unwrap(), name.replace('-', "")), length);
        }
    }
    found
}

struct Parser {
    bits: usize,
    modulus: String,
    primes: [String; 2],
    row: Option<Record>,
    found: Vec<Record>,
    salts: HashMap<(usize, String), usize>,
}

impl Parser {
    fn start(&mut self, hash: &str) {
        self.row = Some(Record {
            bits: self.bits,
            modulus: self.modulus.clone(),
            primes: self.primes.clone(),
            exponent: String::new(),
            secret: String::new(),
            hash: hash.into(),
            message: Vec::new(),
            signature: Vec::new(),
            salt: None,
            pass: false,
        });
    }

    fn finish(&mut self, verdict: &str) {
        let mut done = self.row.take().unwrap();
        done.pass = verdict.starts_with('P');
        if let Some(salt) = done.salt.as_mut() {
            salt.truncate(self.salts[&(done.bits, done.hash.clone())]);
        }
        self.found.push(done);
    }

    fn feed(&mut self, line: &str) {
        if let Some(rest) = line.strip_prefix("[mod = ") {
            self.bits = rest.trim_end_matches(']').parse().unwrap();
        }
        let Some((label, value)) = line.split_once(" = ") else {
            return;
        };
        match label {
            "n" => self.modulus = value.into(),
            "p" => self.primes[0] = value.into(),
            "q" => self.primes[1] = value.into(),
            "SHAAlg" => self.start(value),
            "Result" => self.finish(value),
            _ => self.field(label, value),
        }
    }

    fn field(&mut self, label: &str, value: &str) {
        let Some(row) = self.row.as_mut() else {
            return;
        };
        match label {
            "e" => row.exponent = value.into(),
            "d" => row.secret = value.into(),
            "Msg" => row.message = hex(value),
            "S" => row.signature = hex(value),
            "SaltVal" => row.salt = Some(hex(value)),
            _ => {}
        }
    }
}

pub fn records(name: &str) -> Vec<Record> {
    let text = fs::read_to_string(format!("vectors/rsa/cavp/{name}")).unwrap();
    let salts = if name.contains("PSS") {
        lengths(&text)
    } else {
        HashMap::new()
    };
    let mut parser = Parser {
        bits: 0,
        modulus: String::new(),
        primes: [String::new(), String::new()],
        row: None,
        found: Vec::new(),
        salts,
    };
    for line in text.lines() {
        parser.feed(line);
    }
    parser.found
}

fn supported(name: &str) -> Vec<Record> {
    records(name)
        .into_iter()
        .filter(|row| row.bits >= 2048 && named(&row.hash).is_some())
        .collect()
}

#[test]
fn pkcs1() {
    let all = records("SigVer15_186-3.rsp");
    assert_eq!(all.len(), 270);
    let used = supported("SigVer15_186-3.rsp");
    assert_eq!(used.len(), 108);
    assert_eq!(used.iter().filter(|row| row.pass).count(), 18);
    for row in &used {
        let key = make(&row.modulus, &row.exponent).unwrap();
        let outcome = key.pkcs1(named(&row.hash).unwrap(), &row.message, &row.signature);
        let want = if row.pass { Ok(()) } else { Err(Error::Auth) };
        assert_eq!(
            outcome,
            want,
            "{} {} {}",
            row.bits,
            row.hash,
            row.message.len()
        );
    }
}

#[test]
fn pss() {
    let all = records("SigVerPSS_186-3.rsp");
    assert_eq!(all.len(), 270);
    let used = supported("SigVerPSS_186-3.rsp");
    assert_eq!(used.len(), 108);
    assert_eq!(used.iter().filter(|row| row.pass).count(), 18);
    for row in &used {
        let key = make(&row.modulus, &row.exponent).unwrap();
        let hash = named(&row.hash).unwrap();
        let scheme = Pss {
            hash,
            mgf: hash,
            salt: row.salt.as_ref().unwrap().len(),
        };
        let outcome = key.pss(scheme, &row.message, &row.signature);
        let want = if row.pass { Ok(()) } else { Err(Error::Auth) };
        assert_eq!(
            outcome,
            want,
            "{} {} {}",
            row.bits,
            row.hash,
            row.message.len()
        );
    }
}

#[test]
fn small() {
    for name in ["SigVer15_186-3.rsp", "SigVerPSS_186-3.rsp"] {
        let small: Vec<Record> = records(name)
            .into_iter()
            .filter(|row| row.bits < 2048)
            .collect();
        assert_eq!(small.len(), 90);
        for row in &small {
            assert_eq!(row.bits, 1024);
            assert_eq!(make(&row.modulus, &row.exponent).err(), Some(Error::Key));
        }
    }
}

#[test]
fn others() {
    for name in ["SigVer15_186-3.rsp", "SigVerPSS_186-3.rsp"] {
        let rest: Vec<Record> = records(name)
            .into_iter()
            .filter(|row| row.bits >= 2048 && named(&row.hash).is_none())
            .collect();
        assert_eq!(rest.len(), 72);
        assert!(
            rest.iter()
                .all(|row| row.hash == "SHA1" || row.hash == "SHA224")
        );
    }
}
