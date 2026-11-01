#![cfg(feature = "tokio")]

use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error, Limits, Runner, State, Streams};
use std::io::{Read, Write};
use std::process::Command;
use std::time::Duration;

fn command(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "worker", "--nocapture"])
        .env("RADICULA_PROCESS_WORKER", mode);
    command
}

fn runner(time: u64, bytes: u64) -> Runner {
    Runner::new(Limits {
        time: Duration::from_millis(time),
        output: bytes,
    })
    .unwrap()
}

#[test]
fn worker() {
    let Ok(mode) = std::env::var("RADICULA_PROCESS_WORKER") else {
        return;
    };
    match mode.as_str() {
        "block" => {
            if let Some(path) = std::env::var_os("RADICULA_PROCESS_PID") {
                std::fs::write(path, std::process::id().to_string()).unwrap();
            }
            std::thread::sleep(Duration::from_secs(30));
        }
        "failure" => {
            std::io::stdout().write_all(b"owned output").unwrap();
            std::io::stderr().write_all(b"owned diagnostic").unwrap();
            std::process::exit(7);
        }
        "noisy" => {
            std::io::stderr().write_all(&vec![b'e'; 2_097_152]).unwrap();
        }
        "duplex" => {
            std::io::stderr().write_all(&vec![b'e'; 2_097_152]).unwrap();
            std::io::stdout().write_all(&vec![b'o'; 2_097_152]).unwrap();
        }
        _ => {}
    }
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    std::io::stdout().write_all(&input).unwrap();
    std::io::stdout().write_all(b"done").unwrap();
    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}

#[test]
fn blocking() {
    let output = runner(3000, 8192)
        .blocking(command("echo"), b"payload".to_vec(), Cancel::new())
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.ends_with(b"payloaddone"));
    let failed = runner(3000, 8192)
        .blocking(command("failure"), Vec::new(), Cancel::new())
        .unwrap();
    assert_eq!(failed.status.code(), Some(7));
    assert!(failed.stdout.ends_with(b"owned output"));
    assert_eq!(failed.stderr, b"owned diagnostic");
}

#[tokio::test(flavor = "current_thread")]
async fn duplex() {
    let mut session =
        runner(5000, 8_388_608).capture(command("duplex"), vec![b'i'; 262_144], Cancel::new());
    let observer = session.observer();
    let output = session.output().await.unwrap();
    assert!(output.status.success());
    assert!(output.stdout.ends_with(b"done"));
    assert_eq!(output.stderr.len(), 2_097_152);
    assert!(output.stdout.len() > 2_359_296);
    assert_eq!(observer.state(), State::Settled);
    assert!(matches!(session.wait().await, Err(Error::Consumed)));
}

#[tokio::test(flavor = "current_thread")]
async fn budget() {
    let mut session = runner(3000, 1024).capture(command("duplex"), Vec::new(), Cancel::new());
    let observer = session.observer();
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Output));
    assert!(failure.child.is_none());
    assert!(failure.cleanup.is_none());
    assert!(failure.status.is_some());
    assert!(
        failure.streams.stdout.as_ref().unwrap().len()
            + failure.streams.stderr.as_ref().unwrap().len()
            <= 1024
    );
    assert_eq!(observer.state(), State::Settled);
}

#[tokio::test(flavor = "current_thread")]
async fn timeout() {
    let mut session = runner(100, 8192).capture(command("block"), Vec::new(), Cancel::new());
    let observer = session.observer();
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Time));
    assert!(failure.child.is_none());
    assert!(failure.status.is_some());
    assert_eq!(observer.state(), State::Settled);
}

#[tokio::test(flavor = "current_thread")]
async fn cancel() {
    let mut session = runner(5000, 8192).capture(command("block"), Vec::new(), Cancel::new());
    let observer = session.observer();
    assert!(
        tokio::time::timeout(Duration::from_millis(20), session.output())
            .await
            .is_err()
    );
    assert_eq!(observer.state(), State::Running);
    session.cancel();
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Cancelled));
    assert!(failure.child.is_none());
    assert_eq!(observer.state(), State::Settled);
}

#[tokio::test(flavor = "current_thread")]
async fn dropped() {
    let session = runner(5000, 8192).capture(command("block"), Vec::new(), Cancel::new());
    let observer = session.observer();
    tokio::task::yield_now().await;
    drop(session);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), observer.finish())
            .await
            .unwrap(),
        State::Settled
    );
}

#[tokio::test(flavor = "current_thread")]
async fn precancelled() {
    let cancel = Cancel::new();
    cancel.cancel();
    let mut session = runner(5000, 8192).capture(command("block"), Vec::new(), cancel);
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Cancelled));
    assert!(failure.status.is_none());
    assert!(failure.child.is_none());
}

#[test]
fn abandoned() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let session = {
        let _entered = runtime.enter();
        runner(5000, 8192).capture(command("block"), Vec::new(), Cancel::new())
    };
    let observer = session.observer();
    drop(runtime);
    assert_eq!(observer.state(), State::Uncertain);
    drop(session);
}

#[tokio::test(flavor = "current_thread")]
async fn nested() {
    assert!(matches!(
        runner(5000, 8192).blocking(command("echo"), Vec::new(), Cancel::new()),
        Err(Error::Active)
    ));
    assert!(
        Runner::new(Limits {
            time: Duration::ZERO,
            output: 1
        })
        .is_err()
    );
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "current_thread")]
async fn reaped() {
    let seat = tempfile::tempdir().unwrap();
    let marker = seat.path().join("pid");
    let mut command = command("block");
    command.env("RADICULA_PROCESS_PID", &marker);
    let mut session = runner(3000, 8192).capture(command, Vec::new(), Cancel::new());
    let observer = session.observer();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let pid = std::fs::read_to_string(&marker).unwrap();
    session.cancel();
    let Error::Operation(failure) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Cancelled));
    assert_eq!(observer.state(), State::Settled);
    assert!(!std::path::Path::new("/proc").join(pid).exists());
}

#[test]
fn discarded() {
    let completed = runner(3000, 1024)
        .stream(command("noisy"), Streams::stdout(Vec::new()), Cancel::new())
        .unwrap();
    assert!(completed.status.success());
    assert!(completed.streams.input.is_none());
    assert!(completed.streams.stderr.is_none());
    assert!(completed.streams.stdout.unwrap().ends_with(b"done"));
    let Error::Operation(failure) = runner(3000, 1024)
        .blocking(command("noisy"), Vec::new(), Cancel::new())
        .unwrap_err()
    else {
        panic!("operation failure")
    };
    assert!(matches!(failure.cause, Cause::Output));
    assert!(failure.status.is_some());
    assert!(failure.child.is_none());
}
