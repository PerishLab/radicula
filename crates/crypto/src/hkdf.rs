use crate::{Error, Hmac, Sha256, ct};

const LIMIT: usize = 255 * Sha256::SIZE;

#[derive(Clone)]
pub struct Hkdf {
    keyed: Hmac,
}

impl Hkdf {
    pub const LIMIT: usize = LIMIT;

    pub fn extract(salt: &[u8], ikm: &[u8]) -> Self {
        let mut mac = Hmac::new(salt);
        mac.update(ikm);
        let mut prk = mac.finish();
        let keyed = Hmac::new(&prk);
        ct::wipe(&mut prk);
        Hkdf { keyed }
    }

    pub fn new(prk: &[u8]) -> Result<Self, Error> {
        if prk.len() < Sha256::SIZE {
            return Err(Error::Length);
        }
        Ok(Hkdf {
            keyed: Hmac::new(prk),
        })
    }

    pub fn expand(&self, info: &[u8], out: &mut [u8]) -> Result<(), Error> {
        if out.len() > LIMIT {
            return Err(Error::Length);
        }
        let mut block = [0u8; Sha256::SIZE];
        let mut previous = 0;
        for (index, chunk) in out.chunks_mut(Sha256::SIZE).enumerate() {
            let mut mac = self.keyed.clone();
            mac.update(&block[..previous]);
            mac.update(info);
            mac.update(&[index as u8 + 1]);
            block = mac.finish();
            previous = Sha256::SIZE;
            chunk.copy_from_slice(&block[..chunk.len()]);
        }
        ct::wipe(&mut block);
        Ok(())
    }
}
