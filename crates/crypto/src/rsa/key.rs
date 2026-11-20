use super::digest::Hash;
use super::int::{self, Int, Ring};
use super::{pkcs1, pss};
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pss {
    pub hash: Hash,
    pub mgf: Hash,
    pub salt: usize,
}

#[derive(Clone)]
pub struct Key {
    ring: Ring,
    exponent: u32,
    bits: usize,
}

fn strip(bytes: &[u8]) -> &[u8] {
    let zeros = bytes.iter().take_while(|byte| **byte == 0).count();
    &bytes[zeros..]
}

impl Key {
    pub const MIN: usize = 2048;
    pub const MAX: usize = 4096;

    pub fn new(modulus: &[u8], exponent: &[u8]) -> Result<Self, Error> {
        let limbs = int::load(strip(modulus)).ok_or(Error::Key)?;
        let bits = int::bits(&limbs);
        if limbs[0] & 1 == 0 || !(Self::MIN..=Self::MAX).contains(&bits) {
            return Err(Error::Key);
        }
        let exponent = strip(exponent);
        if exponent.len() > 4 || exponent.last().is_none_or(|last| last & 1 == 0) {
            return Err(Error::Key);
        }
        let value = exponent
            .iter()
            .fold(0u32, |acc, byte| (acc << 8) | u32::from(*byte));
        if value < 3 {
            return Err(Error::Key);
        }
        Ok(Key {
            ring: Ring::new(&limbs),
            exponent: value,
            bits,
        })
    }

    pub fn bits(&self) -> usize {
        self.bits
    }

    pub fn pkcs1(&self, hash: Hash, message: &[u8], signature: &[u8]) -> Result<(), Error> {
        pkcs1::verify(self, hash, message, signature)
    }

    pub fn pss(&self, scheme: Pss, message: &[u8], signature: &[u8]) -> Result<(), Error> {
        pss::verify(self, scheme, message, signature)
    }

    pub(super) fn bytes(&self) -> usize {
        self.bits.div_ceil(8)
    }

    pub(super) fn apply(&self, signature: &[u8]) -> Result<[u8; 512], Error> {
        let size = self.bytes();
        if signature.len() != size {
            return Err(Error::Auth);
        }
        let value: Int = int::load(signature).ok_or(Error::Auth)?;
        if !self.ring.below(&value) {
            return Err(Error::Auth);
        }
        let result = self.ring.power(&value, self.exponent);
        let mut out = [0u8; 512];
        for (at, slot) in out[..size].iter_mut().rev().enumerate() {
            *slot = (result[at / 8] >> (8 * (at % 8))) as u8;
        }
        Ok(out)
    }
}
