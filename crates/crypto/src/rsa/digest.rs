use crate::{Sha256, Sha384, Sha512};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hash {
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    pub(super) fn size(self) -> usize {
        match self {
            Hash::Sha256 => Sha256::SIZE,
            Hash::Sha384 => Sha384::SIZE,
            Hash::Sha512 => Sha512::SIZE,
        }
    }

    pub(super) fn prefix(self) -> &'static [u8] {
        match self {
            Hash::Sha256 => &[
                0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
                0x01, 0x05, 0x00, 0x04, 0x20,
            ],
            Hash::Sha384 => &[
                0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
                0x02, 0x05, 0x00, 0x04, 0x30,
            ],
            Hash::Sha512 => &[
                0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
                0x03, 0x05, 0x00, 0x04, 0x40,
            ],
        }
    }
}

enum Running {
    Small(Sha256),
    Middle(Sha384),
    Large(Sha512),
}

pub(super) struct Digest {
    running: Running,
}

impl Digest {
    pub fn new(hash: Hash) -> Self {
        let running = match hash {
            Hash::Sha256 => Running::Small(Sha256::new()),
            Hash::Sha384 => Running::Middle(Sha384::new()),
            Hash::Sha512 => Running::Large(Sha512::new()),
        };
        Digest { running }
    }

    pub fn update(&mut self, data: &[u8]) {
        match &mut self.running {
            Running::Small(inner) => inner.update(data),
            Running::Middle(inner) => inner.update(data),
            Running::Large(inner) => inner.update(data),
        }
    }

    pub fn finish(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        match self.running {
            Running::Small(inner) => out[..32].copy_from_slice(&inner.finish()),
            Running::Middle(inner) => out[..48].copy_from_slice(&inner.finish()),
            Running::Large(inner) => out.copy_from_slice(&inner.finish()),
        }
        out
    }

    pub fn once(hash: Hash, parts: &[&[u8]]) -> [u8; 64] {
        let mut digest = Digest::new(hash);
        for part in parts {
            digest.update(part);
        }
        digest.finish()
    }
}
