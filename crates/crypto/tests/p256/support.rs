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

pub fn array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes.try_into().expect("fixed length")
}

pub fn load(name: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(format!("vectors/p256/{name}")).unwrap();
    serde_json::from_str(&text).unwrap()
}

pub fn field(row: &serde_json::Value, name: &str) -> Vec<u8> {
    hex(row[name].as_str().unwrap())
}

pub const ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63, 0x25, 0x51,
];

pub fn padded(bytes: &[u8]) -> [u8; 32] {
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    let digits = &bytes[start..];
    assert!(digits.len() <= 32, "integer wider than 256 bits");
    let mut out = [0u8; 32];
    out[32 - digits.len()..].copy_from_slice(digits);
    out
}

pub fn subtract(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut borrow = 0i16;
    for index in (0..32).rev() {
        let mut step = i16::from(left[index]) - i16::from(right[index]) - borrow;
        borrow = i16::from(step < 0);
        step += 256 * borrow;
        out[index] = step as u8;
    }
    assert_eq!(borrow, 0, "subtraction underflow");
    out
}

pub fn uncompressed(x: &[u8], y: &[u8]) -> [u8; 65] {
    let mut out = [4u8; 65];
    out[1..33].copy_from_slice(x);
    out[33..].copy_from_slice(y);
    out
}

pub fn scalar(rng: &mut Rng) -> [u8; 32] {
    loop {
        let bytes: [u8; 32] = array(&rng.bytes(32));
        if bytes != [0u8; 32] && bytes < ORDER {
            return bytes;
        }
    }
}

pub fn integer(value: &[u8; 32]) -> Vec<u8> {
    let start = value.iter().position(|byte| *byte != 0).unwrap_or(31);
    let mut body = Vec::new();
    if value[start] & 0x80 != 0 {
        body.push(0);
    }
    body.extend_from_slice(&value[start..]);
    let mut out = vec![2, body.len() as u8];
    out.extend(body);
    out
}

pub fn der(first: &[u8; 32], second: &[u8; 32]) -> Vec<u8> {
    let mut body = integer(first);
    body.extend(integer(second));
    let mut out = vec![0x30, body.len() as u8];
    out.extend(body);
    out
}
