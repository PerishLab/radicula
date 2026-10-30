use super::worker::Worker;
use crate::{Cancellation, Error, Factory, Policy, Wait};
use std::{
    future::Future,
    sync::mpsc,
    task::{Context, Poll, Waker},
    time::Duration,
};

fn flush(worker: &Worker) {
    let (send, receive) = mpsc::channel();
    drop(worker.send(move || send.send(()).unwrap()).unwrap());
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn cancellation() {
    let gate = Factory::memory().gate("work", Policy::EXCLUSIVE).unwrap();
    let adapter = gate.tokio();
    let worker = adapter.worker.get_or_init(Worker::new).clone().unwrap();
    let (started, ready) = mpsc::channel();
    let (finish, pending) = mpsc::channel();
    drop(
        worker
            .send(move || {
                started.send(()).unwrap();
                pending.recv().unwrap();
            })
            .unwrap(),
    );
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let token = Cancellation::default();
    let mut acquisition = Box::pin(adapter.acquire(Wait::forever().cancel(token.clone())));
    let mut context = Context::from_waker(Waker::noop());
    assert!(acquisition.as_mut().poll(&mut context).is_pending());
    token.cancel();
    finish.send(()).unwrap();
    flush(&worker);
    assert_eq!(gate.status().active, 1);
    assert!(matches!(
        acquisition.as_mut().poll(&mut context),
        Poll::Ready(Err(Error::Cancelled))
    ));
    flush(&worker);
    assert_eq!(gate.status().active, 0);
}
