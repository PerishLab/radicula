use radicula_crypto::{Chacha20, Chacha20poly1305, Poly1305};
use serde_json::Value;

use crate::support::{array, field, load};

fn rows(name: &str) -> Vec<Value> {
    load("rfc8439.json")[name].as_array().unwrap().clone()
}

fn keystream(row: &Value, size: usize) -> Vec<u8> {
    let mut stream = Chacha20::new(
        &array(&field(row, "key")),
        &array(&field(row, "nonce")),
        row["counter"].as_u64().unwrap() as u32,
    );
    let mut out = vec![0u8; size];
    stream.apply(&mut out).unwrap();
    out
}

#[test]
fn block() {
    let rows = rows("block");
    assert_eq!(rows.len(), 6);
    for row in rows {
        let counter = row["counter"].as_u64().unwrap() as u32;
        let out = Chacha20::block(
            &array(&field(&row, "key")),
            &array(&field(&row, "nonce")),
            counter,
        );
        let source = row["source"].as_str().unwrap();
        assert_eq!(out[..], field(&row, "output")[..], "{source}");
        assert_eq!(keystream(&row, 64), out, "{source}");
    }
}

#[test]
fn stream() {
    let rows = rows("stream");
    assert_eq!(rows.len(), 4);
    for row in rows {
        let source = row["source"].as_str().unwrap();
        let plain = field(&row, "plain");
        let mut data = plain.clone();
        let mut stream = Chacha20::new(
            &array(&field(&row, "key")),
            &array(&field(&row, "nonce")),
            row["counter"].as_u64().unwrap() as u32,
        );
        stream.apply(&mut data).unwrap();
        assert_eq!(data, field(&row, "cipher"), "{source}");
        let mut again = field(&row, "cipher");
        let mut stream = Chacha20::new(
            &array(&field(&row, "key")),
            &array(&field(&row, "nonce")),
            row["counter"].as_u64().unwrap() as u32,
        );
        stream.apply(&mut again).unwrap();
        assert_eq!(again, plain, "{source} decrypts");
    }
}

#[test]
fn mac() {
    let rows = rows("mac");
    assert_eq!(rows.len(), 12);
    for row in rows {
        let source = row["source"].as_str().unwrap();
        let key: [u8; 32] = array(&field(&row, "key"));
        let message = field(&row, "message");
        let tag = field(&row, "tag");
        let mut poly = Poly1305::new(&key);
        poly.update(&message);
        assert_eq!(poly.finish()[..], tag[..], "{source}");
        assert!(
            {
                let mut poly = Poly1305::new(&key);
                poly.update(&message);
                poly.verify(&tag)
            },
            "{source} verify"
        );
        for cut in 0..=message.len() {
            let mut poly = Poly1305::new(&key);
            poly.update(&message[..cut]);
            poly.update(&message[cut..]);
            assert_eq!(poly.finish()[..], tag[..], "{source} cut {cut}");
        }
    }
}

#[test]
fn keygen() {
    let rows = rows("keygen");
    assert_eq!(rows.len(), 4);
    for row in rows {
        let out = Chacha20::block(
            &array(&field(&row, "key")),
            &array(&field(&row, "nonce")),
            0,
        );
        let source = row["source"].as_str().unwrap();
        assert_eq!(out[..32], field(&row, "output")[..], "{source}");
    }
}

#[test]
fn aead() {
    let rows = rows("aead");
    assert_eq!(rows.len(), 2);
    for row in rows {
        let source = row["source"].as_str().unwrap();
        let cipher = Chacha20poly1305::new(&array(&field(&row, "key")));
        let nonce: [u8; 12] = array(&field(&row, "nonce"));
        let aad = field(&row, "aad");
        let mut data = field(&row, "plain");
        let tag = cipher.seal(&nonce, &aad, &mut data).unwrap();
        assert_eq!(data, field(&row, "cipher"), "{source} cipher");
        assert_eq!(tag[..], field(&row, "tag")[..], "{source} tag");
        cipher.open(&nonce, &aad, &mut data, &tag).unwrap();
        assert_eq!(data, field(&row, "plain"), "{source} open");
    }
}
