use crate::{Error, Policy, Status};
use std::{
    io,
    sync::{Arc, Condvar, Mutex},
};

pub(super) struct Key {
    pub name: String,
    pub policy: Policy,
    pub keys: Vec<Vec<u8>>,
    pub state: Mutex<Status>,
    pub changed: Condvar,
}

impl Key {
    pub fn new(namespace: &str, name: &str, policy: Policy) -> Result<Self, Error> {
        if name.is_empty() || name.len() > 4096 {
            return Err(Error::Invalid("Redis key must contain1..4096 bytes"));
        }
        if matches!(policy, Policy::Limit(n) if n.get()>i32::MAX as usize) {
            return Err(Error::Unsupported(
                "Redis capacity exceeds supported integer range",
            ));
        }
        let prefix = format!("radicula:lock:v1:{}:{}", hex(namespace), hex(name));
        Ok(Self {
            name: name.into(),
            policy,
            keys: ["policy", "leases", "modes", "sequence"]
                .iter()
                .map(|suffix| format!("{prefix}:{suffix}").into_bytes())
                .collect(),
            state: Mutex::new(Status {
                active: 0,
                fault: None,
            }),
            changed: Condvar::new(),
        })
    }

    pub fn admits(&self) -> Result<(), Error> {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match &state.fault {
            Some(error) => Err(Error::Uncertain(error.clone())),
            None => Ok(()),
        }
    }

    pub fn policy(&self) -> String {
        match self.policy {
            Policy::Limit(n) => n.to_string(),
            Policy::Readers => "readers".into(),
        }
    }

    pub fn status(&self) -> Status {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        Status {
            active: state.active,
            fault: state.fault.clone(),
        }
    }

    pub fn fault(&self, message: &'static str) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state
            .fault
            .get_or_insert_with(|| Arc::new(io::Error::other(message)));
        self.changed.notify_all();
    }

    pub fn acquired(&self) {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .active += 1;
    }

    pub fn finished(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.active -= 1;
        drop(state);
        self.changed.notify_all();
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
