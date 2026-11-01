use crate::{Change, Error};
use std::path::PathBuf;
use std::process::Output;

#[derive(Debug)]
pub struct Reply(Output);

impl Reply {
    pub fn new(output: Output) -> Self {
        Self(output)
    }

    pub fn data(self) -> Result<Vec<u8>, Error> {
        match self.0.status.success() {
            true => Ok(self.0.stdout),
            false => Err(self.failed()),
        }
    }

    pub fn ancestry(self) -> Result<bool, Error> {
        match self.0.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(self.failed()),
        }
    }

    pub fn flag(self) -> Result<bool, Error> {
        match line(&self.data()?).as_slice() {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(Error::Invalid("boolean")),
        }
    }

    pub fn object(self) -> Result<String, Error> {
        let bytes = line(&self.data()?);
        let length = matches!(bytes.len(), 40 | 64);
        if !length
            || !bytes
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        {
            return Err(Error::Invalid("object identity"));
        }
        String::from_utf8(bytes).map_err(|_| Error::Invalid("object identity"))
    }

    pub fn path(self) -> Result<PathBuf, Error> {
        decode(line(&self.data()?))
    }

    pub fn changes(self) -> Result<Vec<Change>, Error> {
        crate::paths::parse(&self.data()?)
    }

    fn failed(self) -> Error {
        Error::Failed {
            status: self.0.status,
            stdout: self.0.stdout,
            stderr: self.0.stderr,
        }
    }
}

pub(crate) fn decode(bytes: Vec<u8>) -> Result<PathBuf, Error> {
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(Error::Invalid("path"));
    }
    platform(bytes)
}

#[cfg(unix)]
fn platform(bytes: Vec<u8>) -> Result<PathBuf, Error> {
    use std::os::unix::ffi::OsStringExt;
    Ok(std::ffi::OsString::from_vec(bytes).into())
}

#[cfg(not(unix))]
fn platform(bytes: Vec<u8>) -> Result<PathBuf, Error> {
    String::from_utf8(bytes)
        .map(PathBuf::from)
        .map_err(|_| Error::Invalid("UTF-8 path"))
}

fn line(bytes: &[u8]) -> Vec<u8> {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    #[cfg(windows)]
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    bytes.to_vec()
}
