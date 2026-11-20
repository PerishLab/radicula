#![no_std]
#![deny(unsafe_code)]

mod aead;
mod error;
mod gcm;
mod hkdf;
mod hmac;
mod sha256;

pub mod ct;
pub mod x25519;

pub use aead::{Chacha20, Chacha20poly1305, Poly1305};
pub use error::Error;
pub use gcm::{Aes, Aesgcm, Ghash};
pub use hkdf::Hkdf;
pub use hmac::Hmac;
pub use sha256::Sha256;
