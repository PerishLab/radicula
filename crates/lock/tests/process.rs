#[path = "process/file.rs"]
mod file;

#[path = "process/scoped.rs"]
mod scoped;

mod support;

use radicula_lock::{Mode, lock, unlock};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use support::Seat;

#[test]
fn worker() {
    support::worker();
}

#[test]
fn competition() {
    let seat = Seat::new();
    for mode in [Mode::Shared, Mode::Exclusive] {
        for backend in ["std", "public"] {
            let held = seat.file();
            match (backend, mode) {
                ("std", Mode::Shared) => File::lock_shared(&held).unwrap(),
                ("std", Mode::Exclusive) => File::lock(&held).unwrap(),
                _ => lock(&held, mode).unwrap(),
            }
            contenders(&seat, mode, backend);
            unlock(&held).unwrap();
            assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
        }
    }
}

fn contenders(seat: &Seat, mode: Mode, backend: &str) {
    for other in ["std", "public"] {
        for wanted in [Mode::Shared, Mode::Exclusive] {
            let expected = if mode == Mode::Shared && wanted == Mode::Shared {
                0
            } else {
                75
            };
            assert_eq!(
                seat.probe(wanted, other),
                expected,
                "{backend} {mode:?} -> {other} {wanted:?}"
            );
        }
    }
}

#[test]
fn blocking() {
    let seat = Seat::new();
    for mode in [Mode::Shared, Mode::Exclusive] {
        let held = seat.file();
        File::lock(&held).unwrap();
        let mut peer = seat.spawn(mode, "public", "block");
        peer.ready();
        peer.pending();
        File::unlock(&held).unwrap();
        assert_eq!(peer.wait(), 0);
    }
}

#[test]
fn lifetime() {
    let seat = Seat::new();
    let held = seat.file();
    lock(&held, Mode::Exclusive).unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    drop(held);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn duplicate() {
    let seat = Seat::new();
    let held = seat.file();
    let duplicate = held.try_clone().unwrap();
    lock(&held, Mode::Exclusive).unwrap();
    drop(held);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    unlock(&duplicate).unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn crash() {
    let seat = Seat::new();
    let _file = seat.file();
    let mut peer = seat.spawn(Mode::Exclusive, "public", "hold");
    peer.ready();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    peer.stop();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn contents() {
    let seat = Seat::new();
    let mut file = seat.file();
    file.write_all(b"journal and owner state\n").unwrap();
    for mode in [Mode::Shared, Mode::Exclusive] {
        lock(&file, mode).unwrap();
        unlock(&file).unwrap();
        assert_eq!(seat.probe(Mode::Exclusive, "public"), 0);
    }
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "journal and owner state\n");
}

#[cfg(unix)]
#[test]
fn replacement() {
    let seat = Seat::new();
    let held = seat.file();
    lock(&held, Mode::Exclusive).unwrap();
    let path = seat.0.join("lease");
    std::fs::rename(&path, seat.0.join("previous")).unwrap();
    let _replacement = seat.file();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    unlock(&held).unwrap();
}

#[cfg(windows)]
#[test]
fn access() {
    let seat = Seat::new();
    let _file = seat.file();
    let file = std::fs::OpenOptions::new()
        .append(true)
        .open(seat.0.join("lease"))
        .unwrap();
    assert!(matches!(
        radicula_lock::attempt(&file, Mode::Exclusive),
        Err(radicula_lock::TryLockError::Error(_))
    ));
    assert!(lock(&file, Mode::Exclusive).is_err());
}
