use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub path: PathBuf,
    pub digest: String,
}

impl Tool {
    pub fn at(path: PathBuf) -> Result<Self, Error> {
        let digest = fingerprint(&path)?;
        Ok(Self { path, digest })
    }

    pub fn verify(&self) -> Result<(), Error> {
        if Self::at(self.path.clone())?.digest != self.digest {
            return Err(Error::Changed(self.path.clone()));
        }
        Ok(())
    }

    pub fn content(&self) -> Result<PathBuf, Error> {
        self.path.canonicalize().map_err(Error::Io)
    }
}

fn fingerprint(path: &Path) -> Result<String, Error> {
    let mut file = std::fs::File::open(path).map_err(Error::Io)?;
    let mut digest = Sha256::new();
    let mut bytes = [0; 8192];
    loop {
        let count = file.read(&mut bytes).map_err(Error::Io)?;
        if count == 0 {
            break;
        }
        digest.update(&bytes[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
