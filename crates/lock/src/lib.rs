#![forbid(unsafe_code)]

pub use std::fs::TryLockError;
use std::{fs::File, io};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Shared,
    Exclusive,
}

pub fn lock(file: &File, mode: Mode) -> io::Result<()> {
    match mode {
        Mode::Shared => file.lock_shared(),
        Mode::Exclusive => file.lock(),
    }
}

pub fn attempt(file: &File, mode: Mode) -> Result<(), TryLockError> {
    match mode {
        Mode::Shared => file.try_lock_shared(),
        Mode::Exclusive => file.try_lock(),
    }
}

pub fn unlock(file: &File) -> io::Result<()> {
    file.unlock()
}
