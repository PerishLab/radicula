use core::hint::black_box;
use core::ptr;
use core::sync::atomic::{Ordering, compiler_fence};

pub trait Zero: Copy {
    const ZERO: Self;
}

impl Zero for u8 {
    const ZERO: Self = 0;
}

impl Zero for u32 {
    const ZERO: Self = 0;
}

impl Zero for u64 {
    const ZERO: Self = 0;
}

#[derive(Clone, Copy)]
pub struct Choice(u8);

impl From<bool> for Choice {
    fn from(flag: bool) -> Self {
        Choice(black_box(u8::from(flag)))
    }
}

impl Choice {
    pub fn bit(value: u8) -> Self {
        Choice(black_box(value & 1))
    }

    pub fn flag(self) -> bool {
        black_box(self.0) == 1
    }

    pub fn select(self, left: u64, right: u64) -> u64 {
        let mask = u64::from(black_box(self.0)).wrapping_neg();
        (left & mask) | (right & !mask)
    }

    pub fn assign(self, target: &mut [u8], source: &[u8]) {
        assert_eq!(target.len(), source.len(), "assign needs equal lengths");
        let mask = black_box(self.0).wrapping_neg();
        for (slot, byte) in target.iter_mut().zip(source) {
            *slot = (*byte & mask) | (*slot & !mask);
        }
    }
}

pub fn eq(left: &[u8], right: &[u8]) -> Choice {
    if left.len() != right.len() {
        return Choice::from(false);
    }
    let mut spread = 0u8;
    for (one, two) in left.iter().zip(right) {
        spread |= one ^ two;
    }
    let spread = black_box(spread);
    Choice((u32::from(spread).wrapping_sub(1) >> 8) as u8 & 1)
}

#[allow(unsafe_code)]
pub fn wipe<T: Zero>(buffer: &mut [T]) {
    for slot in buffer.iter_mut() {
        unsafe { ptr::write_volatile(slot, T::ZERO) };
    }
    compiler_fence(Ordering::SeqCst);
}
