use super::support;
use radicula_context::Context;
use radicula_trace::{self as trace, Ledger, Sink};
use std::sync::Arc;

mod fault;
mod future;
mod lifecycle;
#[path = "support.rs"]
mod probe;

#[test]
fn emits() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    assert_eq!(context.run(|| trace::run("turn", || 42)), 42);
    let seen = ledger.query(|record| record.name == "turn");
    assert_eq!(seen.len(), 1);
    assert!(seen[0].parent.is_none());
    assert!(seen[0].closed >= seen[0].opened);
}

#[test]
fn silent() {
    let result = Context::root().run(|| {
        trace::run("orphan", || {
            trace::note("seen", "true");
            Err::<(), _>("business")
        })
    });
    assert_eq!(result, Err("business"));
    let future = Context::root().run(|| trace::carry("silent", async { 9 }));
    assert_eq!(support::block(future), 9);
}

#[test]
fn lineage() {
    let ledger = Arc::new(Ledger::default());
    Context::root().with(Sink::from(ledger.clone())).run(|| {
        trace::run("turn", || trace::run("effect.dispatch", || {}));
    });
    let turns = ledger.query(|record| record.name == "turn");
    let effects = ledger.query(|record| record.name == "effect.dispatch");
    assert_eq!(effects[0].parent, Some(turns[0].id));
}

#[test]
fn notes() {
    let ledger = Arc::new(Ledger::default());
    Context::root().with(Sink::from(ledger.clone())).run(|| {
        for reason in ["advanced", "operator_resolved", "advanced"] {
            trace::run("effect.dispatch", || trace::note("effect.reason", reason));
        }
    });
    let advanced = ledger.query(|record| record.says("effect.reason", "advanced"));
    let resolved = ledger.query(|record| record.says("effect.reason", "operator_resolved"));
    assert_eq!(advanced.len(), 2);
    assert_eq!(resolved.len(), 1);
}

#[test]
fn carries() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    let task = context.run(|| {
        trace::carry("turn", async {
            trace::note("phase", "worked");
            support::rest().await;
            Err::<(), _>("business")
        })
    });
    assert_eq!(support::block(task), Err("business"));
    let seen = ledger.query(|record| record.name == "turn" && record.says("phase", "worked"));
    assert_eq!(seen.len(), 1);
}
