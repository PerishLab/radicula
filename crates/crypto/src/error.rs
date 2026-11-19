use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Length,
    Auth,
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Length => out.write_str("length outside the range the construction defines"),
            Error::Auth => out.write_str("authentication failed"),
        }
    }
}

impl core::error::Error for Error {}
