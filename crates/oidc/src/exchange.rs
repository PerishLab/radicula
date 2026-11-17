use crate::{Fault, Secret};
use serde::{Deserialize, Serialize};

pub const GRANT: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
pub const ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(rename = "grant_type")]
    pub grant: String,
    #[serde(rename = "subject_token")]
    pub subject: Secret,
    #[serde(rename = "subject_token_type")]
    pub sort: String,
    pub resource: String,
}

impl Request {
    pub fn new(subject: Secret, resource: String) -> Self {
        Self {
            grant: GRANT.into(),
            subject,
            sort: ACCESS.into(),
            resource,
        }
    }

    pub fn validate(&self) -> Result<(), Fault> {
        if self.grant != GRANT || self.sort != ACCESS {
            return Err(Fault::Grant);
        }
        if self.subject.expose().is_empty() || self.resource.is_empty() {
            return Err(Fault::Format);
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    #[serde(rename = "access_token")]
    pub token: Secret,
    #[serde(rename = "issued_token_type")]
    pub sort: String,
    #[serde(rename = "token_type")]
    pub bearer: String,
    #[serde(rename = "expires_in")]
    pub life: u64,
}

impl Response {
    pub fn validate(&self) -> Result<(), Fault> {
        if self.sort != ACCESS || !self.bearer.eq_ignore_ascii_case("Bearer") {
            return Err(Fault::Grant);
        }
        if self.token.expose().is_empty() || self.life == 0 {
            return Err(Fault::Format);
        }
        Ok(())
    }
}
