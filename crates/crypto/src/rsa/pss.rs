use super::digest::Digest;
use super::key::{Key, Pss};
use crate::Error;

fn mask(scheme: Pss, seed: &[u8], out: &mut [u8]) {
    for (counter, block) in out.chunks_mut(scheme.mgf.size()).enumerate() {
        let tag = (counter as u32).to_be_bytes();
        let digest = Digest::once(scheme.mgf, &[seed, &tag]);
        for (slot, byte) in block.iter_mut().zip(digest) {
            *slot ^= byte;
        }
    }
}

pub fn verify(key: &Key, scheme: Pss, message: &[u8], signature: &[u8]) -> Result<(), Error> {
    let recovered = key.apply(signature)?;
    let size = key.bytes();
    let embits = key.bits() - 1;
    let length = embits.div_ceil(8);
    let hash = scheme.hash.size();
    if length == size - 1 && recovered[0] != 0 {
        return Err(Error::Auth);
    }
    let encoded = &recovered[size - length..size];
    if scheme.salt.saturating_add(hash + 2) > length || encoded[length - 1] != 0xbc {
        return Err(Error::Auth);
    }
    let split = length - hash - 1;
    let spare = 8 * length - embits;
    if encoded[0] & !(0xff >> spare) != 0 {
        return Err(Error::Auth);
    }
    let mut block = [0u8; 512];
    block[..split].copy_from_slice(&encoded[..split]);
    mask(scheme, &encoded[split..split + hash], &mut block[..split]);
    block[0] &= 0xff >> spare;
    let pad = split - scheme.salt - 1;
    if block[..pad].iter().any(|byte| *byte != 0) || block[pad] != 1 {
        return Err(Error::Auth);
    }
    let digest = Digest::once(scheme.hash, &[message]);
    let salt = &block[split - scheme.salt..split];
    let check = Digest::once(scheme.hash, &[&[0u8; 8], &digest[..hash], salt]);
    if check[..hash] == encoded[split..split + hash] {
        Ok(())
    } else {
        Err(Error::Auth)
    }
}
