use crate::{Error, Failure, Gate};
use std::{future::Future, ops::Deref};

#[must_use]
pub struct Permit {
    gate: Option<Gate>,
}

impl Permit {
    pub(crate) fn new(gate: Gate) -> Self {
        Self { gate: Some(gate) }
    }

    pub fn release(mut self) -> Result<(), Error> {
        self.gate.take().expect("owned permit").entry.release()
    }

    pub fn run<T>(self, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        self.finish(action())
    }

    pub fn during<F: Future>(
        self,
        action: impl FnOnce() -> F,
    ) -> impl Future<Output = Result<F::Output, Failure<F::Output>>> {
        crate::asynchronous::Execution::new(self, action())
    }

    pub fn attach<T>(self, value: T) -> Held<T> {
        Held {
            value,
            _permit: self,
        }
    }

    pub(crate) fn finish<T>(self, output: T) -> Result<T, Failure<T>> {
        match self.release() {
            Ok(()) => Ok(output),
            Err(error) => Err(Failure::Release { output, error }),
        }
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if let Some(gate) = self.gate.take() {
            let _ = gate.entry.release();
        }
    }
}

#[must_use]
pub struct Held<T> {
    value: T,
    _permit: Permit,
}

impl<T> Deref for Held<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> Held<T> {
    pub fn edit(&mut self) -> &mut T {
        &mut self.value
    }
}
