use crate::Fault;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

pub fn challenge(verifier: &str) -> Result<String, Fault> {
    if !(43..=128).contains(&verifier.len()) || !verifier.bytes().all(allowed) {
        return Err(Fault::Proof);
    }
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())))
}

pub fn verify(verifier: &str, expected: &str) -> Result<(), Fault> {
    if challenge(verifier)? != expected {
        return Err(Fault::Proof);
    }
    Ok(())
}

fn allowed(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~".contains(&byte)
}
