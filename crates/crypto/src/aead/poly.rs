use super::word;
use crate::ct;

const MASK: u32 = 0x03ff_ffff;
const FULL: u32 = 1 << 24;
const CLAMP: [u32; 5] = [
    0x03ff_ffff,
    0x03ff_ff03,
    0x03ff_c0ff,
    0x03f0_3fff,
    0x000f_ffff,
];

pub struct Poly1305 {
    factor: [u32; 5],
    scaled: [u32; 5],
    pad: [u32; 4],
    total: [u32; 5],
    buffer: [u8; 16],
    fill: u32,
}

fn limbs(block: &[u8], hibit: u32) -> [u32; 5] {
    let mut out = [0u32; 5];
    for (index, slot) in out[..4].iter_mut().enumerate() {
        let start = index * 26 / 8;
        *slot = (word(&block[start..start + 4]) >> (index * 26 % 8)) & MASK;
    }
    out[4] = (word(&block[12..16]) >> 8) | hibit;
    out
}

impl Poly1305 {
    pub const KEY: usize = 32;
    pub const TAG: usize = 16;

    pub fn new(key: &[u8; 32]) -> Self {
        let mut factor = limbs(&key[..16], 0);
        for (slot, mask) in factor.iter_mut().zip(CLAMP) {
            *slot &= mask;
        }
        let mut scaled = factor;
        for slot in scaled.iter_mut() {
            *slot *= 5;
        }
        let mut pad = [0u32; 4];
        for (slot, chunk) in pad.iter_mut().zip(key[16..].chunks_exact(4)) {
            *slot = word(chunk);
        }
        Poly1305 {
            factor,
            scaled,
            pad,
            total: [0u32; 5],
            buffer: [0u8; 16],
            fill: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        if self.fill > 0 {
            let held = self.fill as usize;
            let take = (16 - held).min(data.len());
            self.buffer[held..held + take].copy_from_slice(&data[..take]);
            self.fill += take as u32;
            data = &data[take..];
            if self.fill < 16 {
                return;
            }
            let block = self.buffer;
            self.absorb(&block, FULL);
            self.fill = 0;
        }
        let mut chunks = data.chunks_exact(16);
        for chunk in &mut chunks {
            self.absorb(chunk, FULL);
        }
        let rest = chunks.remainder();
        self.buffer[..rest.len()].copy_from_slice(rest);
        self.fill = rest.len() as u32;
    }

    pub fn finish(mut self) -> [u8; 16] {
        if self.fill > 0 {
            let mut last = [0u8; 16];
            let held = self.fill as usize;
            last[..held].copy_from_slice(&self.buffer[..held]);
            last[held] = 1;
            self.absorb(&last, 0);
            ct::wipe(&mut last);
        }
        let mut sum = self.total;
        let mut carry = 0;
        for slot in sum.iter_mut().skip(1) {
            *slot += carry;
            carry = *slot >> 26;
            *slot &= MASK;
        }
        sum[0] += carry * 5;
        carry = sum[0] >> 26;
        sum[0] &= MASK;
        sum[1] += carry;
        let mut shifted = sum;
        carry = 5;
        for slot in shifted.iter_mut() {
            *slot += carry;
            carry = *slot >> 26;
            *slot &= MASK;
        }
        let keep = carry.wrapping_sub(1);
        let mut value = 0u128;
        for (index, (low, high)) in sum.iter().zip(shifted).enumerate() {
            value |= u128::from((low & keep) | (high & !keep)) << (index * 26);
        }
        let mut pad = 0u128;
        for (index, part) in self.pad.iter().enumerate() {
            pad |= u128::from(*part) << (index * 32);
        }
        ct::wipe(&mut sum);
        ct::wipe(&mut shifted);
        value.wrapping_add(pad).to_le_bytes()
    }

    pub fn verify(self, tag: &[u8]) -> bool {
        let mut computed = self.finish();
        let same = ct::eq(&computed, tag).flag();
        ct::wipe(&mut computed);
        same
    }

    fn absorb(&mut self, block: &[u8], hibit: u32) {
        let mut sum = limbs(block, hibit);
        for (slot, held) in sum.iter_mut().zip(self.total) {
            *slot += held;
        }
        let mut wide = [0u64; 5];
        for (row, slot) in wide.iter_mut().enumerate() {
            for (col, value) in sum.iter().enumerate() {
                let coef = if col <= row {
                    self.factor[row - col]
                } else {
                    self.scaled[5 + row - col]
                };
                *slot += u64::from(*value) * u64::from(coef);
            }
        }
        let mut carry = 0u64;
        for (slot, part) in self.total.iter_mut().zip(wide) {
            let value = part + carry;
            *slot = value as u32 & MASK;
            carry = value >> 26;
        }
        let value = u64::from(self.total[0]) + carry * 5;
        self.total[0] = value as u32 & MASK;
        self.total[1] += (value >> 26) as u32;
        ct::wipe(&mut sum);
        ct::wipe(&mut wide);
    }
}

impl Drop for Poly1305 {
    fn drop(&mut self) {
        ct::wipe(&mut self.factor);
        ct::wipe(&mut self.scaled);
        ct::wipe(&mut self.pad);
        ct::wipe(&mut self.total);
        ct::wipe(&mut self.buffer);
        self.fill = 0;
    }
}
