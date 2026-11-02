use std::{collections::BTreeMap, error::Error, fmt};

use anyhow::{Context, Result};
use zeroize::Zeroizing;
pub mod wire;
use serde_json::Value;

pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

pub struct Fault {
    response: Response,
}

pub struct Request<'a> {
    verb: &'a str,
    url: &'a str,
    headers: BTreeMap<String, String>,
    body: Option<&'a Value>,
}

impl<'a> Request<'a> {
    pub fn new(verb: &'a str, url: &'a str) -> Self {
        Self {
            verb,
            url,
            headers: BTreeMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn json(mut self, body: &'a Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn send(self) -> Result<Response> {
        self.using(&wire::Client::new(wire::Policy::default()))
    }
}

impl Fault {
    pub fn response(&self) -> &Response {
        &self.response
    }

    pub fn status(&self) -> u16 {
        self.response.status
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.response
            .headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn body(&self) -> &Value {
        &self.response.body
    }
}

impl fmt::Debug for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fault")
            .field("status", &self.response.status)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.response.body.get("message").and_then(Value::as_str) {
            Some(message) => f.write_str(message),
            None => write!(
                f,
                "HTTP request failed with status {}",
                self.response.status
            ),
        }
    }
}

impl Error for Fault {}

pub fn send(verb: &str, url: &str, auth: Option<&str>, body: Option<&Value>) -> Result<Value> {
    let response = request(verb, url, auth, body)?;
    if (200..300).contains(&response.status) {
        return Ok(response.body);
    }
    Err(Fault { response }.into())
}

pub fn raw(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<(u16, Value)> {
    let response = request(verb, url, auth, body)?;
    Ok((response.status, response.body))
}

pub fn request(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&Value>,
) -> Result<Response> {
    let mut request = Request::new(verb, url);
    if let Some(auth) = auth {
        request = request.header("Authorization", auth);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    request.send()
}

impl Request<'_> {
    pub fn using(&self, client: &wire::Client) -> Result<Response> {
        let encoded = self
            .body
            .map(serde_json::to_vec)
            .transpose()
            .context("unable to encode JSON")?
            .map(Zeroizing::new);
        let mut headers: Vec<_> = self
            .headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        if matches!(self.verb, "POST" | "PATCH" | "PUT") {
            headers.push(("Content-Type", "application/json"));
        }
        let payload = client
            .send(wire::Request {
                method: self.verb,
                url: self.url,
                headers: &headers,
                body: encoded.as_ref().map(|value| value.as_slice()),
            })?
            .collect()?;
        let text = String::from_utf8_lossy(&payload.body);
        let body = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text.into_owned()))
        };
        Ok(Response {
            status: payload.status,
            headers: payload
                .headers
                .into_iter()
                .map(|(name, value)| (name, value.to_string()))
                .collect(),
            body,
        })
    }
}
