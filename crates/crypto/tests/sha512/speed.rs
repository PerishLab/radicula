use std::time::Instant;

use radicula_crypto::{Sha384, Sha512};

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
    let data = vec![0x5au8; 1 << 20];
    let big = rate(data.len(), 40, || {
        std::hint::black_box(Sha512::digest(&data));
    });
    println!("sha512 1 MiB: {big:.1} MB/s");
    let small = rate(data.len(), 40, || {
        std::hint::black_box(Sha384::digest(&data));
    });
    println!("sha384 1 MiB: {small:.1} MB/s");
    let short = rate(64, 400_000, || {
        std::hint::black_box(Sha512::digest(&data[..64]));
    });
    println!("sha512 64 B: {short:.1} MB/s");
}
