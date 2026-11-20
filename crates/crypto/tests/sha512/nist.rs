use std::fs;

use crate::support::{Algo, SHA384, SHA512, hex};

fn cases(name: &str) -> Vec<(usize, Vec<u8>, Vec<u8>)> {
    let text = fs::read_to_string(format!("vectors/sha512/{name}")).unwrap();
    let mut found = Vec::new();
    let mut bits = 0;
    let mut message = Vec::new();
    for line in text.lines() {
        let Some((label, value)) = line.split_once(" = ") else {
            continue;
        };
        match label {
            "Len" => bits = value.parse().unwrap(),
            "Msg" => message = hex(value),
            "MD" => found.push((bits, message.clone(), hex(value))),
            _ => {}
        }
    }
    found
}

fn short(algo: &Algo, name: &str) {
    let found = cases(name);
    assert_eq!(found.len(), 129);
    for (bits, message, digest) in found {
        assert_eq!(bits % 8, 0);
        let message = &message[..bits / 8];
        assert_eq!(digest.len(), algo.size);
        assert_eq!((algo.whole)(message), digest, "{bits} bits");
        assert_eq!((algo.parts)(message, 1), digest, "{bits} bits by byte");
    }
}

fn long(algo: &Algo, name: &str) {
    let found = cases(name);
    assert_eq!(found.len(), 128);
    for (bits, message, digest) in found {
        assert_eq!(bits % 8, 0);
        assert_eq!(message.len(), bits / 8);
        assert_eq!(digest.len(), algo.size);
        assert_eq!((algo.whole)(&message), digest, "{bits} bits");
        assert_eq!((algo.parts)(&message, 97), digest, "{bits} bits chunked");
    }
}

fn monte(algo: &Algo, name: &str) {
    let text = fs::read_to_string(format!("vectors/sha512/{name}")).unwrap();
    let mut seed = Vec::new();
    let mut expected = Vec::new();
    for line in text.lines() {
        match line.split_once(" = ") {
            Some(("Seed", value)) => seed = hex(value),
            Some(("MD", value)) => expected.push(hex(value)),
            _ => {}
        }
    }
    assert_eq!(expected.len(), 100);
    assert_eq!(seed.len(), algo.size);
    for (checkpoint, digest) in expected.iter().enumerate() {
        let mut window = [seed.clone(), seed.clone(), seed.clone()];
        for _ in 0..1000 {
            let joined = window.concat();
            window = [window[1].clone(), window[2].clone(), (algo.whole)(&joined)];
        }
        seed = window[2].clone();
        assert_eq!(seed, *digest, "checkpoint {checkpoint}");
    }
}

#[test]
fn short384() {
    short(&SHA384, "SHA384ShortMsg.rsp");
}

#[test]
fn long384() {
    long(&SHA384, "SHA384LongMsg.rsp");
}

#[test]
fn monte384() {
    monte(&SHA384, "SHA384Monte.rsp");
}

#[test]
fn short512() {
    short(&SHA512, "SHA512ShortMsg.rsp");
}

#[test]
fn long512() {
    long(&SHA512, "SHA512LongMsg.rsp");
}

#[test]
fn monte512() {
    monte(&SHA512, "SHA512Monte.rsp");
}
