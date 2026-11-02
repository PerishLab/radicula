use super::{Client, Fault, Policy, Request};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn receive(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut wire = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(!line.is_empty());
        wire.push_str(&line);
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    wire.push_str(std::str::from_utf8(&body).unwrap());
    wire
}

fn serve(status: &str, headers: &str, body: &str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let status = status.to_string();
    let headers = headers.to_string();
    let body = body.to_string();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let wire = receive(&stream);
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        wire
    });
    (url, worker)
}

#[test]
fn limits() {
    let (url, worker) = serve("200 OK", "", "PRIVATE_RESPONSE_MARKER");
    let client = Client::new(Policy {
        limit: 4,
        ..Policy::default()
    });
    let response = client
        .send(Request {
            method: "GET",
            url: &url,
            headers: &[],
            body: None,
        })
        .unwrap();
    let Err(error) = response.collect() else {
        panic!("response limit");
    };
    assert!(matches!(error, Fault::Read));
    assert!(!format!("{error:?} {error}").contains("PRIVATE_RESPONSE_MARKER"));
    worker.join().unwrap();
}

#[test]
fn redirects() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let location = format!(
        "Location: http://{}/\r\nRetry-After: 7\r\n",
        target.local_addr().unwrap()
    );
    let (url, worker) = serve("302 Found", &location, "{}");
    let client = Client::new(Policy {
        redirects: 0,
        ..Policy::default()
    });
    let response = client
        .send(Request {
            method: "GET",
            url: &url,
            headers: &[],
            body: None,
        })
        .unwrap();
    assert_eq!(response.status, 302);
    let payload = response.collect().unwrap();
    assert_eq!(payload.headers["retry-after"].as_str(), "7");
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    worker.join().unwrap();
}

#[test]
fn exact() {
    let (url, worker) = serve("200 OK", "", "{}");
    let client = Client::new(Policy::default());
    let body = b"{ \"fixture\" : \"public\" }\n";
    let payload = client
        .send(Request {
            method: "POST",
            url: &url,
            headers: &[("Content-Type", "application/json")],
            body: Some(body),
        })
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(payload.body.as_slice(), b"{}");
    assert!(
        worker
            .join()
            .unwrap()
            .ends_with(std::str::from_utf8(body).unwrap())
    );
}

#[test]
fn connections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        for _ in 0..2 {
            assert!(receive(&stream).starts_with("GET / HTTP/1.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
        }
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let client = Client::new(Policy::default());
    for held in [client.clone(), client] {
        let payload = held
            .send(Request {
                method: "GET",
                url: &url,
                headers: &[],
                body: None,
            })
            .unwrap()
            .collect()
            .unwrap();
        assert_eq!(payload.body.as_slice(), b"{}");
    }
    worker.join().unwrap();
}

#[test]
fn deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/PRIVATE_URL_MARKER",
        listener.local_addr().unwrap()
    );
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let stream = (0..200)
            .find_map(|_| {
                if let Ok((stream, _)) = listener.accept() {
                    return Some(stream);
                }
                thread::sleep(Duration::from_millis(5));
                None
            })
            .expect("request connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        receive(&stream);
        thread::sleep(Duration::from_millis(350));
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let client = Client::new(Policy {
        overall: Some(Duration::from_millis(100)),
        ..Policy::default()
    });
    let Err(error) = client.send(Request {
        method: "POST",
        url: &url,
        headers: &[("Authorization", "PRIVATE_HEADER_MARKER")],
        body: Some(b"PRIVATE_BODY_MARKER"),
    }) else {
        panic!("timeout");
    };
    assert!(matches!(error, Fault::Network));
    let text = format!("{error:?} {error}");
    for marker in [
        "PRIVATE_URL_MARKER",
        "PRIVATE_HEADER_MARKER",
        "PRIVATE_BODY_MARKER",
    ] {
        assert!(!text.contains(marker));
    }
    worker.join().unwrap();
}
