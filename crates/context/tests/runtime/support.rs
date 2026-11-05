use radicula_context::Context;
use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Mark(pub &'static str);
pub type Log = Arc<Mutex<Vec<(&'static str, Option<&'static str>)>>>;

pub fn current() -> Option<&'static str> {
    Context::current().get::<Mark>().map(|held| held.0)
}

pub fn context(name: &'static str) -> Context {
    Context::root().with(Mark(name))
}

pub fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut std::task::Context::from_waker(Waker::noop()))
}

#[derive(Clone, Copy)]
pub enum Fault {
    None,
    Poll,
    Drop,
}

pub struct Probe {
    pub log: Log,
    pub left: Cell<usize>,
    pub fault: Fault,
    pin: PhantomPinned,
}

impl Probe {
    pub fn new(log: Log, left: usize, fault: Fault) -> Self {
        Self {
            log,
            left: Cell::new(left),
            fault,
            pin: PhantomPinned,
        }
    }
}

impl Future for Probe {
    type Output = u8;

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<u8> {
        let _ = &self.pin;
        self.log.lock().unwrap().push(("poll", current()));
        if matches!(self.fault, Fault::Poll) {
            panic!("poll probe");
        }
        match self.left.get() {
            0 => Poll::Ready(7),
            left => {
                self.left.set(left - 1);
                task.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.log.lock().unwrap().push(("drop", current()));
        if matches!(self.fault, Fault::Drop) {
            panic!("drop probe");
        }
    }
}

pub struct Output(pub Log);

impl Drop for Output {
    fn drop(&mut self) {
        self.0.lock().unwrap().push(("output", current()));
    }
}
