mod client;
mod config;
mod gate;
mod key;
mod lease;
mod reply;
mod scripts;
mod wire;

pub use config::{Config, Credentials};
pub use gate::{Gate, Permit};
pub use lease::Fence;

use crate::{Error, Policy};
use client::Client;
use key::Key;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Factory {
    client: Arc<Client>,
    keys: Arc<Mutex<BTreeMap<String, Arc<Key>>>>,
}

impl Factory {
    pub fn connect(config: Config) -> Result<Self, Error> {
        Ok(Self {
            client: Client::connect(config)?,
            keys: Arc::default(),
        })
    }

    pub fn epoch(&self) -> &str {
        &self.client.epoch
    }

    pub fn gate(&self, key: impl Into<String>, policy: Policy) -> Result<Gate, Error> {
        let name = key.into();
        let mut keys = self.keys.lock().unwrap_or_else(|error| error.into_inner());
        let key = match keys.get(&name) {
            Some(key) if key.policy != policy => return Err(Error::Conflict(name)),
            Some(key) => key.clone(),
            None => {
                let key = Arc::new(Key::new(&self.client.config.namespace, &name, policy)?);
                keys.insert(name, key.clone());
                key
            }
        };
        Ok(Gate::new(self.client.clone(), key))
    }
}

#[cfg(test)]
mod tests;
