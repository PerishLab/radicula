use super::word;
use crate::{Error, ct};

const SIGMA: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

const LANES: [[usize; 4]; 8] = [
    [0, 4, 8, 12],
    [1, 5, 9, 13],
    [2, 6, 10, 14],
    [3, 7, 11, 15],
    [0, 5, 10, 15],
    [1, 6, 11, 12],
    [2, 7, 8, 13],
    [3, 4, 9, 14],
];

fn quarter(state: &mut [u32; 16], lane: [usize; 4]) {
    let [a, b, c, d] = lane;
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(12);
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(7);
}

fn core(state: &[u32; 16]) -> [u8; 64] {
    let mut work = *state;
    for _ in 0..10 {
        for lane in LANES {
            quarter(&mut work, lane);
        }
    }
    let mut block = [0u8; 64];
    for ((chunk, mixed), start) in block.chunks_exact_mut(4).zip(&work).zip(state) {
        chunk.copy_from_slice(&mixed.wrapping_add(*start).to_le_bytes());
    }
    ct::wipe(&mut work);
    block
}

pub struct Chacha20 {
    state: [u32; 16],
    stream: [u8; 64],
    used: usize,
    left: u64,
}

impl Chacha20 {
    pub const KEY: usize = 32;
    pub const NONCE: usize = 12;
    pub const BLOCK: usize = 64;

    pub fn new(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> Self {
        let mut state = [0u32; 16];
        state[..4].copy_from_slice(&SIGMA);
        for (slot, chunk) in state[4..12].iter_mut().zip(key.chunks_exact(4)) {
            *slot = word(chunk);
        }
        state[12] = counter;
        for (slot, chunk) in state[13..].iter_mut().zip(nonce.chunks_exact(4)) {
            *slot = word(chunk);
        }
        Chacha20 {
            state,
            stream: [0u8; 64],
            used: Self::BLOCK,
            left: ((1u64 << 32) - u64::from(counter)) * Self::BLOCK as u64,
        }
    }

    pub fn block(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> [u8; 64] {
        core(&Self::new(key, nonce, counter).state)
    }

    pub fn apply(&mut self, data: &mut [u8]) -> Result<(), Error> {
        if data.len() as u64 > self.left {
            return Err(Error::Length);
        }
        self.left -= data.len() as u64;
        for byte in data.iter_mut() {
            if self.used == Self::BLOCK {
                self.refill();
            }
            *byte ^= self.stream[self.used];
            self.used += 1;
        }
        Ok(())
    }

    fn refill(&mut self) {
        self.stream = core(&self.state);
        self.state[12] = self.state[12].wrapping_add(1);
        self.used = 0;
    }
}

impl Drop for Chacha20 {
    fn drop(&mut self) {
        ct::wipe(&mut self.state);
        ct::wipe(&mut self.stream);
        self.used = 0;
        self.left = 0;
    }
}
