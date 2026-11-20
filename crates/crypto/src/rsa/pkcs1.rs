use super::digest::{Digest, Hash};
use super::key::Key;
use crate::Error;

pub fn verify(key: &Key, hash: Hash, message: &[u8], signature: &[u8]) -> Result<(), Error> {
    let recovered = key.apply(signature)?;
    let size = key.bytes();
    let digest = Digest::once(hash, &[message]);
    let prefix = hash.prefix();
    let tail = prefix.len() + hash.size();
    let mut expected = [0xffu8; 512];
    expected[0] = 0;
    expected[1] = 1;
    expected[size - tail - 1] = 0;
    expected[size - tail..size - hash.size()].copy_from_slice(prefix);
    expected[size - hash.size()..size].copy_from_slice(&digest[..hash.size()]);
    if recovered[..size] == expected[..size] {
        Ok(())
    } else {
        Err(Error::Auth)
    }
}
