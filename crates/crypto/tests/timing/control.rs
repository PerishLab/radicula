use std::hint::black_box;

use radicula_crypto::Hmac;

use crate::paths::{PAIRS, SIZE, STEPS, SWITCHES, bytes, patterns, tags};
use crate::probe::{Mix, Pools, Probe, leaks, serial};

fn early(left: &[u8], right: &[u8]) -> bool {
    for (one, two) in left.iter().zip(right) {
        if black_box(one) != black_box(two) {
            return false;
        }
    }
    true
}

fn branchy(flag: bool, left: u64, right: u64) -> u64 {
    if flag { left } else { right }
}

struct Compare {
    base: [u8; SIZE],
    tags: Pools<[u8; SIZE]>,
    scratch: [u8; SIZE],
}

impl Probe for Compare {
    fn prepare(&mut self, class: bool, pick: usize) {
        self.scratch = self.tags.pick(class, pick);
    }

    fn run(&mut self) {
        for _ in 0..STEPS {
            black_box(early(black_box(&self.base), black_box(&self.scratch)));
        }
    }
}

struct Verify {
    proto: Hmac,
    tags: Pools<[u8; SIZE]>,
    scratch: [u8; SIZE],
}

impl Probe for Verify {
    fn prepare(&mut self, class: bool, pick: usize) {
        self.scratch = self.tags.pick(class, pick);
    }

    fn run(&mut self) {
        let computed = self.proto.clone().finish();
        black_box(early(&computed, black_box(&self.scratch)));
    }
}

struct Select {
    flags: Pools<Vec<bool>>,
    scratch: Vec<bool>,
}

impl Probe for Select {
    fn prepare(&mut self, class: bool, pick: usize) {
        self.scratch.copy_from_slice(&self.flags.pick(class, pick));
    }

    fn run(&mut self) {
        for (step, flag) in self.scratch.iter().enumerate() {
            let word = step as u64 * 0x9e37_79b9;
            black_box(branchy(black_box(*flag), word, !word));
        }
    }
}

#[test]
fn compare() {
    let _held = serial();
    let mut mix = Mix::new(41);
    let base: [u8; SIZE] = std::array::from_fn(|_| mix.byte());
    for (name, low, high) in PAIRS {
        let tags = Pools::new(tags(base, 42), low, high);
        leaks(
            name,
            20_000,
            &mut Compare {
                base,
                tags,
                scratch: base,
            },
        );
    }
}

#[test]
fn verify() {
    let _held = serial();
    let mut mix = Mix::new(51);
    let mut proto = Hmac::new(&bytes(&mut mix, 32));
    proto.update(b"message");
    let base = proto.clone().finish();
    for (name, low, high) in PAIRS {
        let tags = Pools::new(tags(base, 52), low, high);
        let proto = proto.clone();
        leaks(
            name,
            30_000,
            &mut Verify {
                proto,
                tags,
                scratch: base,
            },
        );
    }
}

#[test]
fn choose() {
    let _held = serial();
    let (name, low, high) = SWITCHES[0];
    let flags = Pools::new(patterns(62), low, high);
    let scratch = vec![true; STEPS];
    leaks(
        &format!("select {name}"),
        20_000,
        &mut Select { flags, scratch },
    );
}
