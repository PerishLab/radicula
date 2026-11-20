use radicula_crypto::{Sha384, Sha512};

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

pub struct Algo {
    pub size: usize,
    pub whole: fn(&[u8]) -> Vec<u8>,
    pub parts: fn(&[u8], usize) -> Vec<u8>,
    pub pieces: fn(&[&[u8]]) -> Vec<u8>,
}

fn pieces384(pieces: &[&[u8]]) -> Vec<u8> {
    let mut hash = Sha384::new();
    for piece in pieces {
        hash.update(piece);
    }
    hash.finish().to_vec()
}

fn whole384(data: &[u8]) -> Vec<u8> {
    Sha384::digest(data).to_vec()
}

fn parts384(input: &[u8], step: usize) -> Vec<u8> {
    let mut hash = Sha384::new();
    for piece in input.chunks(step) {
        hash.update(piece);
    }
    hash.finish().to_vec()
}

fn pieces512(pieces: &[&[u8]]) -> Vec<u8> {
    let mut hash = Sha512::new();
    for piece in pieces {
        hash.update(piece);
    }
    hash.finish().to_vec()
}

fn whole512(bytes: &[u8]) -> Vec<u8> {
    Sha512::digest(bytes).to_vec()
}

fn parts512(source: &[u8], step: usize) -> Vec<u8> {
    let mut hash = Sha512::new();
    for piece in source.chunks(step) {
        hash.update(piece);
    }
    hash.finish().to_vec()
}

pub const SHA384: Algo = Algo {
    size: 48,
    whole: whole384,
    parts: parts384,
    pieces: pieces384,
};

pub const SHA512: Algo = Algo {
    size: 64,
    whole: whole512,
    parts: parts512,
    pieces: pieces512,
};
