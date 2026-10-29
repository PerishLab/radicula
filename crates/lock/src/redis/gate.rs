use super::{
    client::Client,
    key::Key,
    lease::{Fence, Lease},
};
use crate::{Error, Failure, Held, Mode, Policy, Status, Wait};
use std::sync::Arc;

#[derive(Clone)]
pub struct Gate {
    pub(super) client: Arc<Client>,
    pub(super) key: Arc<Key>,
    mode: Mode,
    wait: Wait,
}

impl Gate {
    pub(super) fn new(client: Arc<Client>, key: Arc<Key>) -> Self {
        Self {
            client,
            key,
            mode: Mode::Exclusive,
            wait: Wait::forever(),
        }
    }

    pub fn wait(&self, wait: Wait) -> Self {
        Self {
            wait,
            ..self.clone()
        }
    }

    pub fn shared(&self) -> Result<Self, Error> {
        if self.key.policy != Policy::Readers {
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
        self.key.status()
    }

    pub fn acquire(&self, wait: Wait) -> Result<Permit, Error> {
        let deadline = wait.start()?;
        loop {
            deadline.check()?;
            match self.client.acquire(self.key.clone(), self.mode) {
                Ok(Some(lease)) => return Ok(Permit::new(self.client.clone(), lease)),
                Ok(None) | Err(Error::Busy) => {}
                Err(error) => return Err(error),
            }
            let duration = deadline.pause()?;
            let state = self
                .key
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let _ = self
                .key
                .changed
                .wait_timeout(state, duration)
                .unwrap_or_else(|error| error.into_inner());
            deadline.retry()?;
        }
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
    ) -> Result<Held<T, Permit>, Error> {
        let permit = self.acquire(wait)?;
        permit.check()?;
        Ok(permit.attach(initialize()))
    }
}

#[must_use]
pub struct Permit {
    client: Arc<Client>,
    lease: Option<Arc<Lease>>,
}

impl Permit {
    pub(super) fn new(client: Arc<Client>, lease: Arc<Lease>) -> Self {
        Self {
            client,
            lease: Some(lease),
        }
    }

    pub fn check(&self) -> Result<(), Error> {
        self.lease.as_ref().expect("owned Redis permit").check()
    }

    pub fn fence(&self) -> Result<Fence, Error> {
        self.check()?;
        Ok(self
            .lease
            .as_ref()
            .expect("owned Redis permit")
            .fence
            .clone())
    }

    pub fn release(mut self) -> Result<(), Error> {
        self.lease
            .take()
            .expect("owned Redis permit")
            .release(&self.client)
    }

    pub fn run<T>(self, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        self.check().map_err(Failure::Admission)?;
        crate::scope::run(self, action)
    }

    pub fn attach<T>(self, value: T) -> Held<T, Self> {
        Held {
            value,
            _permit: self,
        }
    }
}

impl crate::scope::Release for Permit {
    fn release(self) -> Result<(), Error> {
        Permit::release(self)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let _ = lease.release(&self.client);
        }
    }
}
