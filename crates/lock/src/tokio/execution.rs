use super::{
    Permit,
    worker::{self, Reply},
};
use crate::{Error, Failure};
use std::{
    any::Any,
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    pin::Pin,
    task::{Context, Poll},
};

type Release = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

pub(super) struct Execution<F: Future> {
    body: Option<Pin<Box<F>>>,
    output: Option<F::Output>,
    release: Option<Release>,
    permit: Option<Permit>,
}

impl<F: Future> Execution<F> {
    pub fn new(permit: Permit, body: F) -> Self {
        Self {
            body: Some(Box::pin(body)),
            output: None,
            release: None,
            permit: Some(permit),
        }
    }

    fn advance(&mut self, context: &mut Context<'_>) -> Poll<()> {
        let output = match self
            .body
            .as_mut()
            .expect("execution body")
            .as_mut()
            .poll(context)
        {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(output) => output,
        };
        drop(self.body.take());
        self.output = Some(output);
        let release = self.permit.take().expect("execution permit").finish();
        self.release = Some(Box::pin(released(release)));
        Poll::Ready(())
    }

    fn unwind(&mut self, panic: Box<dyn Any + Send>) -> ! {
        let permit = self.permit.take();
        drop(self.body.take());
        drop(permit);
        resume_unwind(panic)
    }
}

impl<F: Future> Future for Execution<F> {
    type Output = Result<F::Output, Failure<F::Output>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.body.is_some() {
            match catch_unwind(AssertUnwindSafe(|| this.advance(context))) {
                Ok(Poll::Pending) => return Poll::Pending,
                Ok(Poll::Ready(())) => {}
                Err(panic) => this.unwind(panic),
            }
        }
        let result = match this
            .release
            .as_mut()
            .expect("execution release")
            .as_mut()
            .poll(context)
        {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => result,
        };
        let output = this.output.take().expect("execution output");
        Poll::Ready(match result {
            Ok(()) => Ok(output),
            Err(error) => Err(Failure::Release { output, error }),
        })
    }
}

impl<F: Future> Unpin for Execution<F> {}

async fn released(reply: Result<Reply<Result<(), Error>>, Error>) -> Result<(), Error> {
    worker::receive(reply?).await?
}
