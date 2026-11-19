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

    pub fn bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next() as u8).collect()
    }
}

pub fn array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes.try_into().expect("fixed length")
}

pub fn load(name: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(format!("vectors/{name}")).unwrap();
    serde_json::from_str(&text).unwrap()
}

pub fn field(row: &serde_json::Value, name: &str) -> Vec<u8> {
    hex(row[name].as_str().unwrap())
}
