pub const LIMBS: usize = 64;

pub type Int = [u64; LIMBS];

#[derive(Clone)]
pub struct Ring {
    modulus: Int,
    square: Int,
    inverse: u64,
    size: usize,
}

pub fn load(bytes: &[u8]) -> Option<Int> {
    if bytes.len() > LIMBS * 8 {
        return None;
    }
    let mut limbs = [0u64; LIMBS];
    for (at, byte) in bytes.iter().rev().enumerate() {
        limbs[at / 8] |= u64::from(*byte) << (8 * (at % 8));
    }
    Some(limbs)
}

pub fn bits(value: &Int) -> usize {
    match value.iter().rposition(|limb| *limb != 0) {
        Some(top) => top * 64 + 64 - value[top].leading_zeros() as usize,
        None => 0,
    }
}

fn less(left: &Int, right: &Int, size: usize) -> bool {
    for at in (0..size).rev() {
        if left[at] != right[at] {
            return left[at] < right[at];
        }
    }
    false
}

fn sub(value: &mut Int, other: &Int, size: usize) {
    let mut borrow = false;
    for at in 0..size {
        let (first, one) = value[at].overflowing_sub(other[at]);
        let (second, two) = first.overflowing_sub(u64::from(borrow));
        value[at] = second;
        borrow = one || two;
    }
}

impl Ring {
    pub fn new(modulus: &Int) -> Self {
        let size = bits(modulus).div_ceil(64);
        let mut inverse = modulus[0];
        for _ in 0..6 {
            inverse = inverse.wrapping_mul(2u64.wrapping_sub(modulus[0].wrapping_mul(inverse)));
        }
        let mut ring = Ring {
            modulus: *modulus,
            square: [0; LIMBS],
            inverse: inverse.wrapping_neg(),
            size,
        };
        ring.square = ring.radix();
        ring
    }

    fn radix(&self) -> Int {
        let mut value = [0u64; LIMBS];
        value[0] = 1;
        for _ in 0..2 * 64 * self.size {
            let mut carry = 0;
            for limb in value.iter_mut().take(self.size) {
                let top = *limb >> 63;
                *limb = (*limb << 1) | carry;
                carry = top;
            }
            if carry != 0 || !less(&value, &self.modulus, self.size) {
                sub(&mut value, &self.modulus, self.size);
            }
        }
        value
    }

    pub fn below(&self, value: &Int) -> bool {
        less(value, &self.modulus, self.size)
    }

    fn mul(&self, left: &Int, right: &Int) -> Int {
        let size = self.size;
        let mut work = [0u64; LIMBS + 2];
        for digit in right.iter().take(size) {
            let mut carry = 0u128;
            for at in 0..size {
                let sum = u128::from(work[at]) + u128::from(left[at]) * u128::from(*digit) + carry;
                work[at] = sum as u64;
                carry = sum >> 64;
            }
            let top = u128::from(work[size]) + carry;
            work[size] = top as u64;
            work[size + 1] = (top >> 64) as u64;
            let factor = work[0].wrapping_mul(self.inverse);
            let first = u128::from(work[0]) + u128::from(factor) * u128::from(self.modulus[0]);
            carry = first >> 64;
            for at in 1..size {
                let sum = u128::from(work[at])
                    + u128::from(factor) * u128::from(self.modulus[at])
                    + carry;
                work[at - 1] = sum as u64;
                carry = sum >> 64;
            }
            let last = u128::from(work[size]) + carry;
            work[size - 1] = last as u64;
            work[size] = work[size + 1].wrapping_add((last >> 64) as u64);
            work[size + 1] = 0;
        }
        let mut out = [0u64; LIMBS];
        out[..size].copy_from_slice(&work[..size]);
        if work[size] != 0 || !less(&out, &self.modulus, size) {
            sub(&mut out, &self.modulus, size);
        }
        out
    }

    pub fn power(&self, base: &Int, exponent: u32) -> Int {
        let start = self.mul(base, &self.square);
        let mut acc = start;
        for at in (0..31 - exponent.leading_zeros()).rev() {
            acc = self.mul(&acc, &acc);
            if (exponent >> at) & 1 == 1 {
                acc = self.mul(&acc, &start);
            }
        }
        let mut one = [0u64; LIMBS];
        one[0] = 1;
        self.mul(&acc, &one)
    }
}
