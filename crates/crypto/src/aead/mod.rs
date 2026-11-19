mod chacha;
mod poly;

pub use chacha::Chacha20;
pub use poly::Poly1305;

use crate::{Error, ct};

const LIMIT: u64 = (1 << 38) - 64;
const ZEROS: [u8; 16] = [0u8; 16];

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn pad(poly: &mut Poly1305, length: usize) {
    poly.update(&ZEROS[..(16 - length % 16) % 16]);
}

pub struct Chacha20poly1305 {
    key: [u8; 32],
}

impl Chacha20poly1305 {
    pub const KEY: usize = 32;
    pub const NONCE: usize = 12;
    pub const TAG: usize = 16;

    pub fn new(key: &[u8; 32]) -> Self {
        Chacha20poly1305 { key: *key }
    }

    pub fn seal(&self, nonce: &[u8; 12], aad: &[u8], data: &mut [u8]) -> Result<[u8; 16], Error> {
        if data.len() as u64 > LIMIT {
            return Err(Error::Length);
        }
        Chacha20::new(&self.key, nonce, 1).apply(data)?;
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
        Chacha20::new(&self.key, nonce, 1).apply(data)
    }

    fn tag(&self, nonce: &[u8; 12], aad: &[u8], cipher: &[u8]) -> [u8; 16] {
        let mut block = Chacha20::block(&self.key, nonce, 0);
        let mut onetime = [0u8; 32];
        onetime.copy_from_slice(&block[..32]);
        ct::wipe(&mut block);
        let mut poly = Poly1305::new(&onetime);
        ct::wipe(&mut onetime);
        poly.update(aad);
        pad(&mut poly, aad.len());
        poly.update(cipher);
        pad(&mut poly, cipher.len());
        poly.update(&(aad.len() as u64).to_le_bytes());
        poly.update(&(cipher.len() as u64).to_le_bytes());
        poly.finish()
    }
}

impl Drop for Chacha20poly1305 {
    fn drop(&mut self) {
        ct::wipe(&mut self.key);
    }
}
