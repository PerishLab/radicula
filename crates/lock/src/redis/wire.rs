use super::Config;
use std::{
    io::{self, BufReader, Read, Write},
    net::TcpStream,
    time::Instant,
};

pub(super) enum Reply {
    Text(Vec<u8>),
    Integer(i64),
    Array(Vec<Reply>),
    Empty,
}

pub(super) struct Wire {
    reader: BufReader<TcpStream>,
    end: Instant,
    bytes: usize,
}

pub(super) struct Failure {
    pub error: io::Error,
    pub submitted: bool,
}

impl Wire {
    pub fn open(config: &Config) -> io::Result<Self> {
        let end = Instant::now()
            .checked_add(config.io)
            .ok_or_else(|| invalid("IO clock overflow"))?;
        let socket = TcpStream::connect_timeout(&config.address, config.io)?;
        socket.set_nodelay(true)?;
        let mut wire = Self {
            reader: BufReader::new(socket),
            end,
            bytes: 0,
        };
        if let Some(credentials) = &config.credentials {
            let mut command = vec![b"AUTH".to_vec()];
            if let Some(user) = &credentials.user {
                command.push(user.as_bytes().to_vec());
            }
            command.push(credentials.password.as_bytes().to_vec());
            wire.ok(&command)?;
        }
        if config.database != 0 {
            wire.ok(&[b"SELECT".to_vec(), config.database.to_string().into_bytes()])?;
        }
        Ok(wire)
    }

    fn ok(&mut self, command: &[Vec<u8>]) -> io::Result<()> {
        match self.exchange(command)? {
            Reply::Text(value) if value == b"OK" => Ok(()),
            _ => Err(invalid("Redis initialization was not acknowledged")),
        }
    }

    pub fn call(config: &Config, command: &[Vec<u8>]) -> Result<Reply, Failure> {
        let mut wire = Self::open(config).map_err(|error| Failure {
            error,
            submitted: false,
        })?;
        wire.exchange(command).map_err(|error| Failure {
            error,
            submitted: true,
        })
    }

    fn exchange(&mut self, command: &[Vec<u8>]) -> io::Result<Reply> {
        let mut bytes = format!("*{}\r\n", command.len()).into_bytes();
        for part in command {
            write!(bytes, "${}\r\n", part.len())?;
            bytes.extend_from_slice(part);
            bytes.extend_from_slice(b"\r\n");
        }
        if bytes.len() > 65536 {
            return Err(invalid("Redis command exceeds64KiB"));
        }
        self.limit()?;
        self.reader.get_mut().write_all(&bytes)?;
        self.read(0)
    }

    fn limit(&mut self) -> io::Result<()> {
        let left = self.end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Redis IO budget elapsed",
            ));
        }
        self.reader.get_mut().set_read_timeout(Some(left))?;
        self.reader.get_mut().set_write_timeout(Some(left))
    }

    fn line(&mut self) -> io::Result<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            self.limit()?;
            let mut byte = [0];
            self.reader.read_exact(&mut byte)?;
            self.bytes += 1;
            if self.bytes > 65536 {
                return Err(invalid("Redis reply exceeds64KiB"));
            }
            line.push(byte[0]);
            if line.ends_with(b"\r\n") {
                line.truncate(line.len() - 2);
                return Ok(line);
            }
        }
    }

    fn read(&mut self, depth: usize) -> io::Result<Reply> {
        if depth > 4 {
            return Err(invalid("Redis reply nesting exceeds4"));
        }
        let line = self.line()?;
        let Some((&kind, tail)) = line.split_first() else {
            return Err(invalid("empty Redis frame"));
        };
        match kind {
            b'+' => Ok(Reply::Text(tail.to_vec())),
            b'-' => Err(invalid("Redis command was refused")),
            b':' => number(tail).map(Reply::Integer),
            b'$' => self.bulk(number(tail)?),
            b'*' => self.array(number(tail)?, depth),
            _ => Err(invalid("unsupported Redis reply frame")),
        }
    }

    fn bulk(&mut self, size: i64) -> io::Result<Reply> {
        if size == -1 {
            return Ok(Reply::Empty);
        }
        let size = usize::try_from(size).map_err(|_| invalid("invalid Redis bulk length"))?;
        if size > 65536 || self.bytes + size + 2 > 65536 {
            return Err(invalid("Redis bulk reply exceeds budget"));
        }
        let mut body = vec![0; size + 2];
        let mut offset = 0;
        while offset < body.len() {
            self.limit()?;
            let read = self.reader.read(&mut body[offset..])?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Redis reply ended",
                ));
            }
            offset += read;
        }
        self.bytes += body.len();
        if !body.ends_with(b"\r\n") {
            return Err(invalid("invalid Redis bulk delimiter"));
        }
        body.truncate(size);
        Ok(Reply::Text(body))
    }

    fn array(&mut self, size: i64, depth: usize) -> io::Result<Reply> {
        let size = usize::try_from(size).map_err(|_| invalid("invalid Redis array length"))?;
        if size > 256 {
            return Err(invalid("Redis array exceeds256 items"));
        }
        (0..size)
            .map(|_| self.read(depth + 1))
            .collect::<io::Result<Vec<_>>>()
            .map(Reply::Array)
    }
}

fn number(bytes: &[u8]) -> io::Result<i64> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| invalid("invalid Redis integer"))
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
