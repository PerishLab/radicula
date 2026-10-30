use crate::{Error, Mode, Policy, Status, TryLockError, Wait};
use std::{
    borrow::Borrow,
    fs::File,
    io,
    sync::{Arc, Condvar, Mutex},
};

pub(crate) struct Entry<F = File> {
    pub policy: Policy,
    pub state: Mutex<State>,
    pub changed: Condvar,
    pub file: Option<F>,
}

#[derive(Default)]
pub(crate) struct State {
    pub active: usize,
    pub mode: Option<Mode>,
    pub fault: Option<Arc<io::Error>>,
}

impl<F: Borrow<File>> Entry<F> {
    pub fn new(policy: Policy, file: Option<F>) -> Self {
        Self {
            policy,
            state: Mutex::default(),
            changed: Condvar::new(),
            file,
        }
    }

    pub fn acquire(&self, mode: Mode, wait: Wait) -> Result<(), Error> {
        let deadline = wait.start()?;
        loop {
            deadline.check()?;
            if self.attempt(mode)? {
                return Ok(());
            }
            let duration = deadline.pause()?;
            let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            let _ = self
                .changed
                .wait_timeout(state, duration)
                .unwrap_or_else(|error| error.into_inner());
            deadline.retry()?;
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
        match crate::attempt(file.borrow(), mode) {
            Ok(()) => Ok(true),
            Err(TryLockError::WouldBlock) => Ok(false),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub fn release(&self) -> Result<(), Error> {
        self.finish(|| {
            self.file
                .as_ref()
                .map_or(Ok(()), |file| crate::unlock(file.borrow()))
        })
    }

    fn finish(&self, unlock: impl FnOnce() -> io::Result<()>) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let result = state.release(unlock);
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
    fn release(&mut self, unlock: impl FnOnce() -> io::Result<()>) -> Result<(), Error> {
        self.active -= 1;
        if self.active == 0 {
            self.mode = None;
            self.fault = unlock().err().map(Arc::new);
        }
        self.healthy()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault() {
        let mut state = State {
            active: 1,
            mode: Some(Mode::Exclusive),
            fault: None,
        };
        assert!(matches!(
            state.release(|| Err(io::Error::other("injected release failure"))),
            Err(Error::Uncertain(_))
        ));
        assert_eq!(state.active, 0);
        assert!(matches!(state.healthy(), Err(Error::Uncertain(_))));
        let entry = Entry::<File> {
            policy: Policy::EXCLUSIVE,
            state: Mutex::new(state),
            changed: Condvar::new(),
            file: None,
        };
        assert!(matches!(
            entry.attempt(Mode::Exclusive),
            Err(Error::Uncertain(_))
        ));
    }

    #[test]
    fn readers() {
        let mut state = State {
            active: 2,
            mode: Some(Mode::Shared),
            fault: None,
        };
        state
            .release(|| panic!("another reader still owns admission"))
            .unwrap();
        assert_eq!(state.active, 1);
        state.release(|| Ok(())).unwrap();
        assert_eq!(state.active, 0);
    }
    struct Failing<'a>(Option<&'a Entry>);

    impl crate::scope::Release for Failing<'_> {
        fn release(mut self) -> Result<(), Error> {
            self.0
                .take()
                .unwrap()
                .finish(|| Err(io::Error::other("injected unlock failure")))
        }
    }

    impl Drop for Failing<'_> {
        fn drop(&mut self) {
            if let Some(entry) = self.0.take() {
                let _ = entry.finish(|| Err(io::Error::other("injected unwind cleanup failure")));
            }
        }
    }

    #[test]
    fn output() {
        for panics in [false, true] {
            let entry = Entry::new(Policy::EXCLUSIVE, None::<File>);
            entry.acquire(Mode::Exclusive, Wait::immediate()).unwrap();
            let permit = Failing(Some(&entry));
            if panics {
                assert!(
                    std::panic::catch_unwind(|| crate::scope::run(permit, || panic!(
                        "business panic"
                    )))
                    .is_err()
                );
            } else {
                assert!(matches!(
                    crate::scope::run(permit, || Err::<(), _>("business failure")),
                    Err(crate::Failure::Release {
                        output: Err("business failure"),
                        error: Error::Uncertain(_)
                    })
                ));
            }
            assert_eq!(entry.status().active, 0);
            assert!(entry.status().fault.is_some());
            assert!(matches!(
                entry.acquire(Mode::Exclusive, Wait::immediate()),
                Err(Error::Uncertain(_))
            ));
        }
    }
}
