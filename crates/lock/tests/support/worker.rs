use radicula_lock::{Mode, TryLockError};
use std::fs::{File, OpenOptions};

pub fn run() {
    let Some(path) = std::env::var_os("RADICULA_TEST_PATH") else {
        return;
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mode = match std::env::var("RADICULA_TEST_MODE").unwrap().as_str() {
        "shared" => Mode::Shared,
        "exclusive" => Mode::Exclusive,
        other => panic!("unknown mode {other}"),
    };
    let backend = std::env::var("RADICULA_TEST_BACKEND").unwrap();
    let action = std::env::var("RADICULA_TEST_ACTION").unwrap();
    let marker = std::env::var_os("RADICULA_TEST_MARKER").unwrap();
    match action.as_str() {
        "try" => std::process::exit(attempt(&file, mode, &backend)),
        "block" => {
            std::fs::write(marker, "waiting").unwrap();
            lock(&file, mode, &backend);
        }
        "hold" => {
            lock(&file, mode, &backend);
            std::fs::write(marker, "held").unwrap();
            loop {
                std::thread::park();
            }
        }
        other => panic!("unknown action {other}"),
    }
}

fn lock(file: &File, mode: Mode, backend: &str) {
    match (backend, mode) {
        ("public", _) => radicula_lock::lock(file, mode).unwrap(),
        ("std", Mode::Shared) => File::lock_shared(file).unwrap(),
        ("std", Mode::Exclusive) => File::lock(file).unwrap(),
        _ => panic!("unknown backend {backend}"),
    }
}

fn attempt(file: &File, mode: Mode, backend: &str) -> i32 {
    let result = match (backend, mode) {
        ("public", _) => radicula_lock::attempt(file, mode),
        ("std", Mode::Shared) => File::try_lock_shared(file),
        ("std", Mode::Exclusive) => File::try_lock(file),
        _ => panic!("unknown backend {backend}"),
    };
    match result {
        Ok(()) => 0,
        Err(TryLockError::WouldBlock) => 75,
        Err(TryLockError::Error(error)) => {
            eprintln!("lock I/O error: {error}");
            70
        }
    }
}
