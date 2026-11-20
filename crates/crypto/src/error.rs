use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Length,
    Auth,
    Zero,
    Point,
    Scalar,
    Format,
    Key,
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Length => out.write_str("length outside the range the construction defines"),
            Error::Auth => out.write_str("authentication failed"),
            Error::Zero => out.write_str("shared secret is all zero"),
            Error::Point => out.write_str("point is not a valid public key"),
            Error::Scalar => out.write_str("scalar outside the range the construction defines"),
            Error::Format => out.write_str("encoding is malformed"),
            Error::Key => out.write_str("public key is outside the range the construction defines"),
        }
    }
}

impl core::error::Error for Error {}
