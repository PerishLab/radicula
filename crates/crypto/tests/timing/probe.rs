use std::hint::black_box;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use crate::stats::score;

pub const LIMIT: f64 = 10.0;
const ROUNDS: usize = 4;
pub const POOL: usize = 64;

static BUSY: Mutex<()> = Mutex::new(());

pub fn serial() -> MutexGuard<'static, ()> {
    BUSY.lock().unwrap_or_else(|held| held.into_inner())
}

pub struct Mix(u64);

impl Mix {
    pub fn new(seed: u64) -> Self {
        Mix(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
}

pub trait Probe {
    fn prepare(&mut self, class: bool, pick: usize);
    fn run(&mut self);
}

pub struct Pools<T> {
    items: [Vec<T>; 3],
    low: usize,
    high: usize,
}

impl<T: Clone> Pools<T> {
    pub fn new(items: [Vec<T>; 3], low: usize, high: usize) -> Self {
        Pools { items, low, high }
    }

    pub fn pick(&self, class: bool, pick: usize) -> T {
        self.items[if class { self.high } else { self.low }][pick].clone()
    }
}

pub struct Fed<T, F> {
    pools: Pools<T>,
    scratch: T,
    work: F,
}

impl<T, F> Fed<T, F> {
    pub fn new(pools: Pools<T>, scratch: T, work: F) -> Self {
        Fed {
            pools,
            scratch,
            work,
        }
    }
}

impl<T: Clone, F: FnMut(&T)> Probe for Fed<T, F> {
    fn prepare(&mut self, class: bool, pick: usize) {
        self.scratch = self.pools.pick(class, pick);
    }

    fn run(&mut self) {
        (self.work)(black_box(&self.scratch));
    }
}

pub fn round(count: usize, seed: u64, probe: &mut dyn Probe) -> f64 {
    let mut mix = Mix::new(seed);
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count / 10 {
        probe.prepare(mix.next() & 1 == 1, mix.next() as usize % POOL);
        probe.run();
    }
    for _ in 0..count {
        let class = mix.next() & 1 == 1;
        probe.prepare(class, mix.next() as usize % POOL);
        let start = Instant::now();
        probe.run();
        samples.push((class, start.elapsed().as_nanos() as f64));
    }
    score(&samples)
}

pub fn clean(name: &str, count: usize, probe: &mut dyn Probe) {
    let mut seen = Vec::new();
    for attempt in 0..ROUNDS {
        let value = round(count, 0x9e37 + attempt as u64, probe);
        eprintln!("{name}: round {attempt} t = {value:.2}");
        if value <= LIMIT {
            return;
        }
        seen.push(value);
    }
    panic!("{name}: every round exceeded {LIMIT}: {seen:?}");
}

pub fn leaks(name: &str, count: usize, probe: &mut dyn Probe) {
    let mut seen = Vec::new();
    for attempt in 0..ROUNDS {
        let value = round(count, 0x7f4a + attempt as u64, probe);
        eprintln!("{name}: round {attempt} t = {value:.2}");
        if value > LIMIT {
            return;
        }
        seen.push(value);
    }
    panic!("{name}: the harness never saw the leak: {seen:?}");
}
