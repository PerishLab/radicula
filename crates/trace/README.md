# radicula-trace

Business-neutral local span records over `radicula-context`. This library owns
span completion and synchronous record delivery, without choosing an application,
collector, endpoint, persistence policy or global active session.

```rust
use radicula_context::Context;
use radicula_trace::{Ledger, Sink, note, run};
use std::sync::Arc;

let ledger = Arc::new(Ledger::default());
let context = Context::root().with(Sink::from(ledger.clone()));
let value = context.run(|| run("work", || {
    note("phase", "complete");
    42
}));
assert_eq!(value, 42);
assert_eq!(ledger.query(|record| record.says("phase", "complete")).len(), 1);
```

`run` preserves the body's output, including business errors. `carry` captures
the current Context, opens on first poll and ends on Ready, unwind or destruction.
Inner-future poll and destruction use that captured context; Ready destroys the
inner future before returning its output. An unpolled future opens no span.
Retaining Context views cannot keep an ended frame open. Threads and tasks
require explicit Context propagation; no executor or background task is created.

`Sink` stores an application-supplied `Engine`. A completed `Record` carries
local ID, optional parent ID, name, tags and opening/closing timestamps. IDs are
local process correlation, not a distributed trace identity. Missing sinks and
notes after completion are no-ops. `Ledger` is an in-memory queryable engine.

Engine callbacks run synchronously and must return promptly. Ordinary callback
panics are contained and reported independently to stderr; panic unwinding
clears the local frame without calling the engine. Delivery is not a persistence
acknowledgement. Blocking, process abort, exit and machine failure are outside
this local cleanup guarantee. No business outcome is inferred from cancellation,
panic or a returned value. Applications own export, storage and failure policy.

The optional `macros` feature exports `#[radicula_trace::span("work")]` for
ordinary synchronous and async functions or methods. Without it, the runtime
does not depend on proc-macro tooling. The attribute preserves supported
signatures, borrowing and returned errors; `const`, `unsafe`, `extern`, variadic
and `track_caller` functions refuse. Async construction alone activates no span;
execution uses the context where the async body first runs. The generated code
uses the canonical `radicula_trace` crate name.

Trace depends on Context within Radicula's closed Rust DAG. Both share the
repository release version; consumers must resolve one Context package identity
so stored Sink values and the active context are the same types and state.
