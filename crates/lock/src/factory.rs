use crate::{Error, Gate, Mode, Policy, Wait, state::Entry};
use std::{
    collections::BTreeMap,
    fs::File,
    io,
    sync::{Arc, Mutex},
};

type Open = dyn Fn(&str) -> io::Result<File> + Send + Sync;

#[derive(Clone)]
pub struct Factory {
    entries: Arc<Mutex<BTreeMap<String, Arc<Entry>>>>,
    open: Option<Arc<Open>>,
}

impl Factory {
    #[cfg(feature = "redis")]
    pub fn redis(config: crate::redis::Config) -> Result<crate::redis::Factory, Error> {
        crate::redis::Factory::connect(config)
    }

    pub fn memory() -> Self {
        Self {
            entries: Arc::default(),
            open: None,
        }
    }

    pub fn files(open: impl Fn(&str) -> io::Result<File> + Send + Sync + 'static) -> Self {
        Self {
            entries: Arc::default(),
            open: Some(Arc::new(open)),
        }
    }

    pub fn gate(&self, key: impl Into<String>, policy: Policy) -> Result<Gate, Error> {
        let key = key.into();
        if key.is_empty() {
            return Err(Error::Invalid("coordination key must not be empty"));
        }
        if let Some(gate) = self.existing(&key, policy)? {
            return Ok(gate);
        }
        self.validate(policy)?;
        let file = self.open.as_ref().map(|open| open(&key)).transpose()?;
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let entry = entries
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Entry::new(policy, file)));
        Self::bind(&key, entry, policy)
    }

    fn existing(&self, key: &str, policy: Policy) -> Result<Option<Gate>, Error> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        entries
            .get(key)
            .map(|entry| Self::bind(key, entry, policy))
            .transpose()
    }

    fn bind(key: &str, entry: &Arc<Entry>, policy: Policy) -> Result<Gate, Error> {
        if entry.policy != policy {
            return Err(Error::Conflict(key.to_owned()));
        }
        Ok(Gate {
            entry: entry.clone(),
            mode: Mode::Exclusive,
            wait: Wait::forever(),
        })
    }

    fn validate(&self, policy: Policy) -> Result<(), Error> {
        if self.open.is_some() && matches!(policy, Policy::Limit(n) if n.get() > 1) {
            return Err(Error::Unsupported(
                "whole-file carrier supports exclusive or read/write policy, not capacity greater than one",
            ));
        }
        Ok(())
    }
}
