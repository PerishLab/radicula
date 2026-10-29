use super::{client::Client, key::Key};
use crate::Error;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fence {
    pub epoch: String,
    pub sequence: u64,
}

impl Fence {
    pub fn follows(&self, before: &Self) -> Result<bool, Error> {
        if self.epoch != before.epoch {
            return Err(Error::Epoch);
        }
        Ok(self.sequence > before.sequence)
    }
}

pub(super) struct Lease {
    pub key: Arc<Key>,
    pub token: String,
    pub fence: Fence,
    pub state: Mutex<State>,
}

pub(super) struct State {
    pub end: Instant,
    pub renew: Instant,
    pub lost: bool,
    pub closing: bool,
}

impl Lease {
    pub fn check(&self) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if Instant::now() >= state.end {
            state.lost = true;
        }
        if state.lost {
            self.key
                .fault("Redis lease is lost; old execution may still be alive");
            return Err(Error::Lost);
        }
        if state.closing {
            return Err(Error::Lost);
        }
        Ok(())
    }

    pub fn tick(&self, client: &Client) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let now = Instant::now();
        if state.closing || state.lost {
            return;
        }
        if now >= state.end {
            state.lost = true;
            drop(state);
            self.key
                .fault("Redis lease expired; old execution may still be alive");
            return;
        }
        if client.config.renewal.is_none() || now < state.renew {
            return;
        }
        let started = Instant::now();
        let renewed = client.renew(self);
        if renewed.is_ok() && Instant::now() < state.end {
            state.end = started + client.config.lease;
            state.renew = started + client.config.renewal.expect("renewal enabled");
        } else {
            state.lost = true;
            drop(state);
            self.key
                .fault("Redis renewal is unconfirmed; lease will never become valid automatically");
        }
    }

    pub fn release(&self, client: &Client) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let lost = state.lost || Instant::now() >= state.end;
        state.closing = true;
        drop(state);
        let result = client.release(self);
        self.key.finished();
        match result {
            Ok(()) if !lost => Ok(()),
            Ok(()) | Err(Error::Lost) => {
                self.key.fault("Redis lease is lost");
                Err(Error::Lost)
            }
            Err(error) => {
                self.key
                    .fault("Redis release is unconfirmed; lease expires without renewal");
                Err(error)
            }
        }
    }
}
