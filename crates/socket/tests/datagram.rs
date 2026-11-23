use radicula_socket::{Datagram, Error, Mark};
use std::net::SocketAddr;
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(5);
const SHORT: Duration = Duration::from_millis(150);

struct Pair {
    left: Datagram,
    right: Datagram,
    from: SocketAddr,
    to: SocketAddr,
}

fn pair() -> Pair {
    let left = Datagram::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let right = Datagram::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let from = left.local().unwrap();
    let to = right.local().unwrap();
    Pair {
        left,
        right,
        from,
        to,
    }
}

#[test]
fn trips() {
    let pair = pair();
    assert_eq!(pair.left.send(b"ping", pair.to, Some(LIMIT)).unwrap(), 4);
    let mut buf = [0u8; 8];
    let (count, peer) = pair.right.recv(&mut buf, Some(LIMIT)).unwrap();
    assert_eq!(&buf[..count], b"ping");
    assert_eq!(peer, pair.from);
    assert_eq!(pair.right.send(b"pong", pair.from, None).unwrap(), 4);
    let (count, peer) = pair.left.recv(&mut buf, None).unwrap();
    assert_eq!(&buf[..count], b"pong");
    assert_eq!(peer, pair.to);
}

#[test]
fn zero() {
    let pair = pair();
    let mut buf = [0u8; 1];
    assert!(matches!(
        pair.left.send(b"x", pair.to, Some(Duration::ZERO)),
        Err(Error::Zero)
    ));
    assert!(matches!(
        pair.left.recv(&mut buf, Some(Duration::ZERO)),
        Err(Error::Zero)
    ));
}

#[test]
fn expired() {
    let pair = pair();
    let mut buf = [0u8; 1];
    let start = std::time::Instant::now();
    let outcome = pair.left.recv(&mut buf, Some(SHORT));
    assert!(matches!(outcome, Err(Error::Timeout(_))), "{outcome:?}");
    assert!(start.elapsed() >= SHORT);
}

#[cfg(target_os = "linux")]
#[test]
fn coded() {
    let pair = pair();
    for mark in [Mark::Plain, Mark::Ect0, Mark::Ect1, Mark::Ce] {
        pair.left.mark(mark).unwrap();
        assert_eq!(pair.left.coded().unwrap(), mark, "{mark:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn marks() {
    let pair = pair();
    pair.right.observe(true).unwrap();
    for mark in [Mark::Plain, Mark::Ect0, Mark::Ect1, Mark::Ce] {
        pair.left.mark(mark).unwrap();
        assert_eq!(pair.left.send(b"ec", pair.to, Some(LIMIT)).unwrap(), 2);
        let mut buf = [0u8; 8];
        let (count, peer, got) = pair.right.marked(&mut buf, Some(LIMIT)).unwrap();
        assert_eq!(&buf[..count], b"ec");
        assert_eq!(peer, pair.from);
        assert_eq!(got, mark, "{mark:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn segments() {
    let pair = pair();
    let size = 64usize;
    pair.left.segment(size).unwrap();
    let payload = vec![0xab; size * 3];
    assert_eq!(
        pair.left.send(&payload, pair.to, Some(LIMIT)).unwrap(),
        payload.len()
    );
    let mut got = Vec::new();
    for _ in 0..3 {
        let mut buf = [0u8; 256];
        let (count, _) = pair.right.recv(&mut buf, Some(LIMIT)).unwrap();
        assert_eq!(count, size);
        got.extend_from_slice(&buf[..count]);
    }
    assert_eq!(got, payload);
}

#[test]
fn display() {
    assert_eq!(
        Error::Unsupported.to_string(),
        "unsupported on this platform"
    );
}
