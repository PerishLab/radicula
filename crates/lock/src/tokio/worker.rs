use crate::Error;
use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::mpsc,
    thread,
};
use tokio::sync::oneshot;

type Panic = Box<dyn Any + Send>;
type Job = Box<dyn FnOnce() + Send>;
pub(super) type Reply<T> = oneshot::Receiver<Result<T, Panic>>;

#[derive(Clone)]
pub(super) struct Worker(mpsc::Sender<Job>);

impl Worker {
    pub fn new() -> Result<Self, Error> {
        let (send, receive) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("radicula-carrier".into())
            .spawn(move || {
                for job in receive {
                    job();
                }
            })
            .map_err(Error::from)?;
        Ok(Self(send))
    }

    pub fn send<T: Send + 'static>(
        &self,
        action: impl FnOnce() -> T + Send + 'static,
    ) -> Result<Reply<T>, Error> {
        let (send, receive) = oneshot::channel();
        self.0
            .send(Box::new(move || {
                let result = catch_unwind(AssertUnwindSafe(action));
                let _ = send.send(result);
            }))
            .map_err(|_| {
                Error::from(std::io::Error::other("coordination worker is unavailable"))
            })?;
        Ok(receive)
    }
}

pub(super) async fn receive<T>(reply: Reply<T>) -> Result<T, Error> {
    match reply.await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(panic)) => resume_unwind(panic),
        Err(_) => Err(Error::from(std::io::Error::other(
            "coordination worker ended without a result",
        ))),
    }
}
