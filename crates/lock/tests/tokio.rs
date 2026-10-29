#![cfg(feature = "tokio")]

#[path = "tokio/file.rs"]
mod file;
mod support;

#[test]
fn worker() {
    support::worker();
}

use radicula_lock::{Cancellation, Error, Factory, Gate, Policy, Wait};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Context, Poll},
    time::Duration,
};

fn gate() -> Gate {
    Factory::memory().gate("work", Policy::EXCLUSIVE).unwrap()
}

async fn released(gate: &Gate) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while gate.status().active != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn waiting() {
    let gate = gate();
    let held = gate.acquire(Wait::immediate()).unwrap();
    assert!(matches!(
        gate.tokio()
            .acquire(Wait::timeout(Duration::from_millis(20)))
            .await,
        Err(Error::Timeout)
    ));
    let cancellation = Cancellation::default();
    let requested = cancellation.clone();
    let adapter = gate.tokio();
    let waiting =
        tokio::spawn(async move { adapter.acquire(Wait::forever().cancel(requested)).await });
    cancellation.cancel();
    assert!(matches!(waiting.await.unwrap(), Err(Error::Cancelled)));
    drop(held);
    assert_eq!(
        gate.tokio()
            .guard(|| async { Err::<(), _>("business") })
            .await
            .unwrap(),
        Err("business")
    );
    assert_eq!(gate.status().active, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn contention() {
    let gate = gate();
    let first = gate.acquire(Wait::immediate()).unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let observed = entered.clone();
    let adapter = gate.tokio();
    let waiter = tokio::spawn(async move {
        adapter
            .guard(|| async move {
                observed.store(true, Ordering::SeqCst);
            })
            .await
    });
    tokio::task::yield_now().await;
    assert!(!entered.load(Ordering::SeqCst));
    drop(first);
    waiter.await.unwrap().unwrap();
    assert!(entered.load(Ordering::SeqCst));
    assert_eq!(gate.status().active, 0);
}

struct Body {
    gate: Gate,
    dropped: Arc<AtomicBool>,
    panic: bool,
}

impl Future for Body {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        if self.panic {
            panic!("business poll");
        }
        Poll::Pending
    }
}

impl Drop for Body {
    fn drop(&mut self) {
        assert_eq!(self.gate.status().active, 1);
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation() {
    let gate = gate();
    let dropped = Arc::new(AtomicBool::new(false));
    let (started, ready) = tokio::sync::oneshot::channel();
    let body = Body {
        gate: gate.clone(),
        dropped: dropped.clone(),
        panic: false,
    };
    let adapter = gate.tokio();
    let task = tokio::spawn(async move {
        adapter
            .guard(|| {
                started.send(()).unwrap();
                body
            })
            .await
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(dropped.load(Ordering::SeqCst));
    released(&gate).await;
    assert!(gate.acquire(Wait::immediate()).is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn panic() {
    let gate = gate();
    let dropped = Arc::new(AtomicBool::new(false));
    let body = Body {
        gate: gate.clone(),
        dropped: dropped.clone(),
        panic: true,
    };
    let adapter = gate.tokio();
    let task = tokio::spawn(async move { adapter.guard(|| body).await });
    assert!(task.await.unwrap_err().is_panic());
    assert!(dropped.load(Ordering::SeqCst));
    released(&gate).await;
}

#[tokio::test(flavor = "current_thread")]
async fn blocking() {
    let gate = gate();
    let adapter = gate.tokio();
    let (entered, started) = tokio::sync::oneshot::channel();
    let (finish, pending) = mpsc::channel();
    let observed = gate.clone();
    let task = tokio::spawn(async move {
        adapter
            .blocking(Wait::forever(), move || {
                assert_eq!(observed.status().active, 1);
                entered.send(()).unwrap();
                pending.recv_timeout(Duration::from_secs(2)).unwrap();
                assert_eq!(observed.status().active, 1);
            })
            .await
    });
    started.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(matches!(gate.acquire(Wait::immediate()), Err(Error::Busy)));
    finish.send(()).unwrap();
    released(&gate).await;
}

#[tokio::test(flavor = "current_thread")]
async fn unpolled() {
    let gate = gate();
    let invoked = Arc::new(AtomicBool::new(false));
    let mark = invoked.clone();
    let adapter = gate.tokio();
    let future = adapter.guard(|| {
        mark.store(true, Ordering::SeqCst);
        async {}
    });
    drop(future);
    assert!(!invoked.load(Ordering::SeqCst));
    assert_eq!(gate.status().active, 0);
}

#[test]
fn shutdown() {
    let gate = gate();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let permit = runtime
        .block_on(gate.tokio().acquire(Wait::immediate()))
        .unwrap();
    drop(runtime);
    assert_eq!(gate.status().active, 1);
    drop(permit);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while gate.status().active != 0 {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn local() {
    let gate = gate();
    let value = std::rc::Rc::new(42);
    assert_eq!(
        *gate
            .tokio()
            .guard(|| async { value.clone() })
            .await
            .unwrap(),
        42
    );
    assert_eq!(gate.status().active, 0);
}

#[cfg(feature = "macros")]
#[radicula_lock::guard(gate = adapter, error = |_| Err("coordination"))]
async fn method(adapter: &radicula_lock::tokio::Tokio) -> Result<usize, &'static str> {
    Ok(42)
}

#[cfg(feature = "macros")]
#[tokio::test(flavor = "current_thread")]
async fn methods() {
    let gate = gate();
    assert_eq!(method(&gate.tokio()).await.unwrap(), 42);
    assert_eq!(gate.status().active, 0);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn clock() {
    let gate = gate();
    let held = gate.acquire(Wait::immediate()).unwrap();
    let adapter = gate.tokio();
    let pending = tokio::spawn(async move {
        adapter
            .acquire(Wait::timeout(Duration::from_secs(30)))
            .await
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(31)).await;
    assert!(matches!(pending.await.unwrap(), Err(Error::Timeout)));
    drop(held);
}
