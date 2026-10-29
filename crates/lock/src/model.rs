use std::{fmt, io, num::NonZeroUsize, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Policy {
    Limit(NonZeroUsize),
    Readers,
}

impl Policy {
    pub const EXCLUSIVE: Self = Self::Limit(NonZeroUsize::MIN);

    pub fn limit(capacity: usize) -> Result<Self, Error> {
        NonZeroUsize::new(capacity)
            .map(Self::Limit)
            .ok_or(Error::Invalid("capacity must be positive"))
    }
}

#[derive(Clone, Debug)]
pub enum Error {
    Busy,
    Lost,
    Epoch,
    Unconfirmed(Arc<io::Error>),
    Timeout,
    Cancelled,
    Invalid(&'static str),
    Conflict(String),
    Unsupported(&'static str),
    Io(Arc<io::Error>),
    Uncertain(Arc<io::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lost => f.write_str("execution lease is no longer valid"),
            Self::Epoch => {
                f.write_str("Redis server epoch changed; create a new factory after review")
            }
            Self::Unconfirmed(error) => write!(f, "admission outcome is unknown: {error}"),
            Self::Busy => f.write_str("execution gate is busy"),
            Self::Timeout => f.write_str("execution admission timed out"),
            Self::Cancelled => f.write_str("execution admission was cancelled"),
            Self::Invalid(message) | Self::Unsupported(message) => f.write_str(message),
            Self::Conflict(key) => write!(f, "conflicting policy for key {key:?}"),
            Self::Io(error) => write!(f, "coordination I/O failed: {error}"),
            Self::Uncertain(error) => write!(f, "release is unconfirmed: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) | Self::Uncertain(error) | Self::Unconfirmed(error) => {
                Some(error.as_ref())
            }
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

#[derive(Debug)]
pub enum Failure<T> {
    Admission(Error),
    Release { output: T, error: Error },
}

impl<T> fmt::Display for Failure<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) | Self::Release { error, .. } => error.fmt(f),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for Failure<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(error) | Self::Release { error, .. } => Some(error),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Status {
    pub active: usize,
    pub fault: Option<Arc<io::Error>>,
}
