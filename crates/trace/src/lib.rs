use radicula_context::Context;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

mod frame;
mod future;

use frame::{Finish, Frame};
pub use future::{Carry, carry};
#[cfg(feature = "macros")]
pub use radicula_trace_macros::span;

#[derive(Clone, Debug)]
pub struct Record {
    pub id: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub tags: Vec<(String, String)>,
    pub opened: SystemTime,
    pub closed: SystemTime,
}

impl Record {
    pub fn says(&self, key: &str, value: &str) -> bool {
        self.tags
            .iter()
            .any(|(seen, held)| seen == key && held == value)
    }
}

#[doc = "A synchronous delivery callback that must return promptly. Records do not acknowledge persistence. Trace contains ordinary callback panics, but cannot isolate blocking, abort or process exit."]
pub trait Engine: Send + Sync {
    fn accept(&self, record: Record);
}

#[derive(Clone)]
pub struct Sink {
    engine: Arc<dyn Engine>,
}

impl Sink {
    #[doc = "Attempts delivery outside panic unwinding. Ordinary callback panics produce an independent stderr diagnostic; unwinding skips callbacks. No delivery acknowledgement is returned."]
    pub fn accept(&self, record: Record) {
        if std::thread::panicking() {
            return;
        }
        if catch_unwind(AssertUnwindSafe(|| self.engine.accept(record))).is_err() {
            let _ = std::io::Write::write_all(
                &mut std::io::stderr(),
                b"radicula trace: engine callback panicked\n",
            );
        }
    }
}

impl<E: Engine + 'static> From<Arc<E>> for Sink {
    fn from(engine: Arc<E>) -> Sink {
        Sink { engine }
    }
}

#[derive(Default)]
pub struct Ledger {
    records: Mutex<Vec<Record>>,
}

impl Ledger {
    pub fn query(&self, keep: impl Fn(&Record) -> bool) -> Vec<Record> {
        let records = self
            .records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        records.into_iter().filter(keep).collect()
    }
}

impl Engine for Ledger {
    fn accept(&self, record: Record) {
        self.records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(record);
    }
}

#[doc = "Runs a local span and returns the body's output unchanged. Return, business error and unwinding end the frame independently of retained Context views. Unwinding clears the frame without calling Engine."]
pub fn run<R>(name: &str, work: impl FnOnce() -> R) -> R {
    let context = Context::current();
    let frame = Frame::open(name, &context);
    let active = frame
        .as_ref()
        .map_or_else(|| context.clone(), |frame| context.with(frame.clone()));
    active.run(|| {
        let _finish = Finish::new(frame.as_ref(), true);
        work()
    })
}

#[doc = "Adds a tag to the current open frame. Missing sinks and already-ended frames are no-ops."]
pub fn note(key: &str, value: &str) {
    if let Some(frame) = Context::current().get::<Frame>() {
        frame.note(key, value);
    }
}
