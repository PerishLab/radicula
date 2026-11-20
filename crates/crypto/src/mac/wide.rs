use crate::{Error, Sha384, Sha512, ct};

macro_rules! wide {
    ($mac:ident, $kdf:ident, $hash:ident) => {
        #[derive(Clone)]
        pub struct $mac {
            inner: $hash,
            outer: $hash,
        }

        impl $mac {
            pub const SIZE: usize = $hash::SIZE;

            pub fn new(key: &[u8]) -> Self {
                let mut pad = [0u8; $hash::BLOCK];
                if key.len() > $hash::BLOCK {
                    let mut digest = $hash::digest(key);
                    pad[..$hash::SIZE].copy_from_slice(&digest);
                    ct::wipe(&mut digest);
                } else {
                    pad[..key.len()].copy_from_slice(key);
                }
                let mut inner = $hash::new();
                let mut outer = $hash::new();
                for byte in pad.iter_mut() {
                    *byte ^= 0x36;
                }
                inner.update(&pad);
                for byte in pad.iter_mut() {
                    *byte ^= 0x36 ^ 0x5c;
                }
                outer.update(&pad);
                ct::wipe(&mut pad);
                $mac { inner, outer }
            }

            pub fn update(&mut self, data: &[u8]) {
                self.inner.update(data);
            }

            pub fn finish(self) -> [u8; $hash::SIZE] {
                let $mac { inner, mut outer } = self;
                let mut digest = inner.finish();
                outer.update(&digest);
                ct::wipe(&mut digest);
                outer.finish()
            }

            pub fn verify(self, tag: &[u8]) -> bool {
                let mut computed = self.finish();
                let same = ct::eq(&computed, tag).flag();
                ct::wipe(&mut computed);
                same
            }
        }

        #[derive(Clone)]
        pub struct $kdf {
            keyed: $mac,
        }

        impl $kdf {
            pub const LIMIT: usize = 255 * $hash::SIZE;

            pub fn extract(salt: &[u8], ikm: &[u8]) -> Self {
                let mut mac = $mac::new(salt);
                mac.update(ikm);
                let mut prk = mac.finish();
                let keyed = $mac::new(&prk);
                ct::wipe(&mut prk);
                $kdf { keyed }
            }

            pub fn new(prk: &[u8]) -> Result<Self, Error> {
                if prk.len() < $hash::SIZE {
                    return Err(Error::Length);
                }
                Ok($kdf {
                    keyed: $mac::new(prk),
                })
            }

            pub fn expand(&self, info: &[u8], out: &mut [u8]) -> Result<(), Error> {
                if out.len() > Self::LIMIT {
                    return Err(Error::Length);
                }
                let mut block = [0u8; $hash::SIZE];
                let mut previous = 0;
                for (index, chunk) in out.chunks_mut($hash::SIZE).enumerate() {
                    let mut mac = self.keyed.clone();
                    mac.update(&block[..previous]);
                    mac.update(info);
                    mac.update(&[index as u8 + 1]);
                    block = mac.finish();
                    previous = $hash::SIZE;
                    chunk.copy_from_slice(&block[..chunk.len()]);
                }
                ct::wipe(&mut block);
                Ok(())
            }
        }
    };
}

wide!(Hmac384, Hkdf384, Sha384);
wide!(Hmac512, Hkdf512, Sha512);
