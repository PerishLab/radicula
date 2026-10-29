#![forbid(unsafe_code)]

mod asynchronous;
mod factory;
mod gate;
mod model;
mod permit;
mod state;
mod wait;

pub use asynchronous::Async;
pub use factory::Factory;
pub use gate::Gate;
pub use model::{Error, Failure, Policy, Status};
pub use permit::{Held, Permit};
pub use wait::{Cancellation, Wait};

#[cfg(feature = "macros")]
pub use radicula_lock_macros::guard;

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
