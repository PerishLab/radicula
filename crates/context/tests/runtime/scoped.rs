use super::support::{context, current, poll};
use radicula_context::Context;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::Poll;

#[test]
fn returns() {
    assert_eq!(current(), None);
    let result = context("inner").run(|| {
        assert_eq!(current(), Some("inner"));
        Err::<(), _>("business")
    });
    assert_eq!(result, Err("business"));
    assert_eq!(current(), None);
}

#[test]
fn nests() {
    context("outer").run(|| {
        let captured = Context::current();
        context("inner").run(|| assert_eq!(current(), Some("inner")));
        assert_eq!(current(), Some("outer"));
        assert_eq!(captured.get::<super::support::Mark>().unwrap().0, "outer");
    });
    assert_eq!(current(), None);
}

#[test]
fn unwinds() {
    context("outer").run(|| {
        let result = catch_unwind(AssertUnwindSafe(|| {
            context("inner").run(|| panic!("business"));
        }));
        assert!(result.is_err());
        assert_eq!(current(), Some("outer"));
    });
    assert_eq!(current(), None);
}

#[test]
fn absent() {
    Context::root().with(Some(1_u8)).run(|| {
        Context::current().with(None::<u8>).run(|| {
            assert_eq!(*Context::current().get::<Option<u8>>().unwrap(), None);
        });
        assert_eq!(*Context::current().get::<Option<u8>>().unwrap(), Some(1));
    });
    assert!(Context::current().get::<Option<u8>>().is_none());
}

#[test]
fn construction() {
    let mut future = Box::pin(context("construction").run(|| async { current() }));
    assert_eq!(poll(future.as_mut()), Poll::Ready(None));
}

#[test]
fn threads() {
    context("parent").run(|| {
        let captured = Context::current();
        std::thread::spawn(move || {
            assert_eq!(current(), None);
            captured.run(|| assert_eq!(current(), Some("parent")));
            assert_eq!(current(), None);
        })
        .join()
        .unwrap();
        assert_eq!(current(), Some("parent"));
    });
}
