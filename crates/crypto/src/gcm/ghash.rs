use crate::ct;

const SPREAD: [u64; 4] = [
    0x1111_1111_1111_1111,
    0x2222_2222_2222_2222,
    0x4444_4444_4444_4444,
    0x8888_8888_8888_8888,
];

fn bmul(left: u64, right: u64) -> u64 {
    let x0 = left & SPREAD[0];
    let x1 = left & SPREAD[1];
    let x2 = left & SPREAD[2];
    let x3 = left & SPREAD[3];
    let y0 = right & SPREAD[0];
    let y1 = right & SPREAD[1];
    let y2 = right & SPREAD[2];
    let y3 = right & SPREAD[3];
    let z0 =
        (x0.wrapping_mul(y0) ^ x1.wrapping_mul(y3) ^ x2.wrapping_mul(y2) ^ x3.wrapping_mul(y1))
            & SPREAD[0];
    let z1 =
        (x0.wrapping_mul(y1) ^ x1.wrapping_mul(y0) ^ x2.wrapping_mul(y3) ^ x3.wrapping_mul(y2))
            & SPREAD[1];
    let z2 =
        (x0.wrapping_mul(y2) ^ x1.wrapping_mul(y1) ^ x2.wrapping_mul(y0) ^ x3.wrapping_mul(y3))
            & SPREAD[2];
    let z3 =
        (x0.wrapping_mul(y3) ^ x1.wrapping_mul(y2) ^ x2.wrapping_mul(y1) ^ x3.wrapping_mul(y0))
            & SPREAD[3];
    z0 | z1 | z2 | z3
}

fn clmul(left: u64, right: u64) -> u128 {
    let low = bmul(left, right);
    let high = bmul(left.reverse_bits(), right.reverse_bits()).reverse_bits() >> 1;
    (u128::from(high) << 64) | u128::from(low)
}

fn reduce(high: u128, low: u128) -> u128 {
    let fold = high ^ (high << 1) ^ (high << 2) ^ (high << 7);
    let over = (high >> 127) ^ (high >> 126) ^ (high >> 121);
    low ^ fold ^ over ^ (over << 1) ^ (over << 2) ^ (over << 7)
}

fn multiply(left: u128, right: u128) -> u128 {
    let (a0, a1) = (left as u64, (left >> 64) as u64);
    let (b0, b1) = (right as u64, (right >> 64) as u64);
    let near = clmul(a0, b0);
    let far = clmul(a1, b1);
    let mid = clmul(a0 ^ a1, b0 ^ b1) ^ near ^ far;
    reduce(far ^ (mid >> 64), near ^ (mid << 64))
}

fn load(block: &[u8; 16]) -> u128 {
    u128::from_be_bytes(*block).reverse_bits()
}

fn split(value: u128) -> [u64; 2] {
    [value as u64, (value >> 64) as u64]
}

fn join(words: [u64; 2]) -> u128 {
    u128::from(words[0]) | (u128::from(words[1]) << 64)
}

pub struct Ghash {
    key: [u64; 2],
    state: [u64; 2],
    buffer: [u8; 16],
    fill: u64,
}

impl Ghash {
    pub fn new(key: &[u8; 16]) -> Self {
        Ghash {
            key: split(load(key)),
            state: [0; 2],
            buffer: [0u8; 16],
            fill: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut rest = data;
        while !rest.is_empty() {
            let take = rest.len().min(16 - self.fill as usize);
            let start = self.fill as usize;
            self.buffer[start..start + take].copy_from_slice(&rest[..take]);
            self.fill += take as u64;
            rest = &rest[take..];
            if self.fill == 16 {
                self.absorb();
            }
        }
    }

    pub fn pad(&mut self) {
        if self.fill > 0 {
            let start = self.fill as usize;
            self.buffer[start..].fill(0);
            self.absorb();
        }
    }

    pub fn finish(mut self) -> [u8; 16] {
        self.pad();
        join(self.state).reverse_bits().to_be_bytes()
    }

    fn absorb(&mut self) {
        let value = multiply(join(self.state) ^ load(&self.buffer), join(self.key));
        self.state = split(value);
        self.fill = 0;
    }
}

impl Drop for Ghash {
    fn drop(&mut self) {
        ct::wipe(&mut self.key);
        ct::wipe(&mut self.state);
        ct::wipe(&mut self.buffer);
        self.fill = 0;
    }
}
