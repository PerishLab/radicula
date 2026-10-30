use super::worker::{self, Reply, Worker};
use crate::{Error, Failure, Held, Permit as Core};
use std::future::Future;

#[must_use]
pub struct Permit {
    core: Option<Core>,
    worker: Worker,
}

impl Permit {
    pub(super) fn new(core: Core, worker: Worker) -> Self {
        Self {
            core: Some(core),
            worker,
        }
    }

    pub fn attach<T>(self, value: T) -> Held<T, Self> {
        Held {
            value,
            _permit: self,
        }
    }

    pub fn during<F: Future>(
        self,
        action: impl FnOnce() -> F,
    ) -> impl Future<Output = Result<F::Output, Failure<F::Output>>> {
        super::execution::Execution::new(self, action())
    }

    pub async fn release(self) -> Result<(), Error> {
        worker::receive(self.finish()?).await?
    }

    pub(super) fn finish(mut self) -> Result<Reply<Result<(), Error>>, Error> {
        let core = self.core.take().expect("owned Tokio permit");
        self.worker.send(move || core.release())
    }

    pub(super) fn run<T>(mut self, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
        self.core.take().expect("owned Tokio permit").run(action)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if let Some(core) = self.core.take() {
            let _ = self.worker.send(move || drop(core));
        }
    }
}
