use super::sbox;
use crate::{Error, ct};

type Planes = [u64; 8];

const ONES: u64 = 0x0001_0001_0001_0001;
const NIBBLES: u64 = 0x1111_1111_1111_1111;
const ROWS: u64 = 0x1111_1111_1111_1111;

fn transpose(mut word: u64) -> u64 {
    let mut swap = (word ^ (word >> 7)) & 0x00aa_00aa_00aa_00aa;
    word ^= swap ^ (swap << 7);
    swap = (word ^ (word >> 14)) & 0x0000_cccc_0000_cccc;
    word ^= swap ^ (swap << 14);
    swap = (word ^ (word >> 28)) & 0x0000_0000_f0f0_f0f0;
    word ^ swap ^ (swap << 28)
}

fn pack(bytes: &[u8; 64]) -> Planes {
    let mut planes = [0u64; 8];
    for group in 0..8 {
        let mut chunk = [0u8; 8];
        chunk.copy_from_slice(&bytes[8 * group..8 * group + 8]);
        let spread = transpose(u64::from_le_bytes(chunk));
        for (index, plane) in planes.iter_mut().enumerate() {
            *plane |= ((spread >> (8 * index)) & 0xff) << (8 * group);
        }
    }
    planes
}

fn unpack(planes: &Planes) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    for group in 0..8 {
        let mut spread = 0u64;
        for (index, plane) in planes.iter().enumerate() {
            spread |= ((plane >> (8 * group)) & 0xff) << (8 * index);
        }
        let chunk = transpose(spread).to_le_bytes();
        bytes[8 * group..8 * group + 8].copy_from_slice(&chunk);
    }
    bytes
}

fn lanes(plane: u64) -> u64 {
    let mut out = plane & ROWS;
    for row in 1..4u32 {
        let shift = 4 * row;
        let low = (0xffffu64 >> shift) * ONES;
        let high = ((0xffffu64 << (16 - shift)) & 0xffff) * ONES;
        let part = plane & (ROWS << row);
        out |= ((part >> shift) & low) | ((part << (16 - shift)) & high);
    }
    out
}

fn rotate(plane: u64, step: u32) -> u64 {
    let low = (0xfu64 >> step) * NIBBLES;
    let high = ((0xfu64 << (4 - step)) & 0xf) * NIBBLES;
    ((plane >> step) & low) | ((plane << (4 - step)) & high)
}

fn mix(planes: &mut Planes) {
    let old = *planes;
    for index in 0..8 {
        let next = rotate(old[index], 1);
        let prev = if index == 0 { old[7] } else { old[index - 1] };
        let carry = rotate(prev, 1);
        let fold = if matches!(index, 1 | 3 | 4) {
            rotate(old[7], 1) ^ old[7]
        } else {
            0
        };
        planes[index] = next ^ carry ^ prev ^ rotate(old[index], 2) ^ rotate(old[index], 3) ^ fold;
    }
}

fn add(planes: &mut Planes, key: &Planes) {
    for (plane, byte) in planes.iter_mut().zip(key) {
        *plane ^= byte;
    }
}

fn subword(word: u32) -> u32 {
    let mut bytes = [0u8; 64];
    bytes[..4].copy_from_slice(&word.to_le_bytes());
    let mut planes = pack(&bytes);
    sbox::sub(&mut planes);
    let mut out = unpack(&planes);
    let value = u32::from_le_bytes([out[0], out[1], out[2], out[3]]);
    ct::wipe(&mut bytes);
    ct::wipe(&mut out);
    value
}

pub struct Aes {
    keys: [Planes; 15],
    rounds: u64,
}

impl Aes {
    pub const BLOCK: usize = 16;

    pub fn new(key: &[u8]) -> Result<Self, Error> {
        if key.len() != 16 && key.len() != 32 {
            return Err(Error::Length);
        }
        let count = key.len() / 4;
        let rounds = count + 6;
        let mut words = [0u32; 60];
        for (slot, quad) in words.iter_mut().zip(key.chunks_exact(4)) {
            *slot = u32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]);
        }
        let mut constant = 1u32;
        for index in count..4 * (rounds + 1) {
            let mut word = words[index - 1];
            if index % count == 0 {
                word = subword(word.rotate_right(8)) ^ constant;
                constant = ((constant << 1) ^ ((constant >> 7) * 0x1b)) & 0xff;
            } else if count > 6 && index % count == 4 {
                word = subword(word);
            }
            words[index] = words[index - count] ^ word;
        }
        let mut keys = [[0u64; 8]; 15];
        for (slot, quad) in keys.iter_mut().zip(words.chunks_exact(4)).take(rounds + 1) {
            let mut bytes = [0u8; 64];
            for lane in 0..4 {
                for (index, word) in quad.iter().enumerate() {
                    let start = 16 * lane + 4 * index;
                    bytes[start..start + 4].copy_from_slice(&word.to_le_bytes());
                }
            }
            *slot = pack(&bytes);
            ct::wipe(&mut bytes);
        }
        ct::wipe(&mut words);
        Ok(Aes {
            keys,
            rounds: rounds as u64,
        })
    }

    pub fn encrypt(&self, block: &mut [u8; 16]) {
        let mut bytes = [0u8; 64];
        bytes[..16].copy_from_slice(block);
        self.run(&mut bytes);
        block.copy_from_slice(&bytes[..16]);
        ct::wipe(&mut bytes);
    }

    pub(super) fn xor(&self, nonce: &[u8; 12], first: u32, data: &mut [u8]) {
        let mut counter = first;
        for chunk in data.chunks_mut(64) {
            let mut bytes = [0u8; 64];
            for lane in 0..4 {
                let start = 16 * lane;
                bytes[start..start + 12].copy_from_slice(nonce);
                let tail = counter.wrapping_add(lane as u32).to_be_bytes();
                bytes[start + 12..start + 16].copy_from_slice(&tail);
            }
            self.run(&mut bytes);
            for (slot, mask) in chunk.iter_mut().zip(&bytes) {
                *slot ^= mask;
            }
            ct::wipe(&mut bytes);
            counter = counter.wrapping_add(4);
        }
    }

    fn run(&self, bytes: &mut [u8; 64]) {
        let mut planes = pack(bytes);
        add(&mut planes, &self.keys[0]);
        let last = self.rounds as usize;
        for round in 1..=last {
            sbox::sub(&mut planes);
            for plane in planes.iter_mut() {
                *plane = lanes(*plane);
            }
            if round != last {
                mix(&mut planes);
            }
            add(&mut planes, &self.keys[round]);
        }
        *bytes = unpack(&planes);
        ct::wipe(&mut planes);
    }
}

impl Drop for Aes {
    fn drop(&mut self) {
        for keys in self.keys.iter_mut() {
            ct::wipe(keys);
        }
        self.rounds = 0;
    }
}
