use crate::{Error, Mode, Policy, Status, TryLockError};
use std::{
    fs::File,
    io,
    sync::{Arc, Condvar, Mutex},
};

pub(crate) struct Entry {
    pub policy: Policy,
    pub state: Mutex<State>,
    pub changed: Condvar,
    pub file: Option<File>,
}

#[derive(Default)]
pub(crate) struct State {
    pub active: usize,
    pub mode: Option<Mode>,
    pub fault: Option<Arc<io::Error>>,
}

impl Entry {
    pub fn new(policy: Policy, file: Option<File>) -> Self {
        Self {
            policy,
            state: Mutex::default(),
            changed: Condvar::new(),
            file,
        }
    }

    pub fn attempt(&self, mode: Mode) -> Result<bool, Error> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.healthy()?;
        if !state.admits(self.policy, mode) {
            return Ok(false);
        }
        if state.active == 0 && !self.lock(mode)? {
            return Ok(false);
        }
        state.active += 1;
        state.mode = Some(mode);
        Ok(true)
    }

    fn lock(&self, mode: Mode) -> Result<bool, Error> {
        let Some(file) = &self.file else {
            return Ok(true);
        };
        match crate::attempt(file, mode) {
            Ok(()) => Ok(true),
            Err(TryLockError::WouldBlock) => Ok(false),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub fn release(&self) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.active -= 1;
        if state.active == 0 {
            state.mode = None;
            state.fault = self
                .file
                .as_ref()
                .and_then(|file| crate::unlock(file).err())
                .map(Arc::new);
        }
        let result = state.healthy();
        drop(state);
        self.changed.notify_all();
        result
    }

    pub fn status(&self) -> Status {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        Status {
            active: state.active,
            fault: state.fault.clone(),
        }
    }
}

impl State {
    fn admits(&self, policy: Policy, mode: Mode) -> bool {
        if self.active == usize::MAX {
            return false;
        }
        match policy {
            Policy::Limit(capacity) => self.active < capacity.get(),
            Policy::Readers => {
                self.active == 0 || (self.mode == Some(Mode::Shared) && mode == Mode::Shared)
            }
        }
    }

    fn healthy(&self) -> Result<(), Error> {
        match &self.fault {
            Some(error) => Err(Error::Uncertain(error.clone())),
            None => Ok(()),
        }
    }
}
