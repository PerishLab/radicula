use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

struct Parker(Thread);

impl Wake for Parker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

pub fn block<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = Waker::from(Arc::new(Parker(std::thread::current())));
    let mut task = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut task) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

#[derive(Default)]
pub struct Yield {
    rested: bool,
}

pub fn rest() -> Yield {
    Yield::default()
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<()> {
        if self.rested {
            Poll::Ready(())
        } else {
            self.rested = true;
            task.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
