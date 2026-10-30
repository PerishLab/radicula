use crate::{Error, Failure, Gate, Permit, Wait};
use std::{
    any::Any,
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Clone)]
pub struct Async(pub(crate) Gate);

impl Async {
    pub async fn guard<F: Future>(
        self,
        action: impl FnOnce() -> F,
    ) -> Result<F::Output, Failure<F::Output>> {
        let wait = self.0.wait.clone();
        self.with(wait, action).await
    }

    pub async fn with<F: Future>(
        self,
        wait: Wait,
        action: impl FnOnce() -> F,
    ) -> Result<F::Output, Failure<F::Output>> {
        if !wait.eager() {
            return Err(Failure::Admission(Error::Unsupported(
                "runtime-independent async admission requires Wait::immediate()",
            )));
        }
        let permit = self.0.acquire(wait).map_err(Failure::Admission)?;
        permit.during(action).await
    }
}

pub(crate) struct Execution<F: Future, P = Permit> {
    body: Option<Pin<Box<F>>>,
    permit: Option<P>,
}

impl<F: Future, P> Execution<F, P> {
    pub fn new(permit: P, body: F) -> Self {
        Self {
            body: Some(Box::pin(body)),
            permit: Some(permit),
        }
    }

    fn advance(&mut self, context: &mut Context<'_>) -> Poll<F::Output> {
        let output = match self
            .body
            .as_mut()
            .expect("execution already completed")
            .as_mut()
            .poll(context)
        {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(output) => output,
        };
        drop(self.body.take());
        Poll::Ready(output)
    }

    fn unwind(&mut self, payload: Box<dyn Any + Send>) -> ! {
        let permit = self.permit.take();
        drop(self.body.take());
        drop(permit);
        resume_unwind(payload)
    }
}

impl<F: Future, P: crate::scope::Release + Unpin> Future for Execution<F, P> {
    type Output = Result<F::Output, Failure<F::Output>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let output = match catch_unwind(AssertUnwindSafe(|| this.advance(context))) {
            Ok(Poll::Pending) => return Poll::Pending,
            Ok(Poll::Ready(output)) => output,
            Err(payload) => this.unwind(payload),
        };
        Poll::Ready(crate::scope::finish(
            this.permit.take().expect("owned execution permit"),
            output,
        ))
    }
}
