use super::field::{self, Limbs, P, ZERO};
use crate::Error;
use crate::ct::{self, Choice};

type Coords = [Limbs; 3];

const COEFF: Limbs = [
    0xd89c_df62_29c4_bddf,
    0xacf0_05cd_7884_3090,
    0xe5a2_20ab_f721_2ed6,
    0xdc30_061d_0487_4834,
];
const BASEX: Limbs = [
    0x79e7_30d4_18a9_143c,
    0x75ba_95fc_5fed_b601,
    0x79fb_732b_7762_2510,
    0x1890_5f76_a537_55c6,
];
const BASEY: Limbs = [
    0xddf2_5357_ce95_560a,
    0x8b4a_b8e4_ba19_e45c,
    0xd2e8_8688_dd21_f325,
    0x8571_ff18_2588_5d85,
];

fn sum(left: &Limbs, right: &Limbs) -> Limbs {
    P.add(left, right)
}

fn diff(left: &Limbs, right: &Limbs) -> Limbs {
    P.sub(left, right)
}

fn prod(left: &Limbs, right: &Limbs) -> Limbs {
    P.mul(left, right)
}

fn identity() -> Coords {
    [ZERO, P.unit(), ZERO]
}

fn sumof(left: &Coords, right: &Coords) -> Coords {
    let [x1, y1, z1] = left;
    let [x2, y2, z2] = right;
    let mut t0 = prod(x1, x2);
    let mut t1 = prod(y1, y2);
    let mut t2 = prod(z1, z2);
    let mut t3 = sum(x1, y1);
    let mut t4 = sum(x2, y2);
    t3 = prod(&t3, &t4);
    t4 = sum(&t0, &t1);
    t3 = diff(&t3, &t4);
    t4 = sum(y1, z1);
    let mut x3 = sum(y2, z2);
    t4 = prod(&t4, &x3);
    x3 = sum(&t1, &t2);
    t4 = diff(&t4, &x3);
    x3 = sum(x1, z1);
    let mut y3 = sum(x2, z2);
    x3 = prod(&x3, &y3);
    y3 = sum(&t0, &t2);
    y3 = diff(&x3, &y3);
    let mut z3 = prod(&COEFF, &t2);
    x3 = diff(&y3, &z3);
    z3 = sum(&x3, &x3);
    x3 = sum(&x3, &z3);
    z3 = diff(&t1, &x3);
    x3 = sum(&t1, &x3);
    y3 = prod(&COEFF, &y3);
    t1 = sum(&t2, &t2);
    t2 = sum(&t1, &t2);
    y3 = diff(&y3, &t2);
    y3 = diff(&y3, &t0);
    t1 = sum(&y3, &y3);
    y3 = sum(&t1, &y3);
    t1 = sum(&t0, &t0);
    t0 = sum(&t1, &t0);
    t0 = diff(&t0, &t2);
    t1 = prod(&t4, &y3);
    t2 = prod(&t0, &y3);
    y3 = prod(&x3, &z3);
    y3 = sum(&y3, &t2);
    x3 = prod(&t3, &x3);
    x3 = diff(&x3, &t1);
    z3 = prod(&t4, &z3);
    t1 = prod(&t3, &t0);
    z3 = sum(&z3, &t1);
    [x3, y3, z3]
}

fn twice(point: &Coords) -> Coords {
    let [x, y, z] = point;
    let mut t0 = prod(x, x);
    let t1 = prod(y, y);
    let mut t2 = prod(z, z);
    let mut t3 = prod(x, y);
    t3 = sum(&t3, &t3);
    let mut z3 = prod(x, z);
    z3 = sum(&z3, &z3);
    let mut y3 = prod(&COEFF, &t2);
    y3 = diff(&y3, &z3);
    let mut x3 = sum(&y3, &y3);
    y3 = sum(&x3, &y3);
    x3 = diff(&t1, &y3);
    y3 = sum(&t1, &y3);
    y3 = prod(&x3, &y3);
    x3 = prod(&x3, &t3);
    t3 = sum(&t2, &t2);
    t2 = sum(&t2, &t3);
    z3 = prod(&COEFF, &z3);
    z3 = diff(&z3, &t2);
    z3 = diff(&z3, &t0);
    t3 = sum(&z3, &z3);
    z3 = sum(&z3, &t3);
    t3 = sum(&t0, &t0);
    t0 = sum(&t3, &t0);
    t0 = diff(&t0, &t2);
    t0 = prod(&t0, &z3);
    y3 = sum(&y3, &t0);
    t0 = prod(y, z);
    t0 = sum(&t0, &t0);
    z3 = prod(&t0, &z3);
    x3 = diff(&x3, &z3);
    z3 = prod(&t0, &t1);
    z3 = sum(&z3, &z3);
    z3 = sum(&z3, &z3);
    [x3, y3, z3]
}

fn onecurve(x: &Limbs, y: &Limbs) -> Choice {
    let square = P.square(x);
    let cube = prod(&square, x);
    let triple = sum(&sum(x, x), x);
    let rhs = sum(&diff(&cube, &triple), &COEFF);
    field::same(&P.square(y), &rhs)
}

fn affine(point: &Coords) -> Option<[Limbs; 2]> {
    if field::vanishes(&point[2]).flag() {
        return None;
    }
    let inverse = P.invert(&point[2]);
    Some([
        P.lower(&prod(&point[0], &inverse)),
        P.lower(&prod(&point[1], &inverse)),
    ])
}

fn select(table: &[Coords; 16], nibble: u8) -> Coords {
    let mut out = [ZERO; 3];
    for (index, entry) in table.iter().enumerate() {
        let delta = u32::from(index as u8 ^ nibble);
        let flag = Choice::bit((delta.wrapping_sub(1) >> 8) as u8);
        let mask = flag.select(u64::MAX, 0);
        for (slot, source) in out.iter_mut().zip(entry) {
            for (limb, word) in slot.iter_mut().zip(source) {
                *limb |= word & mask;
            }
        }
    }
    out
}

fn wipe(coords: &mut Coords) {
    for part in coords.iter_mut() {
        ct::wipe(part);
    }
}

pub struct Point {
    coords: Coords,
}

impl Point {
    pub const SIZE: usize = 65;

    pub fn identity() -> Self {
        Point { coords: identity() }
    }

    pub fn base() -> Self {
        Point {
            coords: [BASEX, BASEY, P.unit()],
        }
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != Self::SIZE || bytes[0] != 4 {
            return Err(Error::Point);
        }
        let x = field::load(&bytes[1..33]);
        let y = field::load(&bytes[33..65]);
        let modulus = P.modulus();
        if !field::below(&x, &modulus) || !field::below(&y, &modulus) {
            return Err(Error::Point);
        }
        let (x, y) = (P.lift(&x), P.lift(&y));
        if !onecurve(&x, &y).flag() {
            return Err(Error::Point);
        }
        Ok(Point {
            coords: [x, y, P.unit()],
        })
    }

    pub fn encode(&self) -> Result<[u8; 65], Error> {
        let Some([x, y]) = affine(&self.coords) else {
            return Err(Error::Zero);
        };
        let mut out = [0u8; 65];
        out[0] = 4;
        out[1..33].copy_from_slice(&field::store(&x));
        out[33..65].copy_from_slice(&field::store(&y));
        Ok(out)
    }

    pub fn neutral(&self) -> bool {
        field::vanishes(&self.coords[2]).flag()
    }

    pub fn add(&self, other: &Point) -> Point {
        Point {
            coords: sumof(&self.coords, &other.coords),
        }
    }

    pub fn double(&self) -> Point {
        Point {
            coords: twice(&self.coords),
        }
    }

    pub fn negate(&self) -> Point {
        Point {
            coords: [self.coords[0], P.negate(&self.coords[1]), self.coords[2]],
        }
    }

    pub fn mul(&self, scalar: &[u8; 32]) -> Point {
        let mut table = [identity(); 16];
        table[1] = self.coords;
        for index in 2..16 {
            table[index] = if index % 2 == 0 {
                twice(&table[index / 2])
            } else {
                sumof(&table[index - 1], &self.coords)
            };
        }
        let mut acc = identity();
        for index in 0..64 {
            for _ in 0..4 {
                acc = twice(&acc);
            }
            let byte = scalar[index / 2];
            let nibble = if index % 2 == 0 { byte >> 4 } else { byte & 15 };
            let mut picked = select(&table, nibble);
            acc = sumof(&acc, &picked);
            wipe(&mut picked);
        }
        for entry in table.iter_mut() {
            wipe(entry);
        }
        Point { coords: acc }
    }

    pub(super) fn abscissa(&self) -> Option<Limbs> {
        affine(&self.coords).map(|[x, _]| x)
    }
}

impl Drop for Point {
    fn drop(&mut self) {
        wipe(&mut self.coords);
    }
}
