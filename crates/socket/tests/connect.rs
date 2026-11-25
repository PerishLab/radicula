#[cfg(unix)]
use radicula_socket::Phase;
use radicula_socket::{Error, Stream};
use std::io::ErrorKind;
use std::net::TcpListener;
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(5);

fn listener() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

#[test]
fn accepted() {
    let (listener, port) = listener();
    Stream::connect("127.0.0.1", port, LIMIT).unwrap();
    listener.accept().unwrap();
}

#[test]
fn named() {
    let (listener, port) = listener();
    Stream::connect("localhost", port, LIMIT).unwrap();
    listener.accept().unwrap();
}

#[test]
fn refused() {
    let (listener, port) = listener();
    drop(listener);
    match Stream::connect("127.0.0.1", port, LIMIT) {
        Err(Error::Connect(error)) => assert_eq!(error.kind(), ErrorKind::ConnectionRefused),
        other => panic!("{other:?}"),
    }
}

#[test]
fn zero() {
    let (_listener, port) = listener();
    let outcome = Stream::connect("127.0.0.1", port, Duration::ZERO);
    assert!(matches!(outcome, Err(Error::Zero)));
}

#[test]
fn unresolved() {
    let outcome = Stream::connect("nul\0host", 80, LIMIT);
    assert!(matches!(outcome, Err(Error::Resolve(_))));
}

#[cfg(target_os = "linux")]
#[test]
fn expired() {
    let (_listener, port) = listener();
    let mut held = Vec::new();
    for _ in 0..70000 {
        match Stream::connect("127.0.0.1", port, Duration::from_millis(200)) {
            Ok(stream) => held.push(stream),
            Err(Error::Timeout(Phase::Connect)) => return,
            Err(error) => panic!("{error:?}"),
        }
    }
    panic!("backlog never filled");
}
