use std::time::Instant;

use radicula_crypto::p256::{Point, ecdh, ecdsa};

use crate::support::{array, hex};

fn rate(rounds: usize, mut work: impl FnMut()) -> f64 {
    work();
    let start = Instant::now();
    for _ in 0..rounds {
        work();
    }
    rounds as f64 / start.elapsed().as_secs_f64()
}

#[test]
#[ignore = "prints throughput; run with --release -- --ignored --nocapture"]
fn throughput() {
    let secret = [0x5au8; 32];
    let peer = Point::base().mul(&[0x33; 32]).encode().unwrap();
    let agree = rate(200, || {
        ecdh::agree(&secret, &peer).unwrap();
    });
    println!("ecdh agree: {agree:.0} ops/s");
    let public = rate(200, || {
        ecdh::public(&secret).unwrap();
    });
    println!("ecdh public: {public:.0} ops/s");
    let text = std::fs::read_to_string("vectors/p256/SigVer.rsp").unwrap();
    let block = text
        .split("\n\n")
        .find(|part| part.contains("Result = P"))
        .unwrap();
    let take = |label: &str| {
        let line = block.lines().find(|line| line.starts_with(label)).unwrap();
        hex(line.split('=').nth(1).unwrap())
    };
    let key = [vec![4u8], take("Qx"), take("Qy")].concat();
    let message = take("Msg");
    let raw: [u8; 64] = array(&[take("R"), take("S")].concat());
    let verdict = ecdsa::ieee(&key, &message, &raw).is_ok();
    let verify = rate(200, || {
        let _ = ecdsa::ieee(&key, &message, &raw);
    });
    println!("ecdsa verify (accepted={verdict}): {verify:.0} ops/s");
}
