use std::hint::black_box;

use radicula_crypto::{Chacha20poly1305, Poly1305};

use crate::control::early;
use crate::paths::PAIRS;
use crate::probe::{Fed, Pools, leaks, serial};
use crate::tags::{AAD, KEY, NONCE, OTHERS, PLAIN, authentic, equalled, tags};

#[test]
fn poly() {
    let _held = serial();
    let base = authentic();
    let items = equalled(base, 91);
    for (name, low, high) in PAIRS {
        let pools = Pools::new(items.clone(), low, high);
        let work = |tag: &[u8; 16]| {
            let mut poly = Poly1305::new(&KEY);
            poly.update(b"message");
            black_box(early(&poly.finish(), black_box(tag)));
        };
        leaks(
            &format!("poly {name}"),
            30_000,
            &mut Fed::new(pools, base, work),
        );
    }
}

#[test]
fn open() {
    let _held = serial();
    let (_, tag) = crate::tags::sealed();
    let items = tags(tag, 92);
    let (name, low, high) = OTHERS[0];
    let pools = Pools::new(items, low, high);
    let aead = Chacha20poly1305::new(&KEY);
    let work = |given: &[u8; 16]| {
        let mut data = PLAIN;
        let want = aead.seal(&NONCE, &AAD, &mut data).unwrap();
        black_box(early(&want, black_box(given)));
    };
    leaks(
        &format!("open {name}"),
        30_000,
        &mut Fed::new(pools, tag, work),
    );
}
