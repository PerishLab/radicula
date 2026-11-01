mod engine;
mod model;
mod observe;
mod session;

pub use model::{Captured, Cause, Completed, Error, Failure, Limits, Streams};
pub use observe::{Observer, State};
pub use session::Session;

use crate::Cancel;
use std::process::Command;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Clone, Copy, Debug)]
pub struct Runner {
    limits: Limits,
}

impl Runner {
    pub fn new(limits: Limits) -> Result<Self, crate::Error> {
        if limits.time.is_zero() {
            return Err(crate::Error::Invalid("zero time budget"));
        }
        Ok(Self { limits })
    }

    pub fn start<I, O, E>(
        &self,
        command: Command,
        streams: Streams<I, O, E>,
        cancel: Cancel,
    ) -> Session<Streams<I, O, E>>
    where
        I: AsyncRead + Send + Unpin + 'static,
        O: AsyncWrite + Send + Unpin + 'static,
        E: AsyncWrite + Send + Unpin + 'static,
    {
        let observer = Observer::new(cancel);
        let task = tokio::spawn(engine::execute(
            command,
            streams,
            self.limits,
            observe::End(observer.clone()),
        ));
        Session::new(task, observer)
    }

    pub fn capture(&self, command: Command, input: Vec<u8>, cancel: Cancel) -> Session<Captured> {
        self.start(
            command,
            Streams::new(std::io::Cursor::new(input), Vec::new(), Vec::new()),
            cancel,
        )
    }

    pub fn blocking(
        &self,
        command: Command,
        input: Vec<u8>,
        cancel: Cancel,
    ) -> Result<std::process::Output, Error<Captured>> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(Error::Active);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(Error::Runtime)?;
        runtime.block_on(async { self.capture(command, input, cancel).output().await })
    }
}
