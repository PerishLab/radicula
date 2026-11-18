use serde_json::{Number, Value};

use crate::{Error, Fault, Operation, Path, Presence, edit};

const LIMIT: u64 = 9_007_199_254_740_991;
const CEIL: f64 = 9_007_199_254_740_992.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    value: Value,
}

impl Document {
    pub fn new(value: Value) -> Result<Self, Error> {
        check(&value)?;
        Ok(Self { value })
    }

    pub fn parse(text: &str) -> Result<Self, Error> {
        let value: Value = serde_json::from_str(text).map_err(|_| Error::Syntax)?;
        Self::new(value)
    }

    pub fn root(&self) -> &Value {
        &self.value
    }

    pub fn text(&self) -> String {
        self.value.to_string()
    }

    pub fn take(self) -> Value {
        self.value
    }

    pub fn read(&self, path: &Path) -> Result<Presence<'_>, Error> {
        edit::read(&self.value, path.parts())
    }

    pub fn set(&mut self, path: &Path, value: Value) -> Result<(), Error> {
        check(&value)?;
        edit::Edit::new(&mut self.value).set(path.parts(), value)
    }

    pub fn remove(&mut self, path: &Path) -> Result<(), Error> {
        edit::Edit::new(&mut self.value).remove(path.parts())
    }

    pub fn apply(&mut self, operations: &[Operation]) -> Result<(), Fault> {
        let mut trial = self.value.clone();
        for (index, operation) in operations.iter().enumerate() {
            if let Err(error) = perform(&mut trial, operation) {
                return Err(Fault { index, error });
            }
        }
        self.value = trial;
        Ok(())
    }
}

fn perform(target: &mut Value, operation: &Operation) -> Result<(), Error> {
    match operation {
        Operation::Set { path, value } => {
            check(value)?;
            edit::Edit::new(target).set(path.parts(), value.clone())
        }
        Operation::Remove { path } => edit::Edit::new(target).remove(path.parts()),
    }
}

fn check(value: &Value) -> Result<(), Error> {
    match value {
        Value::Number(held) => test(held),
        Value::Array(items) => items.iter().try_for_each(check),
        Value::Object(map) => map.values().try_for_each(check),
        _ => Ok(()),
    }
}

fn test(held: &Number) -> Result<(), Error> {
    if let Some(value) = held.as_i64() {
        return range(value.unsigned_abs());
    }
    if let Some(value) = held.as_u64() {
        return range(value);
    }
    let value = held.as_f64().ok_or(Error::Number)?;
    if !value.is_finite() {
        return Err(Error::Number);
    }
    if value.fract() != 0.0 || value.abs() < CEIL {
        return Ok(());
    }
    Err(Error::Number)
}

fn range(value: u64) -> Result<(), Error> {
    if value <= LIMIT {
        return Ok(());
    }
    Err(Error::Number)
}
