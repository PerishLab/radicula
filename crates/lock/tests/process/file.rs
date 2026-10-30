use super::support::Seat;
use radicula_lock::{Cancellation, Error, Factory, Failure, Mode, Policy, Wait};
use std::{
    future::Future,
    io::{Read, Seek, SeekFrom, Write},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Mutex,
    task::{Context, Poll, Waker},
    time::Duration,
};

#[cfg(unix)]
fn identity(file: &std::fs::File) -> usize {
    use std::os::fd::AsRawFd;
    file.as_raw_fd() as usize
}
#[cfg(windows)]
fn identity(file: &std::fs::File) -> usize {
    use std::os::windows::io::AsRawHandle;
    file.as_raw_handle() as usize
}

#[test]
fn exits() {
    let seat = Seat::new();
    let file = seat.file();
    let gate = Factory::file(&file, Policy::EXCLUSIVE).unwrap();
    assert!(std::ptr::eq(&file, gate.file()));
    gate.guard(|| {
        assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
        (&file).write_all(b"spool entry").unwrap();
        assert_eq!((&file).stream_position().unwrap(), 11);
        Err::<(), _>("business")
    })
    .map(|output| assert_eq!(output, Err("business")))
    .unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = gate.guard(|| panic!("business"));
        }))
        .is_err()
    );
    assert_eq!(gate.status().active, 0);
    assert!(gate.status().fault.is_none());
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    (&file).seek(SeekFrom::Start(0)).unwrap();
    let mut contents = String::new();
    (&file).read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "spool entry");
}

#[test]
fn spool() {
    let seat = Seat::new();
    let spool = Mutex::new(Factory::file(seat.file(), Policy::EXCLUSIVE).unwrap());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let gate = spool.lock().unwrap();
            let _ = gate.guard(|| {
                gate.file().write_all(b"record").unwrap();
                assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
                panic!("append unwinds");
            });
        }))
        .is_err()
    );
    assert!(spool.is_poisoned());
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    let gate = spool.lock().unwrap_or_else(|error| error.into_inner());
    assert!(gate.status().fault.is_none());
    gate.guard(|| {
        gate.file().rewind().unwrap();
        let mut contents = String::new();
        gate.file().read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "record");
    })
    .unwrap();
}

#[test]
fn session() {
    struct Store {
        permit: radicula_lock::file::Permit,
    }
    let seat = Seat::new();
    let file = seat.file();
    let original = identity(&file);
    let gate = Factory::file(file, Policy::EXCLUSIVE).unwrap();
    let store = Store {
        permit: gate.acquire(Wait::immediate()).unwrap(),
    };
    drop(gate);
    assert_eq!(identity(store.permit.file()), original);
    store.permit.file().write_all(b"journal").unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    drop(store);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn readers() {
    let seat = Seat::new();
    let file = seat.file();
    let writer = Factory::file(&file, Policy::Readers).unwrap();
    let readers = writer.shared().unwrap();
    let first = readers.acquire(Wait::immediate()).unwrap();
    let second = readers.clone().acquire(Wait::immediate()).unwrap();
    assert!(matches!(
        writer.acquire(Wait::immediate()),
        Err(Error::Busy)
    ));
    assert_eq!(seat.probe(Mode::Shared, "std"), 0);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    first.release().unwrap();
    assert_eq!(readers.status().active, 1);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    second.release().unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[test]
fn waiting() {
    let seat = Seat::new();
    let file = seat.file();
    assert!(matches!(
        Factory::file(&file, Policy::limit(2).unwrap()),
        Err(Error::Unsupported(_))
    ));
    let gate = Factory::file(&file, Policy::EXCLUSIVE).unwrap();
    assert!(matches!(gate.shared(), Err(Error::Unsupported(_))));
    let permit = gate.acquire(Wait::immediate()).unwrap();
    assert!(matches!(
        gate.with(Wait::immediate(), || panic!("must not enter")),
        Err(Failure::Admission(Error::Busy))
    ));
    assert!(matches!(
        gate.acquire(Wait::timeout(Duration::from_millis(15))),
        Err(Error::Timeout)
    ));
    let token = Cancellation::default();
    token.cancel();
    assert!(matches!(
        gate.acquire(Wait::forever().cancel(token)),
        Err(Error::Cancelled)
    ));
    drop(permit);
    let mut peer = seat.spawn(Mode::Exclusive, "std", "hold");
    peer.ready();
    assert!(matches!(
        gate.acquire(Wait::timeout(Duration::from_millis(15))),
        Err(Error::Timeout)
    ));
    peer.stop();
    gate.with(Wait::timeout(Duration::from_secs(1)), || ())
        .unwrap();
}

#[test]
fn asynchronous() {
    let seat = Seat::new();
    let file = seat.file();
    let gate = Factory::file(&file, Policy::EXCLUSIVE)
        .unwrap()
        .wait(Wait::immediate());
    let mut context = Context::from_waker(Waker::noop());
    let mut task = Box::pin(gate.asynchronous().guard(|| async {
        assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
        std::future::pending::<()>().await;
    }));
    assert!(task.as_mut().poll(&mut context).is_pending());
    drop(task);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    let mut task = Box::pin(gate.asynchronous().guard(|| async { panic!("poll panic") }));
    assert!(catch_unwind(AssertUnwindSafe(|| task.as_mut().poll(&mut context))).is_err());
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    drop(task);
    let mut task = Box::pin(
        gate.asynchronous()
            .with(Wait::forever(), || async { panic!("must not enter") }),
    );
    assert!(matches!(
        task.as_mut().poll(&mut context),
        Poll::Ready(Err(Failure::Admission(Error::Unsupported(_))))
    ));
}

#[test]
fn cancellation() {
    let seat = Seat::new();
    let file = seat.file();
    let gate = Factory::file(&file, Policy::EXCLUSIVE).unwrap();
    struct Check<'a>(&'a Seat);
    impl Drop for Check<'_> {
        fn drop(&mut self) {
            assert_eq!(self.0.probe(Mode::Exclusive, "std"), 75);
        }
    }
    let permit = gate.acquire(Wait::immediate()).unwrap();
    let execution = permit.during(|| {
        let check = Check(&seat);
        async move {
            let _keep = check;
            std::future::pending::<()>().await;
        }
    });
    drop(execution);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[cfg(feature = "macros")]
#[test]
fn methods() {
    use std::{cell::Cell, rc::Rc};
    #[radicula_lock::guard(gate = gate, error = |_| Err("coordination"))]
    fn write(gate: &radicula_lock::file::Gate<&std::fs::File>) -> Result<(), &'static str> {
        gate.file().write_all(b"method").map_err(|_| "write")
    }
    #[radicula_lock::guard(gate = gate, error = |_| Err("coordination"))]
    async fn local(
        gate: &radicula_lock::file::Gate<Rc<std::fs::File>>,
        value: Rc<Cell<usize>>,
    ) -> Result<(), &'static str> {
        value.set(value.get() + 1);
        Ok(())
    }
    let seat = Seat::new();
    let file = seat.file();
    write(&Factory::file(&file, Policy::EXCLUSIVE).unwrap()).unwrap();
    let gate = Factory::file(Rc::new(file), Policy::EXCLUSIVE)
        .unwrap()
        .wait(Wait::immediate());
    let value = Rc::new(Cell::new(0));
    let mut task = Box::pin(local(&gate, value.clone()));
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(
        task.as_mut().poll(&mut context),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(value.get(), 1);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}
