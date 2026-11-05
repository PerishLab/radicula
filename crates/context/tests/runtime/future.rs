use super::support::{Fault, Log, Output, Probe, context, current, poll};
use std::future::ready;
use std::task::Poll;

#[test]
fn completes() {
    let log = Log::default();
    let mut held = Box::pin(context("inner").carry(Probe::new(log.clone(), 1, Fault::None)));
    context("outer").run(|| {
        assert_eq!(poll(held.as_mut()), Poll::Pending);
        assert_eq!(current(), Some("outer"));
        assert_eq!(poll(held.as_mut()), Poll::Ready(7));
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            ("poll", Some("inner")),
            ("poll", Some("inner")),
            ("drop", Some("inner"))
        ]
    );
    drop(held);
    assert_eq!(log.lock().unwrap().len(), 3);
}

#[test]
fn interleaves() {
    let a = Log::default();
    let b = Log::default();
    let mut first = Box::pin(context("a").carry(Probe::new(a.clone(), 1, Fault::None)));
    let mut second = Box::pin(context("b").carry(Probe::new(b.clone(), 1, Fault::None)));
    assert_eq!(poll(first.as_mut()), Poll::Pending);
    assert_eq!(poll(second.as_mut()), Poll::Pending);
    assert_eq!(poll(first.as_mut()), Poll::Ready(7));
    assert_eq!(poll(second.as_mut()), Poll::Ready(7));
    assert!(a.lock().unwrap().iter().all(|(_, name)| *name == Some("a")));
    assert!(b.lock().unwrap().iter().all(|(_, name)| *name == Some("b")));
    assert_eq!(current(), None);
}

#[test]
fn migrates() {
    let log = Log::default();
    let mut held = Box::pin(context("captured").carry(Probe::new(log.clone(), 1, Fault::None)));
    assert_eq!(poll(held.as_mut()), Poll::Pending);
    assert_eq!(current(), None);
    std::thread::spawn(move || {
        context("worker").run(|| {
            assert_eq!(poll(held.as_mut()), Poll::Ready(7));
            assert_eq!(current(), Some("worker"));
        });
        assert_eq!(current(), None);
    })
    .join()
    .unwrap();
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .all(|(_, name)| *name == Some("captured"))
    );
}

#[test]
fn output() {
    let log = Log::default();
    context("caller").run(|| {
        let mut held = Box::pin(context("inner").carry(ready(Output(log.clone()))));
        let Poll::Ready(output) = poll(held.as_mut()) else {
            panic!("ready")
        };
        drop(held);
        assert!(log.lock().unwrap().is_empty());
        drop(output);
        assert_eq!(*log.lock().unwrap(), vec![("output", Some("caller"))]);
    });
}

#[test]
fn borrows() {
    let mut value = 0;
    {
        let mut held = Box::pin(context("inner").carry(async {
            assert_eq!(current(), Some("inner"));
            value += 1;
            Err::<(), _>("business")
        }));
        assert_eq!(poll(held.as_mut()), Poll::Ready(Err("business")));
    }
    assert_eq!(value, 1);
    assert_eq!(current(), None);
}

#[test]
fn local() {
    let value = std::rc::Rc::new(7);
    let mut held = Box::pin(context("local").carry(async move {
        assert_eq!(current(), Some("local"));
        *value
    }));
    assert_eq!(poll(held.as_mut()), Poll::Ready(7));
    assert_eq!(current(), None);
}
