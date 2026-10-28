use super::support::Seat;
use radicula_lock::{Error, Factory, Mode, Policy, Wait};
use std::{io::Write, panic::AssertUnwindSafe, time::Duration};

fn factory(seat: &Seat) -> Factory {
    let path = seat.0.join("lease");
    drop(seat.file());
    Factory::files(move |_| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
    })
}

#[test]
fn exits() {
    let seat = Seat::new();
    let factory = factory(&seat);
    let gate = factory.gate("work", Policy::EXCLUSIVE).unwrap();
    assert_eq!(
        gate.guard(|| {
            assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
            Err::<(), _>("business")
        })
        .unwrap(),
        Err("business")
    );
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = gate.guard(|| {
            assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
            panic!("business panic");
        });
    }));
    assert!(panic.is_err());
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn readers() {
    let seat = Seat::new();
    let factory = factory(&seat);
    let gate = factory.gate("ledger", Policy::Readers).unwrap();
    let readers = gate.shared().unwrap();
    let first = readers.acquire(Wait::immediate()).unwrap();
    let second = readers.acquire(Wait::immediate()).unwrap();
    assert_eq!(seat.probe(Mode::Shared, "std"), 0);
    assert_eq!(seat.probe(Mode::Exclusive, "public"), 75);
    drop(first);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    drop(second);
    assert_eq!(seat.probe(Mode::Exclusive, "public"), 0);
    assert!(gate.status().fault.is_none());
}

#[test]
fn contention() {
    let seat = Seat::new();
    let factory = factory(&seat);
    let gate = factory.gate("work", Policy::EXCLUSIVE).unwrap();
    let mut peer = seat.spawn(Mode::Exclusive, "std", "hold");
    peer.ready();
    assert!(matches!(gate.acquire(Wait::immediate()), Err(Error::Busy)));
    assert!(matches!(
        gate.acquire(Wait::timeout(Duration::from_millis(15))),
        Err(Error::Timeout)
    ));
    peer.stop();
    assert!(gate.acquire(Wait::timeout(Duration::from_secs(2))).is_ok());
    assert!(matches!(
        factory.gate("slots", Policy::limit(2).unwrap()),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn contents() {
    let seat = Seat::new();
    let mut file = seat.file();
    file.write_all(b"counter=42").unwrap();
    let duplicate = file.try_clone().unwrap();
    let factory = Factory::files(move |_| duplicate.try_clone());
    let gate = factory.gate("spool", Policy::EXCLUSIVE).unwrap();
    gate.guard(|| assert_eq!(seat.probe(Mode::Exclusive, "std"), 75))
        .unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    assert_eq!(std::fs::read(seat.0.join("lease")).unwrap(), b"counter=42");
    drop(gate);
    drop(factory);
    drop(file);
}
