#![no_std]
#![deny(unsafe_code)]

mod aead;
mod error;
mod gcm;
mod hash;
mod mac;

pub mod ct;
pub mod p256;
pub mod rsa;
pub mod x25519;

pub use aead::{Chacha20, Chacha20poly1305, Poly1305};
pub use error::Error;
pub use gcm::{Aes, Aesgcm, Ghash};
pub use hash::{Sha256, Sha384, Sha512};
pub use mac::{Hkdf, Hkdf384, Hkdf512, Hmac, Hmac384, Hmac512};
