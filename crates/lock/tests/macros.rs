#![cfg(feature = "macros")]

use radicula_lock::{Error, Factory, Failure, Gate, Policy, Wait, guard};
use std::{
    cell::Cell,
    future::Future,
    panic::AssertUnwindSafe,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[path = "macros/compile.rs"]
mod compile;

type Output = Result<usize, &'static str>;

fn refusal(failure: Failure<Output>) -> Output {
    match failure {
        Failure::Admission(Error::Busy) => Err("busy"),
        _ => Err("coordination"),
    }
}

struct Work {
    gate: Gate,
    calls: Cell<usize>,
    value: usize,
}

impl Work {
    fn new() -> Self {
        Self {
            gate: Factory::memory()
                .gate("work", Policy::EXCLUSIVE)
                .unwrap()
                .wait(Wait::immediate()),
            calls: Cell::new(0),
            value: 0,
        }
    }

    fn select(&self) -> Gate {
        self.calls.set(self.calls.get() + 1);
        self.gate.clone()
    }

    #[guard(gate = self.select(), error = refusal)]
    fn increment(&mut self, value: Output) -> Output {
        assert_eq!(self.gate.status().active, 1);
        if value == Ok(0) {
            return Err("early");
        }
        self.value += value?;
        Ok(self.value)
    }

    #[guard(gate = self.select(), error = refusal)]
    async fn suspend(&mut self, dropped: Rc<Cell<bool>>) -> Output {
        let witness = Witness(self.gate.clone(), dropped);
        assert_eq!(self.gate.status().active, 1);
        self.value += 1;
        std::future::pending::<()>().await;
        drop(witness);
        Ok(self.value)
    }

    #[guard(gate = self.gate, error = refusal)]
    async fn complete(&mut self, value: Output) -> Output {
        assert_eq!(self.gate.status().active, 1);
        self.value += value?;
        Ok(self.value)
    }
}

struct Witness(Gate, Rc<Cell<bool>>);

impl Drop for Witness {
    fn drop(&mut self) {
        assert_eq!(self.0.status().active, 1);
        self.1.set(true);
    }
}

#[test]
fn synchronous() {
    let mut work = Work::new();
    assert_eq!(work.increment(Ok(2)), Ok(2));
    assert_eq!(work.increment(Ok(0)), Err("early"));
    assert_eq!(work.increment(Err("business")), Err("business"));
    assert_eq!(work.value, 2);
    assert_eq!(work.calls.get(), 3);
    assert_eq!(work.gate.status().active, 0);
    let permit = work.gate.acquire(Wait::immediate()).unwrap();
    assert_eq!(work.increment(Ok(8)), Err("busy"));
    assert_eq!(work.value, 2);
    drop(permit);
    assert_eq!(work.gate.guard(|| work.gate.status().active).unwrap(), 1);
    assert_eq!(work.increment(Ok(3)), Ok(5));
}

#[guard(gate = gate, error = mapper)]
fn generic<T>(gate: &Gate, value: T, mapper: impl FnOnce(Failure<T>) -> T) -> T {
    let _: &Gate = gate;
    value
}

#[test]
fn mapping() {
    let work = Work::new();
    let invoked = Cell::new(0);
    assert_eq!(
        generic(&work.gate, 42, |_| {
            invoked.set(invoked.get() + 1);
            0
        }),
        42
    );
    assert_eq!(invoked.get(), 0);
    let permit = work.gate.acquire(Wait::immediate()).unwrap();
    assert_eq!(
        generic(&work.gate, 42, |_| {
            invoked.set(invoked.get() + 1);
            0
        }),
        0
    );
    assert_eq!(invoked.get(), 1);
    drop(permit);
}

#[guard(gate = gate, error = |_: Failure<()>| panic!("admission failure"))]
fn panicking(gate: &Gate) {
    assert_eq!(gate.status().active, 1);
    panic!("business panic");
}

#[test]
fn unwinding() {
    let work = Work::new();
    assert!(std::panic::catch_unwind(AssertUnwindSafe(|| panicking(&work.gate))).is_err());
    assert_eq!(work.gate.status().active, 0);
}

#[test]
fn asynchronous() {
    let mut work = Work::new();
    let gate = work.gate.clone();
    let mut context = Context::from_waker(Waker::noop());
    let mut task = Box::pin(work.complete(Ok(4)));
    assert!(matches!(
        task.as_mut().poll(&mut context),
        Poll::Ready(Ok(4))
    ));
    assert_eq!(gate.status().active, 0);
    drop(task);
    let mut task = Box::pin(work.complete(Err("business")));
    assert!(matches!(
        task.as_mut().poll(&mut context),
        Poll::Ready(Err("business"))
    ));
    assert_eq!(gate.status().active, 0);
    drop(task);
    let permit = gate.acquire(Wait::immediate()).unwrap();
    let mut task = Box::pin(work.complete(Ok(8)));
    assert!(matches!(
        task.as_mut().poll(&mut context),
        Poll::Ready(Err("busy"))
    ));
    drop(task);
    drop(permit);
    assert_eq!(work.value, 4);
}

#[test]
fn cancellation() {
    let mut work = Work::new();
    let gate = work.gate.clone();
    let dropped = Rc::new(Cell::new(false));
    let task = work.suspend(dropped.clone());
    drop(task);
    assert_eq!(work.calls.get(), 0);
    let mut task = Box::pin(work.suspend(dropped.clone()));
    let mut context = Context::from_waker(Waker::noop());
    assert!(task.as_mut().poll(&mut context).is_pending());
    assert_eq!(gate.status().active, 1);
    assert!(!dropped.get());
    drop(task);
    assert!(dropped.get());
    assert_eq!(gate.status().active, 0);
    assert_eq!(work.calls.get(), 1);
    assert_eq!(work.value, 1);
}

#[test]
fn capacity() {
    #[guard(gate = gate, error = refusal)]
    async fn waiting(gate: &Gate) -> Output {
        std::future::pending().await
    }
    let domain = Factory::memory();
    let first = domain
        .gate("slots", Policy::limit(2).unwrap())
        .unwrap()
        .wait(Wait::immediate());
    let second = domain
        .gate("slots", Policy::limit(2).unwrap())
        .unwrap()
        .wait(Wait::immediate());
    let direct = domain
        .gate("slots", Policy::limit(2).unwrap())
        .unwrap()
        .wait(Wait::immediate());
    let mut left = Box::pin(waiting(&first));
    let mut right = Box::pin(waiting(&second));
    let mut context = Context::from_waker(Waker::noop());
    assert!(left.as_mut().poll(&mut context).is_pending());
    assert!(right.as_mut().poll(&mut context).is_pending());
    assert_eq!(direct.status().active, 2);
    assert!(matches!(
        direct.acquire(Wait::immediate()),
        Err(Error::Busy)
    ));
    drop(left);
    assert_eq!(direct.status().active, 1);
    assert_eq!(direct.guard(|| direct.status().active).unwrap(), 2);
    drop(right);
    assert_eq!(direct.status().active, 0);
}
