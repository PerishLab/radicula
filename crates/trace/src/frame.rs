use super::{Record, Sink};
use radicula_context::Context;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Open {
    record: Record,
    sink: Sink,
}

#[derive(Clone)]
pub(super) struct Frame {
    id: u64,
    held: Arc<Mutex<Option<Open>>>,
}

impl Frame {
    pub(super) fn open(name: &str, context: &Context) -> Option<Self> {
        let sink = context.get::<Sink>()?;
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let opened = SystemTime::now();
        Some(Self {
            id,
            held: Arc::new(Mutex::new(Some(Open {
                record: Record {
                    id,
                    parent: context.get::<Frame>().map(|frame| frame.id),
                    name: name.into(),
                    tags: Vec::new(),
                    opened,
                    closed: opened,
                },
                sink: (*sink).clone(),
            }))),
        })
    }

    pub(super) fn note(&self, key: &str, value: &str) {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(open) = held.as_mut() {
            open.record.tags.push((key.into(), value.into()));
        }
    }

    fn finish(&self) {
        let open = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let Some(mut open) = open else {
            return;
        };
        if std::thread::panicking() {
            return;
        }
        open.record.closed = SystemTime::now();
        open.sink.accept(open.record);
    }
}

pub(super) struct Finish<'a> {
    frame: Option<&'a Frame>,
    pub(super) ready: bool,
}

impl<'a> Finish<'a> {
    pub(super) fn new(frame: Option<&'a Frame>, ready: bool) -> Self {
        Self { frame, ready }
    }
}

impl Drop for Finish<'_> {
    fn drop(&mut self) {
        if let Some(frame) = self
            .frame
            .filter(|_| self.ready || std::thread::panicking())
        {
            frame.finish();
        }
    }
}
