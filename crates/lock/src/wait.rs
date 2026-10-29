use crate::Error;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default)]
pub struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Wait {
    timeout: Option<Duration>,
    immediate: bool,
    cancellation: Option<Cancellation>,
}

impl Wait {
    #[cfg(feature = "tokio")]
    pub(crate) fn budget(&self) -> Option<Duration> {
        self.timeout
    }

    pub(crate) fn check(&self) -> Result<(), Error> {
        if self
            .cancellation
            .as_ref()
            .is_some_and(Cancellation::cancelled)
        {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }

    pub(crate) fn eager(&self) -> bool {
        self.immediate
    }

    pub fn forever() -> Self {
        Self::default()
    }

    pub fn immediate() -> Self {
        Self {
            immediate: true,
            ..Self::default()
        }
    }

    pub fn timeout(duration: Duration) -> Self {
        Self {
            timeout: Some(duration),
            ..Self::default()
        }
    }

    pub fn cancel(mut self, cancellation: Cancellation) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    pub(crate) fn start(&self) -> Result<Deadline<'_>, Error> {
        let end = self
            .timeout
            .map(|duration| Instant::now().checked_add(duration));
        if matches!(end, Some(None)) {
            return Err(Error::Invalid("wait timeout exceeds monotonic clock range"));
        }
        Ok(Deadline {
            wait: self,
            end: end.flatten(),
        })
    }
}

pub(crate) struct Deadline<'a> {
    wait: &'a Wait,
    end: Option<Instant>,
}

impl Deadline<'_> {
    pub fn check(&self) -> Result<(), Error> {
        self.wait.check()
    }

    pub fn pause(&self) -> Result<Duration, Error> {
        self.check()?;
        if self.wait.immediate {
            return Err(Error::Busy);
        }
        let poll = Duration::from_millis(10);
        match self.end {
            Some(end) => {
                let remaining = end.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(Error::Timeout);
                }
                Ok(poll.min(remaining))
            }
            None => Ok(poll),
        }
    }

    pub fn retry(&self) -> Result<(), Error> {
        self.check()?;
        if self.end.is_some_and(|end| Instant::now() >= end) {
            return Err(Error::Timeout);
        }
        Ok(())
    }
}
