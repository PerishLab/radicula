use crate::{Error, Failure, Held, Mode, Policy, Status, Wait, gate::Entry};
use std::{borrow::Borrow, fs::File, future::Future, sync::Arc};

pub struct Gate<F = File> {
    entry: Arc<Entry<F>>,
    mode: Mode,
    wait: Wait,
}

impl<F> Clone for Gate<F> {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            mode: self.mode,
            wait: self.wait.clone(),
        }
    }
}

impl<F: Borrow<File>> Gate<F> {
    pub fn new(file: F, policy: Policy) -> Result<Self, Error> {
        if matches!(policy, Policy::Limit(n) if n.get() > 1) {
            return Err(Error::Unsupported(
                "whole-file carrier supports exclusive or read/write policy, not capacity greater than one",
            ));
        }
        Ok(Self {
            entry: Arc::new(Entry::new(policy, Some(file))),
            mode: Mode::Exclusive,
            wait: Wait::forever(),
        })
    }

    pub fn file(&self) -> &File {
        self.entry.file.as_ref().expect("bound file").borrow()
    }

    pub fn wait(&self, wait: Wait) -> Self {
        Self {
            wait,
            ..self.clone()
        }
    }

    pub fn shared(&self) -> Result<Self, Error> {
        if self.entry.policy != Policy::Readers {
            return Err(Error::Unsupported(
                "shared admission requires a read/write policy",
            ));
        }
        Ok(Self {
            mode: Mode::Shared,
            ..self.clone()
        })
    }

    pub fn status(&self) -> Status {
        self.entry.status()
    }

    pub fn acquire(&self, wait: Wait) -> Result<Permit<F>, Error> {
        self.entry.acquire(self.mode, wait)?;
        Ok(Permit {
            gate: Some(self.clone()),
        })
    }

    pub fn guard<T>(&self, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        self.with(self.wait.clone(), action)
    }

    pub fn with<T>(&self, wait: Wait, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        self.acquire(wait).map_err(Failure::Admission)?.run(action)
    }

    pub fn hold<T>(
        &self,
        wait: Wait,
        initialize: impl FnOnce() -> T,
    ) -> Result<Held<T, Permit<F>>, Error> {
        Ok(self.acquire(wait)?.attach(initialize()))
    }

    pub fn asynchronous(&self) -> Async<F> {
        Async(self.clone())
    }
}

#[must_use]
pub struct Permit<F: Borrow<File> = File> {
    gate: Option<Gate<F>>,
}

impl<F: Borrow<File>> Permit<F> {
    pub fn file(&self) -> &File {
        self.gate.as_ref().expect("owned file permit").file()
    }

    pub fn release(mut self) -> Result<(), Error> {
        self.gate.take().expect("owned file permit").entry.release()
    }

    pub fn run<T>(self, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        crate::scope::run(self, action)
    }

    pub fn attach<T>(self, value: T) -> Held<T, Self> {
        Held {
            value,
            _permit: self,
        }
    }

    pub fn during<B: Future>(
        self,
        action: impl FnOnce() -> B,
    ) -> impl Future<Output = Result<B::Output, Failure<B::Output>>> {
        crate::asynchronous::Execution::new(self, action())
    }
}

impl<F: Borrow<File>> Drop for Permit<F> {
    fn drop(&mut self) {
        if let Some(gate) = self.gate.take() {
            let _ = gate.entry.release();
        }
    }
}

impl<F: Borrow<File>> crate::scope::Release for Permit<F> {
    fn release(self) -> Result<(), Error> {
        Permit::release(self)
    }
}

pub struct Async<F>(Gate<F>);

impl<F> Clone for Async<F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F: Borrow<File>> Async<F> {
    pub async fn guard<B: Future>(
        self,
        action: impl FnOnce() -> B,
    ) -> Result<B::Output, Failure<B::Output>> {
        let wait = self.0.wait.clone();
        self.with(wait, action).await
    }

    pub async fn with<B: Future>(
        self,
        wait: Wait,
        action: impl FnOnce() -> B,
    ) -> Result<B::Output, Failure<B::Output>> {
        if !wait.eager() {
            return Err(Failure::Admission(Error::Unsupported(
                "runtime-independent async admission requires Wait::immediate()",
            )));
        }
        let permit = self.0.acquire(wait).map_err(Failure::Admission)?;
        permit.during(action).await
    }
}
