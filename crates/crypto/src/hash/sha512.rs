use crate::ct;

const K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

const H512: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

const H384: [u64; 8] = [
    0xcbbb9d5dc1059ed8,
    0x629a292a367cd507,
    0x9159015a3070dd17,
    0x152fecd8f70e5939,
    0x67332667ffc00b31,
    0x8eb44a8768581511,
    0xdb0c2e0d64f98fa7,
    0x47b5481dbefa4fa4,
];

#[derive(Clone)]
struct Core {
    state: [u64; 8],
    block: [u8; 128],
    fill: usize,
    total: u64,
}

impl Core {
    fn new(state: [u64; 8]) -> Self {
        Core {
            state,
            block: [0; 128],
            fill: 0,
            total: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        self.absorb(data);
    }

    fn finish(mut self) -> [u8; 64] {
        let bits = u128::from(self.total) * 8;
        let pad = if self.fill < 112 {
            112 - self.fill
        } else {
            240 - self.fill
        };
        self.absorb(&[0x80]);
        self.absorb(&[0; 128][..pad - 1]);
        self.absorb(&bits.to_be_bytes());
        let mut out = [0u8; 64];
        for (chunk, word) in out.chunks_exact_mut(8).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn absorb(&mut self, mut data: &[u8]) {
        if self.fill > 0 {
            let take = (128 - self.fill).min(data.len());
            self.block[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill < 128 {
                return;
            }
            compress(&mut self.state, &self.block);
        }
        let mut blocks = data.chunks_exact(128);
        for block in blocks.by_ref() {
            compress(&mut self.state, block);
        }
        let rest = blocks.remainder();
        self.block[..rest.len()].copy_from_slice(rest);
        self.fill = rest.len();
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        ct::wipe(&mut self.state);
        ct::wipe(&mut self.block);
        self.fill = 0;
        self.total = 0;
    }
}

#[derive(Clone)]
pub struct Sha512 {
    core: Core,
}

impl Sha512 {
    pub const SIZE: usize = 64;
    pub const BLOCK: usize = 128;

    pub fn new() -> Self {
        Sha512 {
            core: Core::new(H512),
        }
    }

    pub fn digest(data: &[u8]) -> [u8; 64] {
        let mut hash = Sha512::new();
        hash.update(data);
        hash.finish()
    }

    pub fn update(&mut self, data: &[u8]) {
        self.core.update(data);
    }

    pub fn finish(self) -> [u8; 64] {
        self.core.finish()
    }
}

impl Default for Sha512 {
    fn default() -> Self {
        Sha512::new()
    }
}

#[derive(Clone)]
pub struct Sha384 {
    core: Core,
}

impl Sha384 {
    pub const SIZE: usize = 48;
    pub const BLOCK: usize = 128;

    pub fn new() -> Self {
        Sha384 {
            core: Core::new(H384),
        }
    }

    pub fn digest(data: &[u8]) -> [u8; 48] {
        let mut hash = Sha384::new();
        hash.update(data);
        hash.finish()
    }

    pub fn update(&mut self, data: &[u8]) {
        self.core.update(data);
    }

    pub fn finish(self) -> [u8; 48] {
        let mut full = self.core.finish();
        let mut out = [0u8; 48];
        out.copy_from_slice(&full[..48]);
        ct::wipe(&mut full);
        out
    }
}

impl Default for Sha384 {
    fn default() -> Self {
        Sha384::new()
    }
}

fn compress(state: &mut [u64; 8], block: &[u8]) {
    let mut words = [0u64; 80];
    for (slot, chunk) in words.iter_mut().zip(block.chunks_exact(8)) {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(chunk);
        *slot = u64::from_be_bytes(bytes);
    }
    for at in 16..80 {
        let low = words[at - 15];
        let high = words[at - 2];
        let first = low.rotate_right(1) ^ low.rotate_right(8) ^ (low >> 7);
        let second = high.rotate_right(19) ^ high.rotate_right(61) ^ (high >> 6);
        words[at] = words[at - 16]
            .wrapping_add(first)
            .wrapping_add(words[at - 7])
            .wrapping_add(second);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (constant, word) in K.iter().zip(words.iter()) {
        let sigma = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let choose = (e & f) ^ (!e & g);
        let one = h
            .wrapping_add(sigma)
            .wrapping_add(choose)
            .wrapping_add(*constant)
            .wrapping_add(*word);
        let sum = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let major = (a & b) ^ (a & c) ^ (b & c);
        let two = sum.wrapping_add(major);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(one);
        d = c;
        c = b;
        b = a;
        a = one.wrapping_add(two);
    }
    for (slot, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(add);
    }
    ct::wipe(&mut words);
}
