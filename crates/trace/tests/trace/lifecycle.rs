use radicula_context::Context;
use radicula_trace::{self as trace, Ledger, Sink};
use std::sync::Arc;

#[test]
fn snapshots() {
    let ledger = Arc::new(Ledger::default());
    let root = Context::root().with(Sink::from(ledger.clone()));
    let saved = root.run(|| {
        trace::run("parent", || {
            trace::note("phase", "before");
            Context::current()
        })
    });
    assert_eq!(ledger.query(|_| true).len(), 1);
    saved
        .clone()
        .with(42_u32)
        .run(|| trace::note("phase", "after"));
    saved.run(|| trace::note("late", "ignored"));
    let seen = ledger.query(|_| true);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].tags, vec![("phase".into(), "before".into())]);
}

#[test]
fn delayed() {
    let ledger = Arc::new(Ledger::default());
    let root = Context::root().with(Sink::from(ledger.clone()));
    let child = root.run(|| {
        trace::run("parent", || {
            trace::carry("child", async {
                trace::note("child", "ran");
            })
        })
    });
    let parent = ledger.query(|_| true);
    assert_eq!(parent.len(), 1);
    super::support::block(child);
    let child = ledger.query(|record| record.name == "child");
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].parent, Some(parent[0].id));
    assert!(child[0].says("child", "ran"));
}

#[test]
fn reentry() {
    let ledger = Arc::new(Ledger::default());
    let root = Context::root().with(Sink::from(ledger.clone()));
    root.run(|| trace::run("first", || {}));
    let seen = ledger.query(|_| {
        root.run(|| trace::run("next", || {}));
        true
    });
    assert_eq!(seen.len(), 1);
    assert_eq!(ledger.query(|_| true).len(), 2);
}
