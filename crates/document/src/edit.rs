use serde_json::Value;

use crate::{Error, Presence};

pub(crate) fn read<'a>(held: &'a Value, parts: &[String]) -> Result<Presence<'a>, Error> {
    let Some((first, rest)) = parts.split_first() else {
        return Ok(Presence::Present(held));
    };
    let next = match held {
        Value::Object(map) => match map.get(first) {
            Some(next) => next,
            None => return Ok(Presence::Missing),
        },
        Value::Array(items) => match seek(items, first)? {
            Some(next) => next,
            None => return Ok(Presence::Missing),
        },
        _ => return Err(Error::Kind),
    };
    read(next, rest)
}

pub(crate) struct Edit<'a> {
    value: &'a mut Value,
}

impl<'a> Edit<'a> {
    pub(crate) fn new(value: &'a mut Value) -> Self {
        Self { value }
    }

    pub(crate) fn set(&mut self, parts: &[String], value: Value) -> Result<(), Error> {
        let Some((first, rest)) = parts.split_first() else {
            *self.value = value;
            return Ok(());
        };
        if rest.is_empty() {
            return self.bind(first, value);
        }
        self.step(first)?.set(rest, value)
    }

    pub(crate) fn remove(&mut self, parts: &[String]) -> Result<(), Error> {
        let Some((first, rest)) = parts.split_first() else {
            return Err(Error::Root);
        };
        if rest.is_empty() {
            return self.erase(first);
        }
        self.step(first)?.remove(rest)
    }

    fn bind(&mut self, part: &str, value: Value) -> Result<(), Error> {
        match self.value {
            Value::Object(map) => {
                map.insert(part.to_string(), value);
                Ok(())
            }
            Value::Array(items) => {
                let at = index(part, items.len())?;
                items[at] = value;
                Ok(())
            }
            _ => Err(Error::Kind),
        }
    }

    fn step(&mut self, part: &str) -> Result<Edit<'_>, Error> {
        let held = &mut *self.value;
        let next = match held {
            Value::Object(map) => map.get_mut(part).ok_or(Error::Missing)?,
            Value::Array(items) => {
                let at = index(part, items.len())?;
                &mut items[at]
            }
            _ => return Err(Error::Kind),
        };
        Ok(Edit::new(next))
    }

    fn erase(&mut self, part: &str) -> Result<(), Error> {
        match self.value {
            Value::Object(map) => {
                if map.remove(part).is_none() {
                    return Err(Error::Missing);
                }
                Ok(())
            }
            Value::Array(items) => {
                let at = index(part, items.len())?;
                items.remove(at);
                Ok(())
            }
            _ => Err(Error::Kind),
        }
    }
}

fn seek<'a>(items: &'a [Value], part: &str) -> Result<Option<&'a Value>, Error> {
    match scan(part)? {
        Some(at) => Ok(items.get(at)),
        None => Ok(None),
    }
}

fn scan(part: &str) -> Result<Option<usize>, Error> {
    let empty = part.is_empty();
    let zero = part.len() > 1 && part.starts_with('0');
    if empty || zero || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Index);
    }
    Ok(part.parse().ok())
}

fn index(part: &str, len: usize) -> Result<usize, Error> {
    match scan(part)? {
        Some(at) if at < len => Ok(at),
        _ => Err(Error::Missing),
    }
}
