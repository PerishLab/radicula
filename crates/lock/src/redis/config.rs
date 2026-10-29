use crate::Error;
use std::{fmt, net::SocketAddr, time::Duration};

#[derive(Clone)]
pub struct Credentials {
    pub(crate) user: Option<String>,
    pub(crate) password: String,
}

impl Credentials {
    pub fn password(password: impl Into<String>) -> Self {
        Self {
            user: None,
            password: password.into(),
        }
    }

    pub fn user(user: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            user: Some(user.into()),
            password: password.into(),
        }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials([redacted])")
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) address: SocketAddr,
    pub(crate) namespace: String,
    pub(crate) database: u32,
    pub(crate) credentials: Option<Credentials>,
    pub(crate) lease: Duration,
    pub(crate) renewal: Option<Duration>,
    pub(crate) io: Duration,
    pub(crate) slots: usize,
}

impl Config {
    pub fn new(address: SocketAddr, namespace: impl Into<String>) -> Self {
        Self {
            address,
            namespace: namespace.into(),
            database: 0,
            credentials: None,
            lease: Duration::from_secs(30),
            renewal: Some(Duration::from_secs(5)),
            io: Duration::from_millis(100),
            slots: 32,
        }
    }

    pub fn database(mut self, database: u32) -> Self {
        self.database = database;
        self
    }

    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn lease(mut self, duration: Duration, renewal: Option<Duration>) -> Self {
        self.lease = duration;
        self.renewal = renewal;
        self
    }

    pub fn io(mut self, duration: Duration) -> Self {
        self.io = duration;
        self
    }

    pub fn slots(mut self, slots: usize) -> Self {
        self.slots = slots;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.namespace.is_empty() || self.namespace.len() > 128 {
            return Err(Error::Invalid("Redis namespace must contain1..128 bytes"));
        }
        if !(1..=1024).contains(&self.slots) || self.io.is_zero() {
            return Err(Error::Invalid(
                "Redis local lease budget must be1..1024 and IO budget positive",
            ));
        }
        let minimum = self
            .io
            .checked_mul((self.slots as u32 + 1) * 4)
            .ok_or(Error::Invalid("Redis IO budget exceeds clock range"))?;
        if self.lease < minimum || self.lease.as_millis() > (1_u128 << 52) {
            return Err(Error::Invalid(
                "Redis lease must exceed four complete local IO cycles",
            ));
        }
        if self
            .renewal
            .is_some_and(|interval| interval.is_zero() || interval > self.lease / 4)
        {
            return Err(Error::Invalid(
                "Redis renewal interval must be positive and at most one quarter lease",
            ));
        }
        if std::time::Instant::now().checked_add(self.lease).is_none() {
            return Err(Error::Invalid("Redis lease exceeds monotonic clock range"));
        }
        if self.io.as_millis() == 0 || self.renewal.is_some_and(|value| value.as_millis() == 0) {
            return Err(Error::Invalid(
                "Redis time budgets use at least one millisecond",
            ));
        }
        Ok(())
    }
}
