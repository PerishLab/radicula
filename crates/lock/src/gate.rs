use crate::{Async, Error, Failure, Held, Mode, Permit, Policy, Status, Wait, state::Entry};
use std::sync::Arc;

#[derive(Clone)]
pub struct Gate {
    pub(crate) entry: Arc<Entry>,
    pub(crate) mode: Mode,
    pub(crate) wait: Wait,
}

impl Gate {
    #[cfg(feature = "tokio")]
    pub fn tokio(&self) -> crate::tokio::Tokio {
        crate::tokio::Tokio::new(self.clone())
    }

    pub fn asynchronous(&self) -> Async {
        Async(self.clone())
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
            entry: self.entry.clone(),
            mode: Mode::Shared,
            wait: self.wait.clone(),
        })
    }

    pub fn status(&self) -> Status {
        self.entry.status()
    }

    pub fn acquire(&self, wait: Wait) -> Result<Permit, Error> {
        let deadline = wait.start()?;
        loop {
            deadline.check()?;
            if let Some(permit) = self.attempt()? {
                return Ok(permit);
            }
            let duration = deadline.pause()?;
            let state = self
                .entry
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let _ = self
                .entry
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
        let permit = self.acquire(wait).map_err(Failure::Admission)?;
        permit.run(action)
    }

    pub fn hold<T>(&self, wait: Wait, initialize: impl FnOnce() -> T) -> Result<Held<T>, Error> {
        let permit = self.acquire(wait)?;
        Ok(permit.attach(initialize()))
    }

    pub(crate) fn attempt(&self) -> Result<Option<Permit>, Error> {
        self.entry
            .attempt(self.mode)
            .map(|acquired| acquired.then(|| Permit::new(self.clone())))
    }
}
