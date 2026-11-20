use std::fs;

use radicula_crypto::Sha256;

use crate::support::hex;

fn cases(name: &str) -> Vec<(usize, Vec<u8>, Vec<u8>)> {
    let text = fs::read_to_string(format!("vectors/sha256/{name}")).unwrap();
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

#[test]
fn short() {
    let found = cases("SHA256ShortMsg.rsp");
    assert_eq!(found.len(), 65);
    for (bits, message, digest) in found {
        assert_eq!(bits % 8, 0);
        let message = &message[..bits / 8];
        assert_eq!(Sha256::digest(message).to_vec(), digest, "{bits} bits");
        let mut hash = Sha256::new();
        for byte in message {
            hash.update(&[*byte]);
        }
        assert_eq!(hash.finish().to_vec(), digest, "{bits} bits by byte");
    }
}

#[test]
fn long() {
    let found = cases("SHA256LongMsg.rsp");
    assert_eq!(found.len(), 64);
    for (bits, message, digest) in found {
        assert_eq!(bits % 8, 0);
        assert_eq!(message.len(), bits / 8);
        assert_eq!(Sha256::digest(&message).to_vec(), digest, "{bits} bits");
        let mut hash = Sha256::new();
        for piece in message.chunks(61) {
            hash.update(piece);
        }
        assert_eq!(hash.finish().to_vec(), digest, "{bits} bits chunked");
    }
}

#[test]
fn monte() {
    let text = fs::read_to_string("vectors/sha256/SHA256Monte.rsp").unwrap();
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
    let mut seed: [u8; 32] = seed.try_into().unwrap();
    for (checkpoint, digest) in expected.iter().enumerate() {
        let mut window = [seed; 3];
        for _ in 0..1000 {
            let mut hash = Sha256::new();
            for part in &window {
                hash.update(part);
            }
            window = [window[1], window[2], hash.finish()];
        }
        seed = window[2];
        assert_eq!(seed.to_vec(), *digest, "checkpoint {checkpoint}");
    }
}
