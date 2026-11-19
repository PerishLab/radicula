use crate::ct::{self, Choice};

const MASK: u64 = (1 << 51) - 1;
const BIAS: [u64; 5] = [
    0x001f_ffff_ffff_ffb4,
    0x001f_ffff_ffff_fffc,
    0x001f_ffff_ffff_fffc,
    0x001f_ffff_ffff_fffc,
    0x001f_ffff_ffff_fffc,
];
const CUTS: [(usize, u32); 5] = [(0, 0), (6, 3), (12, 6), (19, 1), (24, 12)];

#[derive(Clone, Copy)]
pub struct Fe([u64; 5]);

fn load(bytes: &[u8], start: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&bytes[start..start + 8]);
    u64::from_le_bytes(word)
}

fn ripple(mut limbs: [u64; 5]) -> [u64; 5] {
    for index in 0..4 {
        let carry = limbs[index] >> 51;
        limbs[index] &= MASK;
        limbs[index + 1] += carry;
    }
    limbs
}

fn settle(limbs: [u64; 5]) -> [u64; 5] {
    let mut limbs = ripple(limbs);
    let carry = limbs[4] >> 51;
    limbs[4] &= MASK;
    limbs[0] += carry * 19;
    limbs
}

impl Fe {
    pub const ZERO: Fe = Fe([0; 5]);
    pub const ONE: Fe = Fe([1, 0, 0, 0, 0]);
    const SMALL: Fe = Fe([121_665, 0, 0, 0, 0]);

    pub fn unpack(bytes: &[u8; 32]) -> Fe {
        let mut limbs = [0u64; 5];
        for (slot, (start, shift)) in limbs.iter_mut().zip(CUTS) {
            *slot = (load(bytes, start) >> shift) & MASK;
        }
        Fe(limbs)
    }

    pub fn pack(self) -> [u8; 32] {
        let mut limbs = settle(settle(self.0));
        let mut quotient = (limbs[0] + 19) >> 51;
        for limb in &limbs[1..] {
            quotient = (limb + quotient) >> 51;
        }
        limbs[0] += quotient * 19;
        let mut limbs = ripple(limbs);
        limbs[4] &= MASK;
        let mut out = [0u8; 32];
        let (mut acc, mut bits, mut next) = (0u128, 0u32, 0usize);
        for limb in limbs {
            acc |= u128::from(limb) << bits;
            bits += 51;
            while bits >= 8 {
                out[next] = acc as u8;
                acc >>= 8;
                bits -= 8;
                next += 1;
            }
        }
        out[31] = acc as u8;
        ct::wipe(&mut limbs);
        out
    }

    pub fn sum(self, other: Fe) -> Fe {
        let mut limbs = self.0;
        for (slot, add) in limbs.iter_mut().zip(other.0) {
            *slot += add;
        }
        Fe(limbs)
    }

    pub fn diff(self, other: Fe) -> Fe {
        let mut limbs = self.0;
        for ((slot, sub), bias) in limbs.iter_mut().zip(other.0).zip(BIAS) {
            *slot = *slot + bias - sub;
        }
        Fe(limbs)
    }

    pub fn prod(self, other: Fe) -> Fe {
        let left = self.0.map(u128::from);
        let right = other.0.map(u128::from);
        let mut wide = [0u128; 5];
        for (row, one) in left.iter().enumerate() {
            for (col, two) in right.iter().enumerate() {
                let spot = row + col;
                if spot < 5 {
                    wide[spot] += one * two;
                } else {
                    wide[spot - 5] += one * two * 19;
                }
            }
        }
        let mut limbs = [0u64; 5];
        let mut carry = 0u128;
        for (slot, part) in limbs.iter_mut().zip(wide) {
            let value = part + carry;
            *slot = (value & u128::from(MASK)) as u64;
            carry = value >> 51;
        }
        let low = u128::from(limbs[0]) + carry * 19;
        limbs[0] = low as u64 & MASK;
        limbs[1] += (low >> 51) as u64;
        Fe(limbs)
    }

    pub fn square(self) -> Fe {
        self.prod(self)
    }

    pub fn scale(self) -> Fe {
        self.prod(Fe::SMALL)
    }

    pub fn squares(self, count: u32) -> Fe {
        let mut value = self;
        for _ in 0..count {
            value = value.square();
        }
        value
    }

    pub fn invert(self) -> Fe {
        let twice = self.square();
        let nine = twice.squares(2).prod(self);
        let eleven = nine.prod(twice);
        let a5 = eleven.square().prod(nine);
        let a10 = a5.squares(5).prod(a5);
        let a20 = a10.squares(10).prod(a10);
        let a40 = a20.squares(20).prod(a20);
        let a50 = a40.squares(10).prod(a10);
        let a100 = a50.squares(50).prod(a50);
        let a200 = a100.squares(100).prod(a100);
        let a250 = a200.squares(50).prod(a50);
        a250.squares(5).prod(eleven)
    }

    pub fn swap(flag: Choice, left: &mut Fe, right: &mut Fe) {
        for (one, two) in left.0.iter_mut().zip(right.0.iter_mut()) {
            let (first, second) = (*one, *two);
            *one = flag.select(second, first);
            *two = flag.select(first, second);
        }
    }

    pub fn wipe(&mut self) {
        ct::wipe(&mut self.0);
    }
}
