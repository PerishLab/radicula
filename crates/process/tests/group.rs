#![cfg(all(feature = "tokio", target_os = "linux"))]

use radicula_process::Cancel;
use radicula_process::asynchronous::{Cause, Error, Limits, Runner, Streams};
use std::process::{Command, Stdio};
use std::time::Duration;

fn runner(time: u64) -> Runner {
    Runner::group(Limits {
        time: Duration::from_millis(time),
        output: 8192,
    })
    .unwrap()
}

fn command(mode: &str, path: &std::path::Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "worker", "--nocapture"])
        .env("RADICULA_GROUP_MODE", mode)
        .env("RADICULA_GROUP_PID", path);
    command
}

#[test]
fn worker() {
    let Ok(mode) = std::env::var("RADICULA_GROUP_MODE") else {
        return;
    };
    if mode == "moved" {
        assert_eq!(
            unsafe { libc::setpgid(0, libc::getpgid(libc::getppid())) },
            0
        );
        std::process::exit(7);
    }
    let path = std::env::var_os("RADICULA_GROUP_PID").unwrap();
    if mode == "child" {
        std::fs::write(path, std::process::id().to_string()).unwrap();
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    let mut child = command("child", std::path::Path::new(&path))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while !std::path::Path::new(&path).exists() {
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    match mode.as_str() {
        "exit" => std::process::exit(7),
        "noisy" => {
            use std::io::Write;
            std::io::stdout().write_all(&vec![b'x'; 65536]).unwrap();
            std::thread::sleep(Duration::from_secs(30));
        }
        "signal" => unsafe {
            libc::raise(libc::SIGKILL);
        },
        _ => std::thread::sleep(Duration::from_secs(30)),
    }
    child.wait().unwrap();
}

fn stopped(path: &std::path::Path) {
    let pid = std::fs::read_to_string(path).unwrap();
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
    if let Ok(stat) = stat {
        let (_, fields) = stat.rsplit_once(") ").unwrap();
        assert!(matches!(fields.split_whitespace().next(), Some("Z" | "X")));
    }
}

#[test]
fn completion() {
    use std::os::unix::process::ExitStatusExt;
    for mode in ["exit", "signal"] {
        let seat = tempfile::tempdir().unwrap();
        let path = seat.path().join("pid");
        let output = runner(5000)
            .blocking(command(mode, &path), Vec::new(), Cancel::new())
            .unwrap();
        if mode == "exit" {
            assert_eq!(output.status.code(), Some(7));
        } else {
            assert_eq!(output.status.signal(), Some(libc::SIGKILL));
        }
        stopped(&path);
    }
}

#[test]
fn discarded() {
    let seat = tempfile::tempdir().unwrap();
    let path = seat.path().join("pid");
    let output = runner(5000)
        .stream(
            command("exit", &path),
            Streams::stdout(Vec::new()),
            Cancel::new(),
        )
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    stopped(&path);
}

#[test]
fn timeout() {
    let seat = tempfile::tempdir().unwrap();
    let path = seat.path().join("pid");
    let Error::Operation(failed) = runner(500)
        .blocking(command("block", &path), Vec::new(), Cancel::new())
        .unwrap_err()
    else {
        panic!("operation failure")
    };
    assert!(matches!(failed.cause, Cause::Time));
    assert!(failed.cleanup.is_none());
    assert!(failed.child.is_none());
    assert!(failed.status.is_some());
    stopped(&path);
}

#[tokio::test(flavor = "current_thread")]
async fn isolation() {
    let seat = tempfile::tempdir().unwrap();
    let first = seat.path().join("first");
    let second = seat.path().join("second");
    let mut left = runner(5000).capture(command("block", &first), Vec::new(), Cancel::new());
    let mut right = runner(5000).capture(command("block", &second), Vec::new(), Cancel::new());
    tokio::time::timeout(Duration::from_secs(3), async {
        while !first.exists() || !second.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    left.cancel();
    let Error::Operation(failed) = left.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failed.cause, Cause::Cancelled));
    assert!(failed.cleanup.is_none());
    stopped(&first);
    let pid = std::fs::read_to_string(&second)
        .unwrap()
        .parse::<i32>()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    right.cancel();
    let Error::Operation(failed) = right.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(failed.cleanup.is_none());
    stopped(&second);
}

#[test]
fn overflow() {
    let seat = tempfile::tempdir().unwrap();
    let path = seat.path().join("pid");
    let Error::Operation(failed) = runner(5000)
        .blocking(command("noisy", &path), Vec::new(), Cancel::new())
        .unwrap_err()
    else {
        panic!("operation failure")
    };
    assert!(matches!(failed.cause, Cause::Output));
    assert!(failed.cleanup.is_none());
    assert!(failed.child.is_none());
    stopped(&path);
}

#[tokio::test(flavor = "current_thread")]
async fn uncertainty() {
    let seat = tempfile::tempdir().unwrap();
    let mut session =
        runner(5000).capture(command("moved", seat.path()), Vec::new(), Cancel::new());
    let observer = session.observer();
    let Error::Operation(mut failed) = session.wait().await.unwrap_err() else {
        panic!("operation failure")
    };
    assert!(matches!(failed.cause, Cause::Io(_)));
    assert!(failed.cleanup.is_some());
    assert_eq!(
        observer.state(),
        radicula_process::asynchronous::State::Uncertain
    );
    assert_eq!(
        failed.child.as_mut().unwrap().wait().await.unwrap().code(),
        Some(7)
    );
}
