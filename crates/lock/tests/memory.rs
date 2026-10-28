use radicula_lock::{Cancellation, Error, Factory, Gate, Policy, Wait};
use std::{
    panic::AssertUnwindSafe,
    sync::{Arc, Barrier},
    time::Duration,
};

fn gate() -> Gate {
    Factory::memory().gate("work", Policy::EXCLUSIVE).unwrap()
}

#[test]
fn exits() {
    let gate = gate();
    assert_eq!(gate.guard(|| 42).unwrap(), 42);
    assert_eq!(
        gate.guard(|| Err::<(), _>("business")).unwrap(),
        Err("business")
    );
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = gate.guard(|| panic!("business panic"));
    }));
    assert!(result.is_err());
    assert_eq!(gate.status().active, 0);
    assert!(gate.acquire(Wait::immediate()).is_ok());
}

#[test]
fn capacity() {
    let factory = Factory::memory();
    let policy = Policy::limit(2).unwrap();
    let gate = factory.gate("work", policy).unwrap();
    let same = factory.clone().gate("work", policy).unwrap();
    let first = gate.acquire(Wait::immediate()).unwrap();
    let second = same.acquire(Wait::immediate()).unwrap();
    assert!(matches!(gate.acquire(Wait::immediate()), Err(Error::Busy)));
    assert!(matches!(
        factory.gate("work", Policy::EXCLUSIVE),
        Err(Error::Conflict(_))
    ));
    assert!(
        factory
            .gate("other", Policy::EXCLUSIVE)
            .unwrap()
            .acquire(Wait::immediate())
            .is_ok()
    );
    drop(first);
    assert!(same.acquire(Wait::immediate()).is_ok());
    drop(second);
    assert_eq!(gate.status().active, 0);
    assert!(Policy::limit(0).is_err());
}

#[test]
fn readers() {
    let gate = Factory::memory().gate("ledger", Policy::Readers).unwrap();
    let readers = gate.shared().unwrap();
    let first = readers.acquire(Wait::immediate()).unwrap();
    let second = readers.acquire(Wait::immediate()).unwrap();
    assert!(matches!(gate.acquire(Wait::immediate()), Err(Error::Busy)));
    drop(first);
    assert!(matches!(gate.acquire(Wait::immediate()), Err(Error::Busy)));
    drop(second);
    let writer = gate.acquire(Wait::immediate()).unwrap();
    assert!(matches!(
        readers.acquire(Wait::immediate()),
        Err(Error::Busy)
    ));
    drop(writer);
    assert!(readers.acquire(Wait::immediate()).is_ok());
}

#[test]
fn waiting() {
    let gate = gate();
    let permit = gate.acquire(Wait::immediate()).unwrap();
    assert!(matches!(
        gate.acquire(Wait::timeout(Duration::from_millis(15))),
        Err(Error::Timeout)
    ));
    let cancel = Cancellation::default();
    let waiter = gate.clone();
    let token = cancel.clone();
    let thread = std::thread::spawn(move || waiter.acquire(Wait::forever().cancel(token)));
    cancel.cancel();
    assert!(matches!(thread.join().unwrap(), Err(Error::Cancelled)));
    drop(permit);
    assert!(gate.acquire(Wait::timeout(Duration::ZERO)).is_ok());
}

#[test]
fn contention() {
    let gate = gate();
    let barrier = Arc::new(Barrier::new(8));
    let inside = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let threads = (0..8)
        .map(|_| {
            let gate = gate.clone();
            let barrier = barrier.clone();
            let inside = inside.clone();
            std::thread::spawn(move || {
                barrier.wait();
                exercise(gate, inside);
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(gate.status().active, 0);
}

fn exercise(gate: Gate, inside: Arc<std::sync::atomic::AtomicUsize>) {
    for _ in 0..100 {
        gate.guard(|| {
            assert_eq!(inside.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
            std::thread::yield_now();
            assert_eq!(inside.fetch_sub(1, std::sync::atomic::Ordering::SeqCst), 1);
        })
        .unwrap();
    }
}

struct Check(Gate);

impl Drop for Check {
    fn drop(&mut self) {
        assert!(matches!(
            self.0.acquire(Wait::immediate()),
            Err(Error::Busy)
        ));
    }
}

#[test]
fn lifetime() {
    let gate = gate();
    let held = gate
        .hold(Wait::immediate(), || Check(gate.clone()))
        .unwrap();
    assert_eq!(gate.status().active, 1);
    drop(held);
    assert!(gate.acquire(Wait::immediate()).is_ok());
}

struct Panic(Gate);

impl Drop for Panic {
    fn drop(&mut self) {
        assert!(matches!(
            self.0.acquire(Wait::immediate()),
            Err(Error::Busy)
        ));
        panic!("value destructor panic");
    }
}

#[test]
fn destruction() {
    let gate = gate();
    let held = gate
        .hold(Wait::immediate(), || Panic(gate.clone()))
        .unwrap();
    assert!(std::panic::catch_unwind(AssertUnwindSafe(|| drop(held))).is_err());
    assert!(gate.acquire(Wait::immediate()).is_ok());
}
