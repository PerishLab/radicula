use std::fmt;
use std::process::ExitStatus;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub time: Duration,
    pub output: u64,
}

pub struct Streams<I, O, E> {
    pub input: Option<I>,
    pub stdout: Option<O>,
    pub stderr: Option<E>,
}

impl<I, O, E> Streams<I, O, E> {
    pub fn new(input: I, stdout: O, stderr: E) -> Self {
        Self {
            input: Some(input),
            stdout: Some(stdout),
            stderr: Some(stderr),
        }
    }
}

impl<O> Streams<tokio::io::Empty, O, tokio::io::Sink> {
    pub fn stdout(stdout: O) -> Self {
        Self {
            input: None,
            stdout: Some(stdout),
            stderr: None,
        }
    }
}

pub type Captured = Streams<std::io::Cursor<Vec<u8>>, Vec<u8>, Vec<u8>>;

pub type Outcome<T> = Result<Completed<T>, Error<T>>;

pub struct Completed<T> {
    pub status: ExitStatus,
    pub streams: T,
}

#[derive(Debug)]
pub enum Cause {
    Io(std::io::Error),
    Time,
    Output,
    Cancelled,
}

pub struct Failure<T> {
    pub cause: Cause,
    pub streams: T,
    pub status: Option<ExitStatus>,
    pub cleanup: Option<std::io::Error>,
    pub child: Option<tokio::process::Child>,
}

pub enum Error<T> {
    Operation(Box<Failure<T>>),
    Interrupted {
        fault: tokio::task::JoinError,
        observer: super::Observer,
    },
    Runtime(std::io::Error),
    Active,
    Consumed,
}

impl<T> fmt::Debug for Completed<T> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Completed")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl<T> fmt::Debug for Failure<T> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Failure")
            .field("cause", &self.cause)
            .field("status", &self.status)
            .field("cleanup", &self.cleanup)
            .field("retained", &self.child.is_some())
            .finish_non_exhaustive()
    }
}

impl<T> fmt::Debug for Error<T> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(failure) => failure.fmt(out),
            Self::Interrupted { fault, .. } => out.debug_tuple("Interrupted").field(fault).finish(),
            Self::Runtime(error) => out.debug_tuple("Runtime").field(error).finish(),
            Self::Active => out.write_str("Active runtime"),
            Self::Consumed => out.write_str("Consumed session"),
        }
    }
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, out)
    }
}

impl<T> std::error::Error for Error<T> {}
