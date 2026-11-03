use crate::Error;
use serde::de::DeserializeOwned;
use std::path::PathBuf;

pub trait Env: Sized {
    fn read(value: &str) -> Result<Self, String>;
}

impl Env for String {
    fn read(value: &str) -> Result<Self, String> {
        Ok(value.to_string())
    }
}

impl Env for PathBuf {
    fn read(value: &str) -> Result<Self, String> {
        Ok(PathBuf::from(value))
    }
}

impl Env for bool {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("neither true nor false".to_string()),
        }
    }
}

macro_rules! parsed {
    ($($ty:ty),*) => {
        $(impl Env for $ty {
            fn read(value: &str) -> Result<Self, String> {
                value.parse().map_err(|error: std::num::ParseIntError| error.to_string())
            }
        })*
    };
}

parsed!(u16, u32, u64, usize, i32, i64);

impl<T: Env> Env for Option<T> {
    fn read(value: &str) -> Result<Self, String> {
        T::read(value).map(Some)
    }
}

pub struct Sniff<T>(pub std::marker::PhantomData<T>);

impl<T: Env> Sniff<T> {
    pub fn take(&self, held: Option<String>, key: &str) -> Result<Option<T>, Error> {
        let Some(value) = held else {
            return Ok(None);
        };
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        T::read(value).map(Some).map_err(|why| Error::Env {
            key: key.to_string(),
            why,
        })
    }
}

pub trait Opaque<T> {
    fn take(&self, held: Option<String>, key: &str) -> Result<Option<T>, Error>;
}

impl<T> Opaque<T> for &Sniff<T> {
    fn take(&self, _: Option<String>, _: &str) -> Result<Option<T>, Error> {
        Ok(None)
    }
}

pub trait Cascade: Default {
    type Partial: Default + DeserializeOwned;
    fn lookup(prefix: &str, get: &dyn Fn(&str) -> Option<String>) -> Result<Self::Partial, Error>;
    fn merge(self, over: Self::Partial) -> Self;
}
