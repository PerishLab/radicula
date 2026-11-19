use std::hint::black_box;

use radicula_crypto::Hmac;
use radicula_crypto::ct::{self, Choice};

use crate::probe::{Mix, POOL, Pools, Probe, clean, serial};

pub const SIZE: usize = 32;
pub const STEPS: usize = 50;
pub const PAIRS: [(&str, usize, usize); 2] = [("equal-first", 0, 1), ("first-last", 1, 2)];
pub const SWITCHES: [(&str, usize, usize); 2] = [("taken-random", 0, 1), ("taken-skipped", 0, 2)];

pub fn tags(base: [u8; SIZE], seed: u64) -> [Vec<[u8; SIZE]>; 3] {
    let mut mix = Mix::new(seed);
    let mut pools = [Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..POOL {
        let mut first = base;
        let mut last = base;
        first[0] ^= 1 + (mix.byte() % 255);
        last[SIZE - 1] ^= 1 + (mix.byte() % 255);
        pools[0].push(base);
        pools[1].push(first);
        pools[2].push(last);
    }
    pools
}

pub fn patterns(seed: u64) -> [Vec<Vec<bool>>; 3] {
    let mut mix = Mix::new(seed);
    let taken = (0..POOL).map(|_| vec![true; STEPS]).collect();
    let random = (0..POOL)
        .map(|_| (0..STEPS).map(|_| mix.next() & 1 == 1).collect())
        .collect();
    let skipped = (0..POOL).map(|_| vec![false; STEPS]).collect();
    [taken, random, skipped]
}

pub fn bytes(mix: &mut Mix, count: usize) -> Vec<u8> {
    (0..count).map(|_| mix.byte()).collect()
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
            black_box(ct::eq(black_box(&self.base), black_box(&self.scratch)).flag());
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
        black_box(self.proto.clone().verify(black_box(&self.scratch)));
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
            black_box(Choice::from(black_box(*flag)).select(word, !word));
        }
    }
}

struct Assign {
    flags: Pools<Vec<bool>>,
    scratch: Vec<bool>,
    source: Vec<u8>,
    target: Vec<u8>,
}

impl Probe for Assign {
    fn prepare(&mut self, class: bool, pick: usize) {
        self.scratch.copy_from_slice(&self.flags.pick(class, pick));
    }

    fn run(&mut self) {
        for flag in &self.scratch {
            Choice::from(black_box(*flag)).assign(&mut self.target, &self.source);
            black_box(&self.target);
        }
    }
}

#[test]
fn compare() {
    let _held = serial();
    let mut mix = Mix::new(11);
    let base: [u8; SIZE] = std::array::from_fn(|_| mix.byte());
    for (name, low, high) in PAIRS {
        let tags = Pools::new(tags(base, 12), low, high);
        clean(
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
    let mut mix = Mix::new(21);
    let mut proto = Hmac::new(&bytes(&mut mix, 32));
    proto.update(b"message");
    let base = proto.clone().finish();
    for (name, low, high) in PAIRS {
        let tags = Pools::new(tags(base, 22), low, high);
        let proto = proto.clone();
        clean(
            name,
            20_000,
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
    let mut mix = Mix::new(31);
    let source = bytes(&mut mix, 64);
    let target = bytes(&mut mix, 64);
    for (name, low, high) in SWITCHES {
        let flags = || Pools::new(patterns(32), low, high);
        let scratch = vec![true; STEPS];
        let mut select = Select {
            flags: flags(),
            scratch: scratch.clone(),
        };
        clean(&format!("select {name}"), 20_000, &mut select);
        let (source, target) = (source.clone(), target.clone());
        let mut assign = Assign {
            flags: flags(),
            scratch,
            source,
            target,
        };
        clean(&format!("assign {name}"), 20_000, &mut assign);
    }
}
