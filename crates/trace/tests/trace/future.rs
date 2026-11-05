use super::probe::{self, Fault, Mark, Probe, Seen};
use radicula_context::Context;
use radicula_trace::{self as trace, Ledger, Sink};
use std::rc::Rc;
use std::sync::Arc;

struct Returned(Seen);

impl Drop for Returned {
    fn drop(&mut self) {
        self.0
            .lock()
            .unwrap()
            .push(format!("output:{}", probe::who()));
    }
}

#[test]
fn unpolled() {
    let ledger = Arc::new(Ledger::default());
    let seen = Seen::default();
    let root = Context::root()
        .with(Sink::from(ledger.clone()))
        .with(Mark("captured"));
    let task = root.run(|| trace::carry("never", Probe::new(false, Fault::None, &seen)));
    Context::root().with(Mark("caller")).run(|| {
        drop(task);
        assert_eq!(probe::who(), "caller");
    });
    assert_eq!(*seen.lock().unwrap(), vec!["drop:captured"]);
    assert!(ledger.query(|_| true).is_empty());
}

#[test]
fn pending() {
    let ledger = Arc::new(Ledger::default());
    let seen = Seen::default();
    let root = Context::root()
        .with(Sink::from(ledger.clone()))
        .with(Mark("captured"));
    let mut task =
        Box::pin(root.run(|| trace::carry("pending", Probe::new(false, Fault::None, &seen))));
    assert!(probe::poll(task.as_mut()).is_pending());
    assert!(ledger.query(|_| true).is_empty());
    std::thread::spawn(move || {
        assert!(probe::poll(task.as_mut()).is_pending());
        drop(task);
        assert_eq!(probe::who(), "outside");
    })
    .join()
    .unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(
        *seen,
        vec!["poll:captured", "poll:captured", "drop:captured"]
    );
    let records = ledger.query(|_| true);
    assert_eq!(records.len(), 1);
    assert!(records[0].says("drop", "captured"));
}

#[test]
fn ready() {
    let ledger = Arc::new(Ledger::default());
    let seen = Seen::default();
    let root = Context::root()
        .with(Sink::from(ledger.clone()))
        .with(Mark("captured"));
    let mut task =
        Box::pin(root.run(|| trace::carry("ready", Probe::new(true, Fault::None, &seen))));
    assert_eq!(probe::poll(task.as_mut()), std::task::Poll::Ready(7));
    assert_eq!(
        *seen.lock().unwrap(),
        vec!["poll:captured", "drop:captured"]
    );
    let records = ledger.query(|_| true);
    assert_eq!(records.len(), 1);
    assert!(records[0].says("drop", "captured"));
    drop(task);
    assert_eq!(seen.lock().unwrap().len(), 2);
    assert_eq!(ledger.query(|_| true).len(), 1);
}

#[test]
fn interleaves() {
    let ledger = Arc::new(Ledger::default());
    let root = Context::root().with(Sink::from(ledger.clone()));
    let first = root.with(Mark("first"));
    let second = root.with(Mark("second"));
    let mut left = Box::pin(first.run(|| {
        trace::carry("first", async {
            assert_eq!(probe::who(), "first");
            super::support::rest().await;
            assert_eq!(probe::who(), "first");
        })
    }));
    let mut right = Box::pin(second.run(|| {
        trace::carry("second", async {
            assert_eq!(probe::who(), "second");
            super::support::rest().await;
            assert_eq!(probe::who(), "second");
        })
    }));
    assert!(probe::poll(left.as_mut()).is_pending());
    assert!(probe::poll(right.as_mut()).is_pending());
    assert!(probe::poll(left.as_mut()).is_ready());
    assert!(probe::poll(right.as_mut()).is_ready());
    assert_eq!(probe::who(), "outside");
    let records = ledger.query(|_| true);
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| record.parent.is_none()));
}

#[test]
fn borrowed() {
    let value = Rc::new(String::from("borrowed"));
    let task = trace::carry("borrowed", async { value.as_str() });
    assert_eq!(super::support::block(task), "borrowed");
}

#[test]
fn output() {
    let ledger = Arc::new(Ledger::default());
    let seen = Seen::default();
    let root = Context::root()
        .with(Sink::from(ledger.clone()))
        .with(Mark("captured"));
    let mut task = Box::pin(root.run(|| trace::carry("output", async { Returned(seen.clone()) })));
    let std::task::Poll::Ready(output) = probe::poll(task.as_mut()) else {
        panic!("ready output");
    };
    assert_eq!(ledger.query(|_| true).len(), 1);
    drop(task);
    assert!(seen.lock().unwrap().is_empty());
    Context::root().with(Mark("caller")).run(|| drop(output));
    assert_eq!(*seen.lock().unwrap(), vec!["output:caller"]);
}
