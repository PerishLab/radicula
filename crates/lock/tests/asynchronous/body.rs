use radicula_lock::{Error, Gate, Wait};
use std::{
    cell::RefCell,
    future::Future,
    marker::PhantomPinned,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

pub type Events = Rc<RefCell<Vec<&'static str>>>;

#[derive(Clone, Copy)]
pub enum Step {
    Ready,
    Pending,
    Panic,
    Destroy,
    Cancel,
}

pub struct Body {
    gate: Gate,
    events: Events,
    step: Step,
    _pin: PhantomPinned,
}

impl Body {
    pub fn new(gate: Gate, events: Events, step: Step) -> Self {
        assert_eq!(gate.status().active, 1);
        events.borrow_mut().push("create");
        Self {
            gate,
            events,
            step,
            _pin: PhantomPinned,
        }
    }
}

impl Future for Body {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<usize> {
        assert_eq!(self.gate.status().active, 1);
        self.events.borrow_mut().push("poll");
        match self.step {
            Step::Pending | Step::Cancel => Poll::Pending,
            Step::Panic => panic!("poll failure"),
            Step::Ready | Step::Destroy => Poll::Ready(42),
        }
    }
}

impl Drop for Body {
    fn drop(&mut self) {
        assert!(matches!(
            self.gate.acquire(Wait::immediate()),
            Err(Error::Busy)
        ));
        self.events.borrow_mut().push("drop");
        if matches!(self.step, Step::Destroy | Step::Cancel) {
            panic!("destructor failure");
        }
    }
}

pub fn poll<F: Future>(body: Pin<&mut F>) -> Poll<F::Output> {
    body.poll(&mut Context::from_waker(Waker::noop()))
}
