use radicula_socket::{Error, Phase};
use std::error::Error as _;
use std::io;

#[test]
fn source() {
    let wrapped = Error::Io(io::Error::from(io::ErrorKind::BrokenPipe));
    assert!(wrapped.source().is_some());
    assert!(Error::Timeout(Phase::Read).source().is_none());
    assert!(Error::Zero.source().is_none());
    assert!(Error::Empty.source().is_none());
}

#[test]
fn display() {
    assert_eq!(Error::Timeout(Phase::Write).to_string(), "Write timed out");
    assert!(
        Error::Connect(io::Error::from(io::ErrorKind::ConnectionRefused))
            .to_string()
            .starts_with("connection failed")
    );
}
