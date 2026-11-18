use radicula_socket::{Error, Phase, Shutdown, Stream};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(5);
const SHORT: Duration = Duration::from_millis(150);

fn pair() -> (Stream, TcpStream) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let client = Stream::connect("127.0.0.1", port, LIMIT).unwrap();
    let (peer, _) = listener.accept().unwrap();
    (client, peer)
}

#[test]
fn exchange() {
    let (mut client, mut peer) = pair();
    assert_eq!(client.write(b"ping", Some(LIMIT)).unwrap(), 4);
    let mut buf = [0u8; 4];
    peer.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"ping");
    peer.write_all(b"pong").unwrap();
    let mut back = [0u8; 8];
    let count = client.read(&mut back, Some(LIMIT)).unwrap();
    assert_eq!(&back[..count], b"pong");
}

#[test]
fn unlimited() {
    let (mut client, mut peer) = pair();
    peer.write_all(b"x").unwrap();
    let mut buf = [0u8; 1];
    assert_eq!(client.read(&mut buf, None).unwrap(), 1);
    assert_eq!(client.write(b"y", None).unwrap(), 1);
}

#[test]
fn closed() {
    let (mut client, mut peer) = pair();
    peer.write_all(b"last").unwrap();
    drop(peer);
    let mut buf = [0u8; 16];
    assert_eq!(client.read(&mut buf, Some(LIMIT)).unwrap(), 4);
    assert_eq!(client.read(&mut buf, Some(LIMIT)).unwrap(), 0);
}

#[test]
fn silent() {
    let (mut client, _peer) = pair();
    let mut buf = [0u8; 16];
    let start = Instant::now();
    let outcome = client.read(&mut buf, Some(SHORT));
    assert!(matches!(outcome, Err(Error::Timeout(Phase::Read))));
    assert!(start.elapsed() >= SHORT - Duration::from_millis(20));
}

#[test]
fn stalled() {
    let (mut client, _peer) = pair();
    let chunk = [0u8; 65536];
    for _ in 0..100000 {
        match client.write(&chunk, Some(SHORT)) {
            Ok(_) => continue,
            Err(Error::Timeout(Phase::Write)) => return,
            Err(error) => panic!("{error:?}"),
        }
    }
    panic!("buffers never filled");
}

#[test]
fn halved() {
    let (mut client, mut peer) = pair();
    client.write(b"request", Some(LIMIT)).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut seen = Vec::new();
    peer.read_to_end(&mut seen).unwrap();
    assert_eq!(seen, b"request");
    peer.write_all(b"response").unwrap();
    let mut buf = [0u8; 16];
    let count = client.read(&mut buf, Some(LIMIT)).unwrap();
    assert_eq!(&buf[..count], b"response");
}

#[test]
fn broken() {
    let (mut client, peer) = pair();
    drop(peer);
    for _ in 0..50 {
        match client.write(b"data", Some(LIMIT)) {
            Ok(_) => std::thread::sleep(Duration::from_millis(20)),
            Err(Error::Io(_)) => return,
            Err(error) => panic!("{error:?}"),
        }
    }
    panic!("write never failed");
}

#[test]
fn zero() {
    let (mut client, _peer) = pair();
    let mut buf = [0u8; 1];
    assert!(matches!(
        client.read(&mut buf, Some(Duration::ZERO)),
        Err(Error::Zero)
    ));
    assert!(matches!(
        client.write(b"x", Some(Duration::ZERO)),
        Err(Error::Zero)
    ));
}
