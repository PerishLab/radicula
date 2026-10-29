use crate::{released, support::Seat};
use radicula_lock::{Error, Factory, Mode, Policy, Wait};
use std::{fs::OpenOptions, sync::Arc, time::Duration};

fn gate(seat: &Seat) -> radicula_lock::Gate {
    let path = seat.0.join("lease");
    Factory::files(move |_| OpenOptions::new().read(true).write(true).open(&path))
        .gate("work", Policy::Readers)
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn competition() {
    let seat = Seat::new();
    let _file = seat.file();
    let mut peer = seat.spawn(Mode::Exclusive, "std", "hold");
    peer.ready();
    let gate = gate(&seat);
    let (tick, ticked) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        tokio::task::yield_now().await;
        tick.send(()).unwrap();
    });
    assert!(matches!(
        gate.tokio()
            .acquire(Wait::timeout(Duration::from_millis(30)))
            .await,
        Err(Error::Timeout)
    ));
    ticked.await.unwrap();
    peer.stop();
    let permit = gate.tokio().acquire(Wait::immediate()).await.unwrap();
    assert_eq!(seat.probe(Mode::Shared, "std"), 75);
    let mut waiting = seat.spawn(Mode::Exclusive, "std", "block");
    waiting = tokio::task::spawn_blocking(move || {
        waiting.ready();
        waiting.pending();
        waiting
    })
    .await
    .unwrap();
    permit.release().await.unwrap();
    assert_eq!(waiting.wait(), 0);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn readers() {
    let seat = Seat::new();
    let _file = seat.file();
    let gate = gate(&seat).shared().unwrap();
    let adapter = gate.tokio();
    let first = adapter.acquire(Wait::immediate()).await.unwrap();
    let second = adapter.acquire(Wait::immediate()).await.unwrap();
    assert_eq!(seat.probe(Mode::Shared, "std"), 0);
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    first.release().await.unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 75);
    second.release().await.unwrap();
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn retained() {
    let seat = Seat::new();
    let _file = seat.file();
    let gate = gate(&seat);
    let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let body = crate::Body {
        gate: gate.clone(),
        dropped: dropped.clone(),
        panic: true,
    };
    let adapter = gate.tokio();
    let mut execution = Box::pin(adapter.guard(|| body));
    std::future::poll_fn(|context| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execution.as_mut().poll(context)
        })) {
            Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
            Ok(std::task::Poll::Ready(_)) => panic!("body did not panic"),
            Err(_) => std::task::Poll::Ready(()),
        }
    })
    .await;
    assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    released(&gate).await;
    assert_eq!(seat.probe(Mode::Exclusive, "std"), 0);
    drop(execution);
}
