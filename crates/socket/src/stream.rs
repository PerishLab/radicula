use crate::error::{Error, Phase};
use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Stream {
    inner: TcpStream,
}

impl Stream {
    pub fn connect(host: &str, port: u16, timeout: Duration) -> Result<Self, Error> {
        if timeout.is_zero() {
            return Err(Error::Zero);
        }
        let deadline = Instant::now().checked_add(timeout);
        let addresses = (host, port).to_socket_addrs().map_err(Error::Resolve)?;
        let mut failure = None;
        for address in addresses {
            let left = deadline.map_or(timeout, |at| at.saturating_duration_since(Instant::now()));
            if left.is_zero() {
                return Err(Error::Timeout(Phase::Connect));
            }
            match TcpStream::connect_timeout(&address, left) {
                Ok(inner) => return Ok(Self { inner }),
                Err(error) => failure = Some(error),
            }
        }
        match failure {
            None => Err(Error::Empty),
            Some(error) if Self::expired(&error) => Err(Error::Timeout(Phase::Connect)),
            Some(error) => Err(Error::Connect(error)),
        }
    }

    pub fn read(&mut self, buf: &mut [u8], timeout: Option<Duration>) -> Result<usize, Error> {
        self.inner
            .set_read_timeout(Self::bound(timeout)?)
            .map_err(Error::Io)?;
        loop {
            match self.inner.read(buf) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                other => return Self::settle(Phase::Read, other),
            }
        }
    }

    pub fn write(&mut self, data: &[u8], timeout: Option<Duration>) -> Result<usize, Error> {
        self.inner
            .set_write_timeout(Self::bound(timeout)?)
            .map_err(Error::Io)?;
        loop {
            match self.inner.write(data) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                other => return Self::settle(Phase::Write, other),
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        self.inner.shutdown(how).map_err(Error::Io)
    }

    fn bound(timeout: Option<Duration>) -> Result<Option<Duration>, Error> {
        match timeout {
            Some(limit) if limit.is_zero() => Err(Error::Zero),
            other => Ok(other),
        }
    }

    fn expired(error: &io::Error) -> bool {
        matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
    }

    fn settle(phase: Phase, outcome: io::Result<usize>) -> Result<usize, Error> {
        match outcome {
            Ok(count) => Ok(count),
            Err(error) if Self::expired(&error) => Err(Error::Timeout(phase)),
            Err(error) => Err(Error::Io(error)),
        }
    }
}
