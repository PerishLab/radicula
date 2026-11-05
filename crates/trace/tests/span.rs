use super::support;

use radicula_context::Context;
use radicula_trace::{self as trace, Ledger, Sink, span};
use std::rc::Rc;
use std::sync::Arc;

#[span("work.step")]
fn step(count: u32) -> u32 {
    trace::note("count", &count.to_string());
    count + 1
}

#[span("work.flow")]
async fn flow() -> u32 {
    support::rest().await;
    step(1)
}

#[span("work.pick")]
#[inline]
fn pick<'a, T>(value: Option<&'a T>, other: &T) -> Result<&'a T, &'static str> {
    let value = value.ok_or("empty")?;
    if std::ptr::eq(value, other) {
        return Err("same");
    }
    Ok(value)
}

struct Work(Rc<String>);

impl Work {
    #[span("work.read")]
    fn read(&self) -> &str {
        &self.0
    }

    #[span("work.borrow")]
    async fn borrow(&self) -> &str {
        support::rest().await;
        &self.0
    }
}

#[span("work.recurse")]
fn recurse(depth: u8) -> u8 {
    if depth == 0 {
        return 0;
    }
    1 + recurse(depth - 1)
}

#[test]
fn wraps() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    context.run(|| {
        assert_eq!(step(1), 2);
    });
    let seen = ledger.query(|record| record.name == "work.step" && record.says("count", "1"));
    assert_eq!(seen.len(), 1);
}

#[test]
fn weaves() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    assert_eq!(support::block(context.carry(flow())), 2);
    let flows = ledger.query(|record| record.name == "work.flow");
    let steps = ledger.query(|record| record.name == "work.step");
    assert_eq!(flows.len(), 1);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].parent, Some(flows[0].id));
}

#[test]
fn borrows() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    let first = String::from("first");
    let second = String::from("second");
    let work = Work(Rc::new(String::from("borrowed")));
    context.run(|| {
        assert!(std::ptr::eq(pick(Some(&first), &second).unwrap(), &first));
        assert_eq!(pick(None, &second), Err("empty"));
        assert_eq!(pick(Some(&first), &first), Err("same"));
        assert_eq!(work.read(), "borrowed");
    });
    assert_eq!(support::block(context.carry(work.borrow())), "borrowed");
    assert_eq!(ledger.query(|_| true).len(), 5);
}

#[test]
fn recurses() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    assert_eq!(context.run(|| recurse(3)), 3);
    let seen = ledger.query(|_| true);
    assert_eq!(seen.len(), 4);
    assert_eq!(
        seen.iter().filter(|record| record.parent.is_none()).count(),
        1
    );
    assert!(
        seen.iter()
            .filter_map(|record| record.parent)
            .all(|parent| { seen.iter().any(|record| record.id == parent) })
    );
}

#[test]
fn construction() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    let future = context.run(flow);
    assert_eq!(support::block(future), 2);
    assert!(ledger.query(|_| true).is_empty());
}
