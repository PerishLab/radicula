use super::probe::{self, Fault, Mark, Probe, Seen};
use radicula_context::Context;
use radicula_trace::{self as trace, Engine, Ledger, Record, Sink};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Broken(AtomicUsize);

impl Engine for Broken {
    fn accept(&self, _: Record) {
        self.0.fetch_add(1, Ordering::Relaxed);
        panic!("engine failed");
    }
}

#[test]
fn delivery() {
    let engine = Arc::new(Broken::default());
    let context = Context::root().with(Sink::from(engine.clone()));
    let result = context.run(|| trace::run("ordinary", || Err::<(), _>("business")));
    assert_eq!(result, Err("business"));
    let task = context.run(|| trace::carry("async", async { 42 }));
    assert_eq!(super::support::block(task), 42);
    let mut task = Box::pin(context.run(|| trace::carry("dropped", std::future::pending::<()>())));
    assert!(probe::poll(task.as_mut()).is_pending());
    drop(task);
    assert_eq!(engine.0.load(Ordering::Relaxed), 3);
}

#[test]
fn unwinds() {
    let engine = Arc::new(Broken::default());
    let context = Context::root()
        .with(Sink::from(engine.clone()))
        .with(Mark("inside"));
    let saved = std::cell::RefCell::new(None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        context.run(|| {
            trace::run("unwind", || {
                saved.replace(Some(Context::current()));
                panic!("business failed");
            })
        })
    }));
    assert!(result.is_err());
    saved
        .into_inner()
        .unwrap()
        .run(|| trace::note("late", "ignored"));
    assert_eq!(engine.0.load(Ordering::Relaxed), 0);
    assert_eq!(probe::who(), "outside");
}

#[test]
fn poll() {
    let ledger = Arc::new(Ledger::default());
    let seen = Seen::default();
    let root = Context::root()
        .with(Sink::from(ledger.clone()))
        .with(Mark("captured"));
    let mut task =
        Box::pin(root.run(|| trace::carry("panic", Probe::new(false, Fault::Poll, &seen))));
    assert!(catch_unwind(AssertUnwindSafe(|| probe::poll(task.as_mut()))).is_err());
    assert_eq!(probe::who(), "outside");
    drop(task);
    assert_eq!(
        *seen.lock().unwrap(),
        vec!["poll:captured", "drop:captured"]
    );
    assert!(ledger.query(|_| true).is_empty());
}

#[test]
fn cleanup() {
    for ready in [false, true] {
        let ledger = Arc::new(Ledger::default());
        let seen = Seen::default();
        let root = Context::root()
            .with(Sink::from(ledger.clone()))
            .with(Mark("captured"));
        let mut task =
            Box::pin(root.run(|| trace::carry("panic", Probe::new(ready, Fault::Drop, &seen))));
        let polled = catch_unwind(AssertUnwindSafe(|| probe::poll(task.as_mut())));
        if ready {
            assert!(polled.is_err());
            std::mem::drop(task);
        } else {
            assert!(polled.unwrap().is_pending());
            assert!(catch_unwind(AssertUnwindSafe(|| std::mem::drop(task))).is_err());
        }
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["poll:captured", "drop:captured"]
        );
        assert_eq!(probe::who(), "outside");
        assert!(ledger.query(|_| true).is_empty());
    }
}
