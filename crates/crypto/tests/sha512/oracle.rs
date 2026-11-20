use sha2::Digest;

use crate::support::{Algo, Rng, SHA384, SHA512};

const EDGES: [usize; 24] = [
    0, 1, 111, 112, 113, 119, 120, 127, 128, 129, 239, 240, 241, 247, 248, 255, 256, 257, 383, 384,
    385, 511, 512, 513,
];

fn lengths() -> Vec<usize> {
    let mut found: Vec<usize> = (0..=600).collect();
    found.extend(EDGES);
    found.extend([1023, 1024, 1025, 4096, 65_537]);
    found
}

fn agree(algo: &Algo, reference: fn(&[u8]) -> Vec<u8>, seed: u64) {
    let mut rng = Rng::new(seed);
    for length in lengths() {
        let data = rng.bytes(length);
        let want = reference(&data);
        assert_eq!((algo.whole)(&data), want, "length {length}");
        let step = 1 + rng.below(200);
        assert_eq!(
            (algo.parts)(&data, step),
            want,
            "length {length} step {step}"
        );
    }
}

fn ragged(algo: &Algo, reference: fn(&[u8]) -> Vec<u8>, seed: u64) {
    let mut rng = Rng::new(seed);
    for round in 0..300 {
        let size = rng.below(2000);
        let data = rng.bytes(size);
        let want = reference(&data);
        let mut cuts: Vec<usize> = (0..rng.below(8))
            .map(|_| rng.below(data.len() + 1))
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        let mut pieces = Vec::new();
        let mut from = 0;
        for cut in cuts.into_iter().chain([data.len()]) {
            pieces.push(&data[from..cut]);
            from = cut;
        }
        assert_eq!(pieces.concat(), data);
        assert_eq!((algo.pieces)(&pieces), want, "round {round}");
    }
}

fn reference384(data: &[u8]) -> Vec<u8> {
    sha2::Sha384::digest(data).to_vec()
}

fn reference512(data: &[u8]) -> Vec<u8> {
    sha2::Sha512::digest(data).to_vec()
}

#[test]
fn lengths384() {
    agree(&SHA384, reference384, 384);
}

#[test]
fn lengths512() {
    agree(&SHA512, reference512, 512);
}

#[test]
fn random384() {
    ragged(&SHA384, reference384, 3840);
}

#[test]
fn random512() {
    ragged(&SHA512, reference512, 5120);
}
