use std::fmt;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Connect,
    Read,
    Write,
}

#[derive(Debug)]
pub enum Error {
    Resolve(io::Error),
    Empty,
    Connect(io::Error),
    Timeout(Phase),
    Zero,
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(error) => write!(out, "name resolution failed: {error}"),
            Self::Empty => write!(out, "name resolved to no address"),
            Self::Connect(error) => write!(out, "connection failed: {error}"),
            Self::Timeout(phase) => write!(out, "{phase:?} timed out"),
            Self::Zero => write!(out, "a timeout must not be zero"),
            Self::Io(error) => write!(out, "i/o failed: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolve(error) | Self::Connect(error) | Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
