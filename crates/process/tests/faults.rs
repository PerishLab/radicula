#![cfg(feature = "tokio")]

#[path = "support/sink.rs"]
mod sink;

use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error, Limits, Runner, State, Streams};
use std::io::Write;
use std::process::Command;
use std::time::Duration;

#[test]
fn worker() {
    if std::env::var_os("RADICULA_PROCESS_FAULT").is_none() {
        return;
    }
    std::io::stdout().write_all(b"trigger sink").unwrap();
    std::thread::sleep(Duration::from_secs(30));
}

fn session(
    panic: bool,
) -> radicula_process::asynchronous::Session<Streams<tokio::io::Empty, sink::Sink, Vec<u8>>> {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "worker", "--nocapture"])
        .env("RADICULA_PROCESS_FAULT", "1");
    let runner = Runner::new(Limits {
        time: Duration::from_secs(3),
        output: 8192,
    })
    .unwrap();
    runner.start(
        command,
        Streams::new(
            tokio::io::empty(),
            sink::Sink { panic, writes: 0 },
            Vec::new(),
        ),
        Cancel::new(),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn failure() {
    let mut session = session(false);
    let observer = session.observer();
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Io(_)));
    assert!(failure.streams.stdout.writes > 0);
    assert!(failure.child.is_none());
    assert!(failure.cleanup.is_none());
    assert!(failure.status.is_some());
    assert_eq!(observer.state(), State::Settled);
    assert!(!format!("{failure:?}").contains("trigger sink"));
}

#[tokio::test(flavor = "current_thread")]
async fn panic() {
    let mut session = session(true);
    let observer = session.observer();
    assert!(matches!(
        session.wait().await,
        Err(Error::Interrupted { .. })
    ));
    assert_eq!(observer.state(), State::Uncertain);
}
