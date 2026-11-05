use radicula_context::Context;
use radicula_trace as trace;
use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub(super) struct Mark(pub &'static str);

#[derive(PartialEq)]
pub(super) enum Fault {
    None,
    Poll,
    Drop,
}

pub(super) type Seen = Arc<Mutex<Vec<String>>>;

pub(super) struct Probe {
    ready: Cell<bool>,
    fault: Fault,
    seen: Seen,
    _pin: PhantomPinned,
}

impl Probe {
    pub(super) fn new(ready: bool, fault: Fault, seen: &Seen) -> Self {
        Self {
            ready: Cell::new(ready),
            fault,
            seen: seen.clone(),
            _pin: PhantomPinned,
        }
    }

    fn note(&self, phase: &str) {
        let who = who();
        self.seen.lock().unwrap().push(format!("{phase}:{who}"));
        trace::note(phase, who);
    }
}

impl Future for Probe {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<u32> {
        self.note("poll");
        assert!(self.fault != Fault::Poll, "probe poll failed");
        if self.ready.get() {
            Poll::Ready(7)
        } else {
            Poll::Pending
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.note("drop");
        assert!(self.fault != Fault::Drop, "probe drop failed");
    }
}

pub(super) fn who() -> &'static str {
    Context::current()
        .get::<Mark>()
        .map_or("outside", |mark| mark.0)
}

pub(super) fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut std::task::Context::from_waker(Waker::noop()))
}
