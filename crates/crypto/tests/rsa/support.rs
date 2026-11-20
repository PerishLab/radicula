use radicula_crypto::rsa::{Hash, Key};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

pub fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let padded = if digits.len() % 2 == 1 {
        let mut fixed = vec![b'0'];
        fixed.extend(digits);
        fixed
    } else {
        digits
    };
    padded
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

pub fn load(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(format!("vectors/rsa/{path}")).unwrap();
    serde_json::from_str(&text).unwrap()
}

pub fn files(folder: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(format!("vectors/rsa/{folder}"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

pub fn named(name: &str) -> Option<Hash> {
    match name {
        "SHA-256" | "SHA256" | "sha256" => Some(Hash::Sha256),
        "SHA-384" | "SHA384" | "sha384" => Some(Hash::Sha384),
        "SHA-512" | "SHA512" | "sha512" => Some(Hash::Sha512),
        _ => None,
    }
}

pub fn make(modulus: &str, exponent: &str) -> Result<Key, radicula_crypto::Error> {
    Key::new(&hex(modulus), &hex(exponent))
}

pub fn digest(hash: Hash, message: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    match hash {
        Hash::Sha256 => sha2::Sha256::digest(message).to_vec(),
        Hash::Sha384 => sha2::Sha384::digest(message).to_vec(),
        Hash::Sha512 => sha2::Sha512::digest(message).to_vec(),
    }
}

pub fn prefix(hash: Hash) -> Vec<u8> {
    let tail = match hash {
        Hash::Sha256 => "01050004 20",
        Hash::Sha384 => "02050004 30",
        Hash::Sha512 => "03050004 40",
    };
    let head = match hash {
        Hash::Sha256 => "3031300d 06096086 48016503 0402",
        Hash::Sha384 => "3041300d 06096086 48016503 0402",
        Hash::Sha512 => "3051300d 06096086 48016503 0402",
    };
    hex(&format!("{head}{tail}"))
}

pub struct Secret {
    pub key: rsa::RsaPrivateKey,
    pub modulus: String,
    pub exponent: String,
}

pub fn secret(row: &crate::vector::cavp::Record) -> Secret {
    let number = |text: &str| rsa::BigUint::from_bytes_be(&hex(text));
    let key = rsa::RsaPrivateKey::from_components(
        number(&row.modulus),
        number(&row.exponent),
        number(&row.secret),
        vec![number(&row.primes[0]), number(&row.primes[1])],
    )
    .unwrap();
    Secret {
        key,
        modulus: row.modulus.clone(),
        exponent: row.exponent.clone(),
    }
}

pub fn raw(secret: &Secret, encoded: &[u8]) -> Vec<u8> {
    let value = rsa::BigUint::from_bytes_be(encoded);
    let result =
        rsa::hazmat::rsa_decrypt_and_check(&secret.key, None::<&mut ChaCha8Rng>, &value).unwrap();
    let bytes = result.to_bytes_be();
    let mut out = vec![0u8; encoded.len() - bytes.len()];
    out.extend(bytes);
    out
}

pub fn pick(bits: usize) -> Secret {
    let row = crate::vector::cavp::records("SigVerPSS_186-3.rsp")
        .into_iter()
        .find(|row| row.bits == bits && row.hash == "SHA256" && row.pass)
        .unwrap();
    secret(&row)
}

pub fn stream(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}
