use crate::Fault;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    pub(super) fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(value) => value == expected,
            Self::Many(values) => values.iter().any(|value| value == expected),
        }
    }

    fn valid(&self) -> bool {
        match self {
            Self::One(value) => !value.is_empty(),
            Self::Many(values) => !values.is_empty() && values.iter().all(|v| !v.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub aud: Audience,
    pub exp: u64,
    pub iat: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<u64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Claims {
    pub(super) fn shape(&self) -> Result<(), Fault> {
        if self.iss.is_empty() || self.sub.is_empty() || !self.aud.valid() {
            return Err(Fault::Format);
        }
        if self.exp <= self.iat || self.nbf.is_some_and(|n| n >= self.exp) {
            return Err(Fault::Format);
        }
        if ["iss", "sub", "aud", "exp", "iat", "nbf"]
            .iter()
            .any(|key| self.extra.contains_key(*key))
        {
            return Err(Fault::Claim);
        }
        Ok(())
    }
}
