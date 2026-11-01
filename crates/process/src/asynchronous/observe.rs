use crate::Cancel;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Running,
    Stopping,
    Settled,
    Uncertain,
}

struct Shared {
    state: AtomicU8,
    changed: tokio::sync::Notify,
}

#[derive(Clone)]
pub struct Observer {
    shared: Arc<Shared>,
    pub(crate) cancel: Cancel,
}

impl Observer {
    pub(crate) fn new(cancel: Cancel) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: AtomicU8::new(0),
                changed: tokio::sync::Notify::new(),
            }),
            cancel,
        }
    }

    pub fn state(&self) -> State {
        match self.shared.state.load(Ordering::Acquire) {
            0 if self.cancel.cancelled() => State::Stopping,
            0 => State::Running,
            1 => State::Stopping,
            2 => State::Settled,
            _ => State::Uncertain,
        }
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub async fn finish(&self) -> State {
        loop {
            let notified = self.shared.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let state = self.state();
            if matches!(state, State::Settled | State::Uncertain) {
                return state;
            }
            notified.await;
        }
    }

    pub(crate) fn set(&self, state: State) {
        let value = match state {
            State::Running => 0,
            State::Stopping => 1,
            State::Settled => 2,
            State::Uncertain => 3,
        };
        self.shared.state.store(value, Ordering::Release);
        self.shared.changed.notify_waiters();
    }
}

pub(crate) struct End(pub Observer);

impl Drop for End {
    fn drop(&mut self) {
        if !matches!(self.0.state(), State::Settled | State::Uncertain) {
            self.0.set(State::Uncertain);
        }
    }
}
