use std::{collections::BTreeMap, error::Error, fmt, time::Duration};
use zeroize::Zeroizing;

pub struct Policy {
    pub redirects: u32,
    pub overall: Option<Duration>,
    pub connect: Option<Duration>,
    pub limit: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            redirects: 10,
            overall: None,
            connect: None,
            limit: 10 * 1024 * 1024,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    limit: u64,
}

pub struct Request<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<&'a [u8]>,
}

pub struct Response {
    pub status: u16,
    headers: BTreeMap<String, Zeroizing<String>>,
    body: ureq::Body,
    limit: u64,
}

pub struct Payload {
    pub status: u16,
    pub headers: BTreeMap<String, Zeroizing<String>>,
    pub body: Zeroizing<Vec<u8>>,
}

#[derive(Debug)]
pub enum Fault {
    Method,
    Body(&'static str),
    Network,
    Read,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Method => f.write_str("unsupported HTTP method"),
            Self::Body(method) => write!(f, "HTTP {method} requests do not support a JSON body"),
            Self::Network => f.write_str("request failed"),
            Self::Read => f.write_str("request response is unreadable"),
        }
    }
}

impl Error for Fault {}

impl Client {
    pub fn new(policy: Policy) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .max_redirects(policy.redirects)
                .timeout_global(policy.overall)
                .timeout_connect(policy.connect)
                .build()
                .into(),
            limit: policy.limit,
        }
    }

    pub fn send(&self, request: Request<'_>) -> Result<Response, Fault> {
        if request.body.is_some() {
            match request.method {
                "GET" => return Err(Fault::Body("GET")),
                "DELETE" => return Err(Fault::Body("DELETE")),
                _ => (),
            }
        }
        let response = match request.method {
            "GET" => mark(self.agent.get(request.url), request.headers).call(),
            "DELETE" => mark(self.agent.delete(request.url), request.headers).call(),
            "POST" => mark(self.agent.post(request.url), request.headers)
                .send(request.body.unwrap_or_default()),
            "PATCH" => mark(self.agent.patch(request.url), request.headers)
                .send(request.body.unwrap_or_default()),
            "PUT" => mark(self.agent.put(request.url), request.headers)
                .send(request.body.unwrap_or_default()),
            _ => return Err(Fault::Method),
        }
        .map_err(|_| Fault::Network)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|value| {
                    (
                        name.as_str().to_ascii_lowercase(),
                        Zeroizing::new(value.to_string()),
                    )
                })
            })
            .collect();
        Ok(Response {
            status,
            headers,
            body: response.into_body(),
            limit: self.limit,
        })
    }
}

impl Response {
    pub fn collect(mut self) -> Result<Payload, Fault> {
        let body = Zeroizing::new(
            self.body
                .with_config()
                .limit(self.limit)
                .read_to_vec()
                .map_err(|_| Fault::Read)?,
        );
        Ok(Payload {
            status: self.status,
            headers: self.headers,
            body,
        })
    }
}

fn mark<B>(mut held: ureq::RequestBuilder<B>, headers: &[(&str, &str)]) -> ureq::RequestBuilder<B> {
    for (name, value) in headers {
        held = held.header(*name, *value);
    }
    held
}

#[cfg(test)]
mod tests;
