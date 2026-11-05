use super::frame::{Finish, Frame};
use radicula_context::Context;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

#[doc = "Captures the current Root Context. The first poll opens the span; Ready, unwind or destruction ends it locally. Inner-future poll and destruction use the captured context, and Ready destroys the inner future before returning its output. An unpolled future opens no span. Threads and tasks require explicit propagation."]
pub fn carry<F: Future>(name: &str, future: F) -> Carry<F> {
    Carry {
        context: Context::current(),
        name: Some(name.into()),
        frame: None,
        future: Some(future),
    }
}

pin_project_lite::pin_project! {
    #[must_use = "a traced future must be polled to execute its body"]
    pub struct Carry<F> {
        context: Context,
        name: Option<String>,
        frame: Option<Frame>,
        #[pin]
        future: Option<F>,
    }

    impl<F> PinnedDrop for Carry<F> {
        fn drop(this: Pin<&mut Self>) {
            let held = this.project();
            let mut future = held.future;
            held.context.run(|| {
                let _finish = Finish::new(held.frame.as_ref(), true);
                future.set(None);
            });
        }
    }
}

impl<F: Future> Future for Carry<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let held = self.project();
        if let Some(name) = held.name.take() {
            *held.frame = Frame::open(&name, held.context);
            if let Some(frame) = held.frame.as_ref() {
                *held.context = held.context.with(frame.clone());
            }
        }
        let mut future = held.future;
        held.context.run(|| {
            let mut finish = Finish::new(held.frame.as_ref(), false);
            let result = future
                .as_mut()
                .as_pin_mut()
                .expect("traced future polled after completion")
                .poll(task);
            if result.is_ready() {
                finish.ready = true;
                future.set(None);
            }
            result
        })
    }
}
