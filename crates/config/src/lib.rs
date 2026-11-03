extern crate self as radicula_config;

pub use radicula_config_macros::Cascade;
pub use serde;
use serde::de::DeserializeOwned;

pub mod layer;
pub use layer::{Cascade, Env, Opaque, Sniff};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Env {
        key: String,
        why: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(out, "cannot read {}: {source}", path.display()),
            Error::Parse { path, source } => {
                write!(out, "cannot parse {}: {source}", path.display())
            }
            Error::Env { key, why } => write!(out, "cannot parse {key}: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Read { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
            Error::Env { .. } => None,
        }
    }
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })
}
