use crate::{Sha256, ct};

#[derive(Clone)]
pub struct Hmac {
    inner: Sha256,
    outer: Sha256,
}

impl Hmac {
    pub const SIZE: usize = 32;

    pub fn new(key: &[u8]) -> Self {
        let mut pad = [0u8; Sha256::BLOCK];
        if key.len() > Sha256::BLOCK {
            let mut digest = Sha256::digest(key);
            pad[..Sha256::SIZE].copy_from_slice(&digest);
            ct::wipe(&mut digest);
        } else {
            pad[..key.len()].copy_from_slice(key);
        }
        let mut inner = Sha256::new();
        let mut outer = Sha256::new();
        for byte in pad.iter_mut() {
            *byte ^= 0x36;
        }
        inner.update(&pad);
        for byte in pad.iter_mut() {
            *byte ^= 0x36 ^ 0x5c;
        }
        outer.update(&pad);
        ct::wipe(&mut pad);
        Hmac { inner, outer }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    pub fn finish(self) -> [u8; 32] {
        let Hmac { inner, mut outer } = self;
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
