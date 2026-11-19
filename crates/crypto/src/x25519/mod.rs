mod field;

use field::Fe;

use crate::Error;
use crate::ct::{self, Choice};

pub const SIZE: usize = 32;
pub const BASE: [u8; 32] = [
    9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

struct Ladder {
    x2: Fe,
    z2: Fe,
    x3: Fe,
    z3: Fe,
}

impl Ladder {
    fn swap(&mut self, flag: Choice) {
        Fe::swap(flag, &mut self.x2, &mut self.x3);
        Fe::swap(flag, &mut self.z2, &mut self.z3);
    }

    fn step(&mut self, base: Fe) {
        let a = self.x2.sum(self.z2);
        let aa = a.square();
        let b = self.x2.diff(self.z2);
        let bb = b.square();
        let e = aa.diff(bb);
        let c = self.x3.sum(self.z3);
        let d = self.x3.diff(self.z3);
        let da = d.prod(a);
        let cb = c.prod(b);
        self.x3 = da.sum(cb).square();
        self.z3 = base.prod(da.diff(cb).square());
        self.x2 = aa.prod(bb);
        self.z2 = e.prod(aa.sum(e.scale()));
    }
}

impl Drop for Ladder {
    fn drop(&mut self) {
        self.x2.wipe();
        self.z2.wipe();
        self.x3.wipe();
        self.z3.wipe();
    }
}

pub fn mul(scalar: &[u8; 32], point: &[u8; 32]) -> [u8; 32] {
    let mut key = *scalar;
    key[0] &= 248;
    key[31] &= 127;
    key[31] |= 64;
    let base = Fe::unpack(point);
    let mut ladder = Ladder {
        x2: Fe::ONE,
        z2: Fe::ZERO,
        x3: base,
        z3: Fe::ONE,
    };
    let mut last = 0u8;
    for index in (0..255usize).rev() {
        let bit = (key[index >> 3] >> (index & 7)) & 1;
        ladder.swap(Choice::bit(bit ^ last));
        ladder.step(base);
        last = bit;
    }
    ladder.swap(Choice::bit(last));
    ct::wipe(&mut key);
    ladder.x2.prod(ladder.z2.invert()).pack()
}

pub fn base(scalar: &[u8; 32]) -> [u8; 32] {
    mul(scalar, &BASE)
}

pub fn agree(scalar: &[u8; 32], point: &[u8; 32]) -> Result<[u8; 32], Error> {
    let mut shared = mul(scalar, point);
    if ct::eq(&shared, &[0u8; 32]).flag() {
        ct::wipe(&mut shared);
        return Err(Error::Zero);
    }
    Ok(shared)
}
