#[path = "asynchronous/body.rs"]
mod body;

use body::{Body, Events, Step, poll};
use radicula_lock::{Cancellation, Error, Factory, Failure, Gate, Policy, Wait};
use std::{
    cell::Cell,
    future::{pending, ready},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    task::Poll,
    time::Duration,
};

fn gate() -> Gate {
    Factory::memory()
        .gate("work", Policy::EXCLUSIVE)
        .unwrap()
        .wait(Wait::immediate())
}

#[test]
fn completion() {
    let gate = gate();
    let events = Events::default();
    let mut task = Box::pin(
        gate.asynchronous()
            .guard(|| Body::new(gate.clone(), events.clone(), Step::Ready)),
    );
    assert_eq!(gate.status().active, 0);
    assert!(matches!(poll(task.as_mut()), Poll::Ready(Ok(42))));
    assert_eq!(gate.status().active, 0);
    assert_eq!(*events.borrow(), ["create", "poll", "drop"]);
    let mut business = Box::pin(
        gate.asynchronous()
            .guard(|| ready(Err::<(), _>("business"))),
    );
    assert!(matches!(
        poll(business.as_mut()),
        Poll::Ready(Ok(Err("business")))
    ));
    assert_eq!(gate.status().active, 0);
}

#[test]
fn cancellation() {
    let gate = gate();
    let events = Events::default();
    let mut task = Box::pin(
        gate.asynchronous()
            .guard(|| Body::new(gate.clone(), events.clone(), Step::Pending)),
    );
    assert!(poll(task.as_mut()).is_pending());
    assert_eq!(gate.status().active, 1);
    drop(task);
    assert_eq!(*events.borrow(), ["create", "poll", "drop"]);
    assert!(gate.acquire(Wait::immediate()).is_ok());
}

#[test]
fn unwinding() {
    let gate = gate();
    let events = Events::default();
    let mut task = Box::pin(
        gate.asynchronous()
            .guard(|| Body::new(gate.clone(), events.clone(), Step::Panic)),
    );
    let result = catch_unwind(AssertUnwindSafe(|| poll(task.as_mut())));
    assert!(result.is_err());
    assert_eq!(gate.status().active, 0);
    assert_eq!(*events.borrow(), ["create", "poll", "drop"]);
    assert!(gate.acquire(Wait::immediate()).is_ok());
    drop(task);
}

#[test]
fn destruction() {
    let gate = gate();
    let events = Events::default();
    let mut task = Box::pin(
        gate.asynchronous()
            .guard(|| Body::new(gate.clone(), events.clone(), Step::Destroy)),
    );
    assert!(catch_unwind(AssertUnwindSafe(|| poll(task.as_mut()))).is_err());
    assert_eq!(gate.status().active, 0);
    assert_eq!(*events.borrow(), ["create", "poll", "drop"]);
    drop(task);
}

#[test]
fn cleanup() {
    let gate = gate();
    let events = Events::default();
    let mut task = Box::pin(
        gate.asynchronous()
            .guard(|| Body::new(gate.clone(), events.clone(), Step::Cancel)),
    );
    assert!(poll(task.as_mut()).is_pending());
    assert!(catch_unwind(AssertUnwindSafe(|| drop(task))).is_err());
    assert_eq!(gate.status().active, 0);
    assert_eq!(*events.borrow(), ["create", "poll", "drop"]);
}

#[test]
fn queued() {
    let gate = gate();
    let entered = Cell::new(false);
    let task = gate.asynchronous().guard(|| {
        entered.set(true);
        ready(42)
    });
    drop(task);
    assert!(!entered.get());
    let events = Events::default();
    let permit = gate.acquire(Wait::immediate()).unwrap();
    let task = permit.during(|| Body::new(gate.clone(), events.clone(), Step::Ready));
    assert_eq!(gate.status().active, 1);
    drop(task);
    assert_eq!(*events.borrow(), ["create", "drop"]);
    assert_eq!(gate.status().active, 0);
}

#[test]
fn creation() {
    let gate = gate();
    let mut task = Box::pin(gate.asynchronous().guard(|| {
        assert_eq!(gate.status().active, 1);
        if gate.status().active == 1 {
            panic!("creation failure");
        }
        ready(42)
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| poll(task.as_mut()))).is_err());
    assert_eq!(gate.status().active, 0);
}

#[test]
fn admission() {
    let gate = gate();
    let entered = Cell::new(false);
    let permit = gate.acquire(Wait::immediate()).unwrap();
    let mut task = Box::pin(gate.asynchronous().guard(|| {
        entered.set(true);
        ready(42)
    }));
    assert!(matches!(
        poll(task.as_mut()),
        Poll::Ready(Err(Failure::Admission(Error::Busy)))
    ));
    assert!(!entered.get());
    drop(permit);
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let mut task = Box::pin(
        gate.asynchronous()
            .with(Wait::immediate().cancel(cancellation), || ready(42)),
    );
    assert!(matches!(
        poll(task.as_mut()),
        Poll::Ready(Err(Failure::Admission(Error::Cancelled)))
    ));
}

#[test]
fn waiting() {
    let gate = gate();
    let entered = Cell::new(false);
    for wait in [Wait::forever(), Wait::timeout(Duration::ZERO)] {
        let mut task = Box::pin(gate.asynchronous().with(wait, || {
            entered.set(true);
            ready(42)
        }));
        assert!(matches!(
            poll(task.as_mut()),
            Poll::Ready(Err(Failure::Admission(Error::Unsupported(_))))
        ));
    }
    assert!(!entered.get());
    assert_eq!(gate.status().active, 0);
}

#[test]
fn capacity() {
    let gate = Factory::memory()
        .gate("routes", Policy::limit(2).unwrap())
        .unwrap()
        .wait(Wait::immediate());
    let mut first = Box::pin(gate.asynchronous().guard(pending::<()>));
    let mut second = Box::pin(gate.asynchronous().guard(pending::<()>));
    assert!(poll(first.as_mut()).is_pending());
    assert!(poll(second.as_mut()).is_pending());
    let mut refused = Box::pin(gate.asynchronous().guard(|| ready(42)));
    assert!(matches!(
        poll(refused.as_mut()),
        Poll::Ready(Err(Failure::Admission(Error::Busy)))
    ));
    drop(first);
    let mut next = Box::pin(gate.asynchronous().guard(|| ready(42)));
    assert!(matches!(poll(next.as_mut()), Poll::Ready(Ok(42))));
    assert_eq!(gate.status().active, 1);
    drop(second);
    assert_eq!(gate.status().active, 0);
}

#[test]
fn owned() {
    let events = Events::default();
    let other = Rc::new(Cell::new(0));
    let gate = gate();
    let retained = gate.clone();
    let mut task = Box::pin(gate.asynchronous().guard(move || {
        other.set(42);
        Body::new(retained, events, Step::Pending)
    }));
    drop(gate);
    assert!(poll(task.as_mut()).is_pending());
    drop(task);
}
