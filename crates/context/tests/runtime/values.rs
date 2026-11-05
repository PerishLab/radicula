use radicula_context::Context;
use std::sync::{Arc, Mutex};

#[test]
fn derives() {
    let root = Context::root();
    let first = root.with(1_u8);
    let second = first.with(2_u8).with("other");
    assert!(root.get::<u8>().is_none());
    assert_eq!(*first.get::<u8>().unwrap(), 1);
    assert_eq!(*second.get::<u8>().unwrap(), 2);
    assert_eq!(*second.get::<&str>().unwrap(), "other");
    assert!(second.get::<u16>().is_none());
}

#[test]
fn shares() {
    let first = Context::default().with(Mutex::new(1_u8));
    let second = first.clone().with(false);
    let a = first.get::<Mutex<u8>>().unwrap();
    let b = second.get::<Mutex<u8>>().unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    *a.lock().unwrap() = 2;
    assert_eq!(*b.lock().unwrap(), 2);
}

#[test]
fn masks() {
    let first = Context::root().with(Some(1_u8));
    let second = first.with(None::<u8>);
    assert_eq!(*first.get::<Option<u8>>().unwrap(), Some(1));
    assert_eq!(*second.get::<Option<u8>>().unwrap(), None);
    assert!(Context::root().get::<Option<u8>>().is_none());
}
