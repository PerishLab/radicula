use super::support::{Fault, Log, Probe, context, current, poll};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::Poll;

#[test]
fn unopened() {
    let log = Log::default();
    let held = context("inner").carry(Probe::new(log.clone(), 1, Fault::None));
    context("outer").run(|| {
        drop(held);
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(*log.lock().unwrap(), vec![("drop", Some("inner"))]);
}

#[test]
fn pending() {
    let log = Log::default();
    let mut held = Box::pin(context("inner").carry(Probe::new(log.clone(), 1, Fault::None)));
    assert_eq!(poll(held.as_mut()), Poll::Pending);
    context("outer").run(|| {
        drop(held);
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(
        *log.lock().unwrap(),
        vec![("poll", Some("inner")), ("drop", Some("inner"))]
    );
}

#[test]
fn polling() {
    let log = Log::default();
    let mut held = Box::pin(context("inner").carry(Probe::new(log.clone(), 0, Fault::Poll)));
    context("outer").run(|| {
        assert!(catch_unwind(AssertUnwindSafe(|| poll(held.as_mut()))).is_err());
        assert_eq!(current(), Some("outer"));
        drop(held);
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(
        *log.lock().unwrap(),
        vec![("poll", Some("inner")), ("drop", Some("inner"))]
    );
}

#[test]
fn destruction() {
    let log = Log::default();
    let held = Box::pin(context("inner").carry(Probe::new(log.clone(), 0, Fault::Drop)));
    context("outer").run(|| {
        assert!(catch_unwind(AssertUnwindSafe(|| drop(held))).is_err());
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(*log.lock().unwrap(), vec![("drop", Some("inner"))]);
}

#[test]
fn completing() {
    let log = Log::default();
    let mut held = Box::pin(context("inner").carry(Probe::new(log.clone(), 0, Fault::Drop)));
    context("outer").run(|| {
        assert!(catch_unwind(AssertUnwindSafe(|| poll(held.as_mut()))).is_err());
        assert_eq!(current(), Some("outer"));
        drop(held);
    });
    assert_eq!(
        *log.lock().unwrap(),
        vec![("poll", Some("inner")), ("drop", Some("inner"))]
    );
}

#[test]
fn completed() {
    let log = Log::default();
    let mut held = Box::pin(context("inner").carry(Probe::new(log.clone(), 0, Fault::None)));
    assert_eq!(poll(held.as_mut()), Poll::Ready(7));
    assert!(catch_unwind(AssertUnwindSafe(|| poll(held.as_mut()))).is_err());
    assert_eq!(current(), None);
    drop(held);
    assert_eq!(log.lock().unwrap().len(), 2);
}

#[test]
fn travels() {
    let log = Log::default();
    let mut held = Box::pin(context("captured").carry(Probe::new(log.clone(), 1, Fault::None)));
    assert_eq!(poll(held.as_mut()), Poll::Pending);
    std::thread::spawn(move || {
        context("worker").run(|| {
            drop(held);
            assert_eq!(current(), Some("worker"));
        });
        assert_eq!(current(), None);
    })
    .join()
    .unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        vec![("poll", Some("captured")), ("drop", Some("captured"))]
    );
}
