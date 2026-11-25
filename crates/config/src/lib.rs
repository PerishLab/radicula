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
    Section {
        path: PathBuf,
        section: String,
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
            Error::Section { path, section, why } => write!(
                out,
                "cannot load section {section} from {}: {why}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Read { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
            Error::Env { .. } | Error::Section { .. } => None,
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

pub fn section<T: DeserializeOwned>(path: &Path, key: &str) -> Result<Option<T>, Error> {
    let source = Source { path };
    let Some(text) = source.text()? else {
        return Ok(None);
    };
    let table = source.root(&text)?;
    let Some(table) = source.walk(key, table)? else {
        return Ok(None);
    };
    source.decode(key, table).map(Some)
}

struct Source<'a> {
    path: &'a Path,
}

impl Source<'_> {
    fn text(&self) -> Result<Option<String>, Error> {
        match std::fs::read_to_string(self.path) {
            Ok(text) => Ok(Some(text)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Read {
                path: self.path.to_path_buf(),
                source,
            }),
        }
    }

    fn root(&self, text: &str) -> Result<toml::Table, Error> {
        toml::from_str(text).map_err(|source| Error::Parse {
            path: self.path.to_path_buf(),
            source,
        })
    }

    fn fail(&self, key: &str, why: impl Into<String>) -> Error {
        Error::Section {
            path: self.path.to_path_buf(),
            section: key.to_string(),
            why: why.into(),
        }
    }

    fn parts<'a>(&self, key: &'a str) -> Result<Vec<&'a str>, Error> {
        if key.is_empty() {
            return Err(self.fail(key, "section path is empty"));
        }
        let parts: Vec<&str> = key.split('.').collect();
        if parts.iter().any(|part| part.is_empty()) {
            return Err(self.fail(key, "section path contains an empty segment"));
        }
        Ok(parts)
    }

    fn walk(&self, key: &str, mut table: toml::Table) -> Result<Option<toml::Table>, Error> {
        for part in self.parts(key)? {
            match table.remove(part) {
                None => return Ok(None),
                Some(toml::Value::Table(next)) => table = next,
                Some(_) => {
                    return Err(self.fail(key, format!("key `{part}` exists but is not a table")));
                }
            }
        }
        Ok(Some(table))
    }

    fn decode<T: DeserializeOwned>(&self, key: &str, table: toml::Table) -> Result<T, Error> {
        table
            .try_into()
            .map_err(|source| self.fail(key, source.to_string()))
    }
}
