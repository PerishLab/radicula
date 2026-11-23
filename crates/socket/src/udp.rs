use crate::error::{Error, Phase};
use crate::sys;
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Plain,
    Ect1,
    Ect0,
    Ce,
}

impl Mark {
    fn tos(self) -> u8 {
        match self {
            Self::Plain => 0,
            Self::Ect1 => 1,
            Self::Ect0 => 2,
            Self::Ce => 3,
        }
    }

    pub(crate) fn parse(tos: u8) -> Self {
        match tos & 3 {
            1 => Self::Ect1,
            2 => Self::Ect0,
            3 => Self::Ce,
            _ => Self::Plain,
        }
    }
}

#[derive(Debug)]
pub struct Datagram {
    inner: UdpSocket,
}

impl Datagram {
    pub fn bind(address: SocketAddr) -> Result<Self, Error> {
        let inner = UdpSocket::bind(address).map_err(Error::Io)?;
        Ok(Self { inner })
    }

    pub fn local(&self) -> Result<SocketAddr, Error> {
        self.inner.local_addr().map_err(Error::Io)
    }

    pub fn send(
        &self,
        buf: &[u8],
        target: SocketAddr,
        timeout: Option<Duration>,
    ) -> Result<usize, Error> {
        self.inner
            .set_write_timeout(bound(timeout)?)
            .map_err(Error::Io)?;
        loop {
            match self.inner.send_to(buf, target) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                other => return settle(Phase::Write, other),
            }
        }
    }

    pub fn recv(
        &self,
        buf: &mut [u8],
        timeout: Option<Duration>,
    ) -> Result<(usize, SocketAddr), Error> {
        self.inner
            .set_read_timeout(bound(timeout)?)
            .map_err(Error::Io)?;
        loop {
            match self.inner.recv_from(buf) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Ok((count, from)) => return Ok((count, from)),
                Err(error) if expired(&error) => return Err(Error::Timeout(Phase::Read)),
                Err(error) => return Err(Error::Io(error)),
            }
        }
    }

    pub fn mark(&self, mark: Mark) -> Result<(), Error> {
        set(self, sys::IP, sys::TOS, i32::from(mark.tos()))
    }

    pub fn coded(&self) -> Result<Mark, Error> {
        get(self, sys::IP, sys::TOS).map(|tos| Mark::parse(tos as u8))
    }

    pub fn observe(&self, flag: bool) -> Result<(), Error> {
        set(self, sys::IP, sys::RECV, i32::from(flag))
    }

    pub fn segment(&self, size: usize) -> Result<(), Error> {
        if size > u16::MAX as usize {
            return Err(Error::Io(io::Error::from(ErrorKind::InvalidInput)));
        }
        match set(self, sys::UDP, sys::SEGMENT, size as i32) {
            Err(Error::Io(error)) if denied(&error) => Err(Error::Unsupported),
            other => other,
        }
    }

    pub fn marked(
        &self,
        buf: &mut [u8],
        timeout: Option<Duration>,
    ) -> Result<(usize, SocketAddr, Mark), Error> {
        self.inner
            .set_read_timeout(bound(timeout)?)
            .map_err(Error::Io)?;
        loop {
            match take(self, buf) {
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Ok(found) => return Ok(found),
                Err(error) if expired(&error) => return Err(Error::Timeout(Phase::Read)),
                Err(error) if error.kind() == ErrorKind::Unsupported => {
                    return Err(Error::Unsupported);
                }
                Err(error) => return Err(Error::Io(error)),
            }
        }
    }
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
        Err(error) if expired(&error) => Err(Error::Timeout(phase)),
        Err(error) => Err(Error::Io(error)),
    }
}

fn denied(error: &io::Error) -> bool {
    sys::denied(error)
}

fn set(socket: &Datagram, level: i32, name: i32, value: i32) -> Result<(), Error> {
    sys::set(&socket.inner, level, name, value)
}

fn get(socket: &Datagram, level: i32, name: i32) -> Result<i32, Error> {
    sys::get(&socket.inner, level, name)
}

fn take(socket: &Datagram, buf: &mut [u8]) -> io::Result<(usize, SocketAddr, Mark)> {
    sys::take(&socket.inner, buf)
}
