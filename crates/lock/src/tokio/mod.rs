mod clock;
mod execution;
mod permit;
#[cfg(test)]
mod tests;
mod worker;

use crate::{Error, Failure, Gate, Wait};
pub use permit::Permit;
use std::{
    future::Future,
    sync::{Arc, OnceLock},
    time::Duration,
};
use worker::Worker;

#[derive(Clone)]
pub struct Tokio {
    gate: Gate,
    worker: Arc<OnceLock<Result<Worker, Error>>>,
}

impl Tokio {
    pub(crate) fn new(gate: Gate) -> Self {
        Self {
            gate,
            worker: Arc::default(),
        }
    }

    pub fn asynchronous(&self) -> Self {
        self.clone()
    }

    pub async fn acquire(&self, wait: Wait) -> Result<Permit, Error> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| Error::Unsupported("Tokio admission requires a running runtime"))?;
        let deadline = clock::Clock::new(&wait)?;
        let worker = self.worker.get_or_init(Worker::new).clone()?;
        loop {
            deadline.check()?;
            let gate = self.gate.clone();
            let held = worker.clone();
            let reply = worker.send(move || {
                gate.attempt()
                    .map(|permit| permit.map(|permit| Permit::new(permit, held)))
            })?;
            let result = admitted(reply, &deadline, wait.eager()).await?;
            if let Some(permit) = result {
                return Ok(permit);
            }
            let duration = deadline.pause()?;
            tokio::time::sleep(duration).await;
            deadline.retry()?;
        }
    }

    pub async fn guard<F: Future>(
        &self,
        action: impl FnOnce() -> F,
    ) -> Result<F::Output, Failure<F::Output>> {
        self.with(self.gate.wait.clone(), action).await
    }

    pub async fn with<F: Future>(
        &self,
        wait: Wait,
        action: impl FnOnce() -> F,
    ) -> Result<F::Output, Failure<F::Output>> {
        let permit = self.acquire(wait).await.map_err(Failure::Admission)?;
        execution::Execution::new(permit, action()).await
    }

    pub async fn blocking<T: Send + 'static>(
        &self,
        wait: Wait,
        action: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, Failure<T>> {
        let permit = self.acquire(wait).await.map_err(Failure::Admission)?;
        match tokio::task::spawn_blocking(move || permit.run(action)).await {
            Ok(result) => result,
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(_) => Err(Failure::Admission(Error::Cancelled)),
        }
    }
}

fn check(deadline: &clock::Clock<'_>, immediate: bool) -> Result<(), Error> {
    if immediate {
        deadline.check()
    } else {
        deadline.retry()
    }
}

async fn admitted(
    reply: worker::Reply<Result<Option<Permit>, Error>>,
    deadline: &clock::Clock<'_>,
    immediate: bool,
) -> Result<Option<Permit>, Error> {
    let mut pending = Box::pin(worker::receive(reply));
    loop {
        tokio::select! {
            result = &mut pending => { deadline.check()?; return result?; }
            _ = tokio::time::sleep(Duration::from_millis(10)) => { check(deadline, immediate)?; }
        }
    }
}
