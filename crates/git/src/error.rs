use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Failed {
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed { status, .. } => write!(out, "Git command failed with {status}"),
            Self::Invalid(what) => write!(out, "invalid Git {what}"),
        }
    }
}

impl std::error::Error for Error {}
