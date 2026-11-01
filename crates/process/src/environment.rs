use std::ffi::OsString;
use std::fmt;
use std::process::Command;

#[derive(Clone)]
pub struct Environment {
    values: Vec<(OsString, OsString)>,
}

impl Environment {
    pub fn new(values: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn values(&self) -> &[(OsString, OsString)] {
        &self.values
    }

    pub fn apply(&self, command: &mut Command) {
        command.env_clear().envs(self.values.iter().cloned());
    }
}

impl fmt::Debug for Environment {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("Environment")
            .field("entries", &self.values.len())
            .finish_non_exhaustive()
    }
}
