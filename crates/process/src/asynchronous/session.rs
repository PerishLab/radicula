use super::{Captured, Completed, Error, Failure, Observer, Outcome, State};

pub struct Session<T> {
    task: Option<tokio::task::JoinHandle<Result<Completed<T>, Failure<T>>>>,
    observer: Observer,
}

impl<T> Session<T> {
    pub(crate) fn new(
        task: tokio::task::JoinHandle<Result<Completed<T>, Failure<T>>>,
        observer: Observer,
    ) -> Self {
        Self {
            task: Some(task),
            observer,
        }
    }

    pub fn observer(&self) -> Observer {
        self.observer.clone()
    }

    pub fn cancel(&self) {
        self.observer.cancel();
    }

    pub async fn wait(&mut self) -> Outcome<T> {
        let task = self.task.as_mut().ok_or(Error::Consumed)?;
        let result = task.await;
        self.task.take();
        match result {
            Ok(Ok(completed)) => Ok(completed),
            Ok(Err(failure)) => Err(Error::Operation(Box::new(failure))),
            Err(fault) => {
                self.observer.set(State::Uncertain);
                Err(Error::Interrupted {
                    fault,
                    observer: self.observer.clone(),
                })
            }
        }
    }
}

impl Session<Captured> {
    pub async fn output(&mut self) -> Result<std::process::Output, Error<Captured>> {
        let completed = self.wait().await?;
        Ok(std::process::Output {
            status: completed.status,
            stdout: completed.streams.stdout.unwrap_or_default(),
            stderr: completed.streams.stderr.unwrap_or_default(),
        })
    }
}

impl<T> Drop for Session<T> {
    fn drop(&mut self) {
        if matches!(self.observer.state(), State::Running | State::Stopping) {
            self.observer.cancel();
        }
    }
}
