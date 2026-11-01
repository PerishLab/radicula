use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Lookup(which::Error),
    Invalid(&'static str),
    Changed(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(out),
            Self::Lookup(error) => error.fmt(out),
            Self::Invalid(why) => write!(out, "invalid process context: {why}"),
            Self::Changed(path) => write!(out, "resolved tool changed: {}", path.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Lookup(error) => Some(error),
            _ => None,
        }
    }
}
