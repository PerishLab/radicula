use super::{Config, Credentials, Fence, key::Key, reply, wire::Reply};
use crate::{Error, Policy};
use std::time::Duration;

fn config() -> Config {
    Config::new("127.0.0.1:6379".parse().unwrap(), "conformance")
}

#[test]
fn budgets() {
    assert!(config().validate().is_ok());
    assert!(config().slots(0).validate().is_err());
    assert!(config().io(Duration::ZERO).validate().is_err());
    assert!(
        config()
            .lease(Duration::from_secs(1), None)
            .validate()
            .is_err()
    );
    assert!(
        config()
            .lease(Duration::from_secs(30), Some(Duration::from_secs(8)))
            .validate()
            .is_err()
    );
    assert!(config().lease(Duration::MAX, None).validate().is_err());
}

#[test]
fn redaction() {
    let value = config().credentials(Credentials::user("private-user", "private-secret"));
    let rendered = format!("{value:?}");
    assert!(!rendered.contains("private-user"));
    assert!(!rendered.contains("private-secret"));
}

#[test]
fn uncertain() {
    let key = Key::new("test", "work", Policy::EXCLUSIVE).unwrap();
    let malformed = Reply::Array(vec![Reply::Integer(1)]);
    assert!(matches!(
        reply::admission(malformed, &key, "epoch"),
        Err(Error::Unconfirmed(_))
    ));
    assert!(key.admits().is_err());
    assert!(key.status().fault.is_some());
    assert_eq!(key.status().active, 0);
}

#[test]
fn busy() {
    let key = Key::new("test", "work", Policy::EXCLUSIVE).unwrap();
    assert!(
        reply::admission(Reply::Array(vec![Reply::Integer(0)]), &key, "epoch")
            .unwrap()
            .is_none()
    );
    assert!(key.admits().is_ok());
}

#[test]
fn epochs() {
    let key = Key::new("test", "work", Policy::EXCLUSIVE).unwrap();
    assert!(matches!(
        reply::admission(Reply::Array(vec![Reply::Integer(-2)]), &key, "old"),
        Err(Error::Epoch)
    ));
    assert!(key.admits().is_err());
    let before = Fence {
        epoch: "old".into(),
        sequence: 20,
    };
    let after = Fence {
        epoch: "new".into(),
        sequence: 21,
    };
    assert!(matches!(after.follows(&before), Err(Error::Epoch)));
    assert!(
        Fence {
            sequence: 21,
            ..before.clone()
        }
        .follows(&before)
        .unwrap()
    );
}

#[test]
fn precision() {
    let key = Key::new("test", "work", Policy::EXCLUSIVE).unwrap();
    let reply = Reply::Array(vec![
        Reply::Integer(1),
        Reply::Text(b"epoch".to_vec()),
        Reply::Text(b"9007199254740993".to_vec()),
    ]);
    let fence = reply::admission(reply, &key, "epoch").unwrap().unwrap();
    assert_eq!(fence.sequence, 9_007_199_254_740_993);
}
