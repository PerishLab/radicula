use crate::{Error, Wait};
use std::time::Duration;
use tokio::time::Instant;

pub(super) struct Clock<'a> {
    wait: &'a Wait,
    end: Option<Instant>,
}

impl<'a> Clock<'a> {
    pub fn new(wait: &'a Wait) -> Result<Self, Error> {
        let end = wait
            .budget()
            .map(|budget| Instant::now().checked_add(budget));
        if matches!(end, Some(None)) {
            return Err(Error::Invalid("wait timeout exceeds Tokio clock range"));
        }
        Ok(Self {
            wait,
            end: end.flatten(),
        })
    }

    pub fn check(&self) -> Result<(), Error> {
        self.wait.check()
    }

    pub fn retry(&self) -> Result<(), Error> {
        self.check()?;
        if self.end.is_some_and(|end| Instant::now() >= end) {
            return Err(Error::Timeout);
        }
        Ok(())
    }

    pub fn pause(&self) -> Result<Duration, Error> {
        self.check()?;
        if self.wait.eager() {
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
}
