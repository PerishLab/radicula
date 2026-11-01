use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Default)]
pub struct Cancel(Arc<State>);

#[derive(Default)]
struct State {
    cancelled: AtomicBool,
    #[cfg(feature = "tokio")]
    notify: tokio::sync::Notify,
}

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        #[cfg(feature = "tokio")]
        self.0.notify.notify_waiters();
    }

    pub fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    #[cfg(feature = "tokio")]
    pub(crate) async fn wait(&self) {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.cancelled() {
                return;
            }
            notified.await;
        }
    }
}
