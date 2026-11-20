use crate::ct::Choice;

pub type Limbs = [u64; 4];

pub const ZERO: Limbs = [0; 4];
pub const ONE: Limbs = [1, 0, 0, 0];

pub struct Modulus {
    value: Limbs,
    factor: u64,
    unit: Limbs,
    rsquared: Limbs,
}

pub const P: Modulus = Modulus {
    value: [
        0xffff_ffff_ffff_ffff,
        0x0000_0000_ffff_ffff,
        0x0000_0000_0000_0000,
        0xffff_ffff_0000_0001,
    ],
    factor: 1,
    unit: [
        0x0000_0000_0000_0001,
        0xffff_ffff_0000_0000,
        0xffff_ffff_ffff_ffff,
        0x0000_0000_ffff_fffe,
    ],
    rsquared: [
        0x0000_0000_0000_0003,
        0xffff_fffb_ffff_ffff,
        0xffff_ffff_ffff_fffe,
        0x0000_0004_ffff_fffd,
    ],
};

pub const N: Modulus = Modulus {
    value: [
        0xf3b9_cac2_fc63_2551,
        0xbce6_faad_a717_9e84,
        0xffff_ffff_ffff_ffff,
        0xffff_ffff_0000_0000,
    ],
    factor: 0xccd1_c8aa_ee00_bc4f,
    unit: [
        0x0c46_353d_039c_daaf,
        0x4319_0552_58e8_617b,
        0x0000_0000_0000_0000,
        0x0000_0000_ffff_ffff,
    ],
    rsquared: [
        0x8324_4c95_be79_eea2,
        0x4699_799c_49bd_6fa6,
        0x2845_b239_2b6b_ec59,
        0x66e1_2d94_f3d9_5620,
    ],
};

fn mac(base: u64, left: u64, right: u64, carry: u64) -> (u64, u64) {
    let full = u128::from(base) + u128::from(left) * u128::from(right) + u128::from(carry);
    (full as u64, (full >> 64) as u64)
}

fn adc(left: u64, right: u64, carry: u64) -> (u64, u64) {
    let full = u128::from(left) + u128::from(right) + u128::from(carry);
    (full as u64, (full >> 64) as u64)
}

fn sbb(left: u64, right: u64, borrow: u64) -> (u64, u64) {
    let full = u128::from(left)
        .wrapping_sub(u128::from(right))
        .wrapping_sub(u128::from(borrow));
    (full as u64, (full >> 127) as u64)
}

pub fn plus(left: &Limbs, right: &Limbs) -> (Limbs, u64) {
    let mut out = ZERO;
    let mut carry = 0;
    for index in 0..4 {
        (out[index], carry) = adc(left[index], right[index], carry);
    }
    (out, carry)
}

pub fn minus(whole: &Limbs, part: &Limbs) -> (Limbs, u64) {
    let mut out = ZERO;
    let mut borrow = 0;
    for index in 0..4 {
        (out[index], borrow) = sbb(whole[index], part[index], borrow);
    }
    (out, borrow)
}

pub fn pick(flag: Choice, left: &Limbs, right: &Limbs) -> Limbs {
    let mut out = ZERO;
    for index in 0..4 {
        out[index] = flag.select(left[index], right[index]);
    }
    out
}

pub fn vanishes(value: &Limbs) -> Choice {
    let spread = value[0] | value[1] | value[2] | value[3];
    let spread = core::hint::black_box(spread);
    let high = (spread | spread.wrapping_neg()) >> 63;
    Choice::bit((high ^ 1) as u8)
}

pub fn same(first: &Limbs, second: &Limbs) -> Choice {
    let mut spread = 0u64;
    for index in 0..4 {
        spread |= first[index] ^ second[index];
    }
    vanishes(&[spread, 0, 0, 0])
}

pub fn below(value: &Limbs, bound: &Limbs) -> bool {
    minus(value, bound).1 == 1
}

pub fn load(bytes: &[u8]) -> Limbs {
    let mut out = ZERO;
    for (index, chunk) in bytes.rchunks_exact(8).enumerate() {
        let mut word = [0u8; 8];
        word.copy_from_slice(chunk);
        out[index] = u64::from_be_bytes(word);
    }
    out
}

pub fn store(value: &Limbs) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, word) in value.iter().enumerate() {
        let start = 24 - 8 * index;
        out[start..start + 8].copy_from_slice(&word.to_be_bytes());
    }
    out
}

impl Modulus {
    pub fn modulus(&self) -> Limbs {
        self.value
    }

    pub fn unit(&self) -> Limbs {
        self.unit
    }

    fn settle(&self, value: &Limbs, top: u64) -> Limbs {
        let (reduced, borrow) = minus(value, &self.value);
        let flag = Choice::bit((top | (borrow ^ 1)) as u8);
        pick(flag, &reduced, value)
    }

    pub fn add(&self, left: &Limbs, right: &Limbs) -> Limbs {
        let (sum, carry) = plus(left, right);
        self.settle(&sum, carry)
    }

    pub fn sub(&self, left: &Limbs, right: &Limbs) -> Limbs {
        let (diff, borrow) = minus(left, right);
        let (fixed, _) = plus(&diff, &self.value);
        pick(Choice::bit(borrow as u8), &fixed, &diff)
    }

    pub fn negate(&self, value: &Limbs) -> Limbs {
        self.sub(&ZERO, value)
    }

    pub fn mul(&self, left: &Limbs, right: &Limbs) -> Limbs {
        let mut acc = [0u64; 6];
        for word in right {
            let mut carry = 0;
            for index in 0..4 {
                (acc[index], carry) = mac(acc[index], left[index], *word, carry);
            }
            let top = u128::from(acc[4]) + u128::from(carry);
            acc[4] = top as u64;
            acc[5] = (top >> 64) as u64;
            let factor = acc[0].wrapping_mul(self.factor);
            let (_, mut carry) = mac(acc[0], factor, self.value[0], 0);
            for index in 1..4 {
                (acc[index - 1], carry) = mac(acc[index], factor, self.value[index], carry);
            }
            let top = u128::from(acc[4]) + u128::from(carry);
            acc[3] = top as u64;
            acc[4] = acc[5] + (top >> 64) as u64;
        }
        let out = self.settle(&[acc[0], acc[1], acc[2], acc[3]], acc[4]);
        crate::ct::wipe(&mut acc);
        out
    }

    pub fn square(&self, value: &Limbs) -> Limbs {
        self.mul(value, value)
    }

    pub fn lift(&self, value: &Limbs) -> Limbs {
        self.mul(value, &self.rsquared)
    }

    pub fn lower(&self, value: &Limbs) -> Limbs {
        self.mul(value, &ONE)
    }

    pub fn power(&self, base: &Limbs, exponent: &Limbs) -> Limbs {
        let mut out = self.unit;
        for word in exponent.iter().rev() {
            for shift in (0..64).rev() {
                out = self.square(&out);
                if (word >> shift) & 1 == 1 {
                    out = self.mul(&out, base);
                }
            }
        }
        out
    }

    pub fn invert(&self, value: &Limbs) -> Limbs {
        let (exponent, _) = minus(&self.value, &[2, 0, 0, 0]);
        self.power(value, &exponent)
    }

    pub fn reduce(&self, value: &Limbs) -> Limbs {
        self.settle(value, 0)
    }
}
