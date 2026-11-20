use std::time::Instant;

use radicula_crypto::{Aesgcm, Ghash};

fn rate(bytes: usize, rounds: usize, mut work: impl FnMut()) -> f64 {
    work();
    let start = Instant::now();
    for _ in 0..rounds {
        work();
    }
    (bytes * rounds) as f64 / start.elapsed().as_secs_f64() / 1e6
}

#[test]
#[ignore = "prints throughput; run with --release -- --ignored --nocapture"]
fn throughput() {
    let mut data = vec![0x5au8; 1 << 20];
    for (name, size) in [("aes128", 16), ("aes256", 32)] {
        let cipher = Aesgcm::new(&vec![7u8; size]).unwrap();
        let seal = rate(data.len(), 20, || {
            cipher.seal(&[1u8; 12], b"x", &mut data).unwrap();
        });
        println!("{name}-gcm seal 1 MiB: {seal:.1} MB/s");
        let small = rate(1350, 20_000, || {
            cipher.seal(&[1u8; 12], b"x", &mut data[..1350]).unwrap();
        });
        println!("{name}-gcm seal 1350 B: {small:.1} MB/s");
    }
    let mut hash = Ghash::new(&[3u8; 16]);
    let plain = rate(data.len(), 20, || hash.update(&data));
    println!("ghash 1 MiB: {plain:.1} MB/s");
}
