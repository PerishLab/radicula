use radicula_crypto::Error;

pub fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    assert_eq!(digits.len() % 2, 0, "odd hex length");
    digits
        .chunks_exact(2)
        .map(|pair| (nibble(pair[0]) << 4) | nibble(pair[1]))
        .collect()
}

fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => panic!("not a hex digit"),
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut mix = self.0;
        mix = (mix ^ (mix >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        mix = (mix ^ (mix >> 27)).wrapping_mul(0x94d049bb133111eb);
        mix ^ (mix >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    pub fn bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next() as u8).collect()
    }
}

pub type Made = Result<Vec<u8>, Error>;

pub struct Mac {
    pub size: usize,
    pub tag: fn(&[u8], &[&[u8]]) -> Vec<u8>,
    pub check: fn(&[u8], &[u8], &[u8]) -> bool,
}

pub struct Kdf {
    pub size: usize,
    pub limit: usize,
    pub derive: fn(&[u8], &[u8], &[u8], usize) -> Made,
    pub direct: fn(&[u8], &[u8], usize) -> Made,
}

macro_rules! suite {
    () => {
        pub const MAC: Mac = Mac {
            size: Hmac::SIZE,
            tag: |key, pieces| {
                let mut mac = Hmac::new(key);
                for piece in pieces {
                    mac.update(piece);
                }
                mac.finish().to_vec()
            },
            check: |key, data, tag| {
                let mut mac = Hmac::new(key);
                mac.update(data);
                mac.verify(tag)
            },
        };
        pub const KDF: Kdf = Kdf {
            size: Hmac::SIZE,
            limit: Hkdf::LIMIT,
            derive: |salt, ikm, info, length| {
                let kdf = Hkdf::extract(salt, ikm);
                let mut out = vec![0u8; length];
                kdf.expand(info, &mut out)?;
                Ok(out)
            },
            direct: |prk, info, length| {
                let kdf = Hkdf::new(prk)?;
                let mut out = vec![0u8; length];
                kdf.expand(info, &mut out)?;
                Ok(out)
            },
        };
    };
}

mod small {
    use super::{Kdf, Mac};
    use radicula_crypto::{Hkdf384 as Hkdf, Hmac384 as Hmac};

    suite!();
}

mod large {
    use super::{Kdf, Mac};
    use radicula_crypto::{Hkdf512 as Hkdf, Hmac512 as Hmac};

    suite!();
}

pub use large::{KDF as KDF512, MAC as MAC512};
pub use small::{KDF as KDF384, MAC as MAC384};
