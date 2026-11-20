mod aes;
mod ghash;
mod sbox;

pub use aes::Aes;
pub use ghash::Ghash;

use crate::{Error, ct};

const LIMIT: u64 = (1 << 36) - 32;

pub struct Aesgcm {
    aes: Aes,
    hash: [u8; 16],
}

impl Aesgcm {
    pub const NONCE: usize = 12;
    pub const TAG: usize = 16;

    pub fn new(key: &[u8]) -> Result<Self, Error> {
        let aes = Aes::new(key)?;
        let mut hash = [0u8; 16];
        aes.encrypt(&mut hash);
        Ok(Aesgcm { aes, hash })
    }

    pub fn seal(&self, nonce: &[u8; 12], aad: &[u8], data: &mut [u8]) -> Result<[u8; 16], Error> {
        if data.len() as u64 > LIMIT {
            return Err(Error::Length);
        }
        self.aes.xor(nonce, 2, data);
        Ok(self.tag(nonce, aad, data))
    }

    pub fn open(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        data: &mut [u8],
        tag: &[u8],
    ) -> Result<(), Error> {
        if data.len() as u64 > LIMIT {
            return Err(Error::Length);
        }
        let mut computed = self.tag(nonce, aad, data);
        let same = ct::eq(&computed, tag).flag();
        ct::wipe(&mut computed);
        if !same {
            return Err(Error::Auth);
        }
        self.aes.xor(nonce, 2, data);
        Ok(())
    }

    fn tag(&self, nonce: &[u8; 12], aad: &[u8], cipher: &[u8]) -> [u8; 16] {
        let mut ghash = Ghash::new(&self.hash);
        ghash.update(aad);
        ghash.pad();
        ghash.update(cipher);
        ghash.pad();
        ghash.update(&(aad.len() as u64 * 8).to_be_bytes());
        ghash.update(&(cipher.len() as u64 * 8).to_be_bytes());
        let mut tag = ghash.finish();
        let mut mask = [0u8; 16];
        mask[..12].copy_from_slice(nonce);
        mask[15] = 1;
        self.aes.encrypt(&mut mask);
        for (slot, byte) in tag.iter_mut().zip(&mask) {
            *slot ^= byte;
        }
        ct::wipe(&mut mask);
        tag
    }
}

impl Drop for Aesgcm {
    fn drop(&mut self) {
        ct::wipe(&mut self.hash);
    }
}
