use super::field::{self, Limbs, N, ZERO};
use super::point::Point;
use crate::{Error, Sha256};

fn padded(body: &[u8]) -> bool {
    body.len() > 1 && body[0] == 0 && body[1] & 0x80 == 0
}

fn element(data: &[u8]) -> Result<([u8; 32], &[u8]), Error> {
    if data.len() < 3 || data[0] != 2 || data[1] >= 0x80 {
        return Err(Error::Format);
    }
    let size = usize::from(data[1]);
    if size == 0 || data.len() < 2 + size {
        return Err(Error::Format);
    }
    let body = &data[2..2 + size];
    if body[0] & 0x80 != 0 || padded(body) {
        return Err(Error::Format);
    }
    let digits = if body[0] == 0 { &body[1..] } else { body };
    if digits.len() > 32 {
        return Err(Error::Format);
    }
    let mut out = [0u8; 32];
    out[32 - digits.len()..].copy_from_slice(digits);
    Ok((out, &data[2 + size..]))
}

fn parse(der: &[u8]) -> Result<[[u8; 32]; 2], Error> {
    if der.len() < 8 || der[0] != 0x30 || der[1] >= 0x80 {
        return Err(Error::Format);
    }
    if usize::from(der[1]) + 2 != der.len() {
        return Err(Error::Format);
    }
    let (first, rest) = element(&der[2..])?;
    let (second, rest) = element(rest)?;
    if !rest.is_empty() {
        return Err(Error::Format);
    }
    Ok([first, second])
}

fn combine(first: &Limbs, second: &Limbs, key: &Point) -> Point {
    let base = Point::base();
    let both = base.add(key);
    let mut acc = Point::identity();
    for bit in (0..256).rev() {
        acc = acc.double();
        let (word, shift) = (bit / 64, bit % 64);
        acc = match ((first[word] >> shift) & 1, (second[word] >> shift) & 1) {
            (1, 1) => acc.add(&both),
            (1, 0) => acc.add(&base),
            (0, 1) => acc.add(key),
            _ => acc,
        };
    }
    acc
}

fn check(key: &[u8], digest: &[u8; 32], pair: &[[u8; 32]; 2]) -> Result<(), Error> {
    let public = Point::decode(key)?;
    let (r, s) = (field::load(&pair[0]), field::load(&pair[1]));
    let order = N.modulus();
    let unfit = |value: &Limbs| *value == ZERO || !field::below(value, &order);
    if unfit(&r) || unfit(&s) {
        return Err(Error::Auth);
    }
    let hash = N.reduce(&field::load(digest));
    let inverse = N.invert(&N.lift(&s));
    let first = N.lower(&N.mul(&N.lift(&hash), &inverse));
    let second = N.lower(&N.mul(&N.lift(&r), &inverse));
    let total = combine(&first, &second, &public);
    match total.abscissa() {
        Some(x) if N.reduce(&x) == r => Ok(()),
        _ => Err(Error::Auth),
    }
}

pub fn verify(key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), Error> {
    let pair = parse(signature)?;
    check(key, &Sha256::digest(message), &pair)
}

pub fn ieee(key: &[u8], message: &[u8], signature: &[u8; 64]) -> Result<(), Error> {
    let mut pair = [[0u8; 32]; 2];
    pair[0].copy_from_slice(&signature[..32]);
    pair[1].copy_from_slice(&signature[32..]);
    check(key, &Sha256::digest(message), &pair)
}
