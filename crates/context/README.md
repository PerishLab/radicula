# radicula-context

Business-neutral execution context with explicit synchronous and asynchronous
propagation. This strict root library knows no product, resource, trace, cancel
signal, configuration, executor or serialization protocol.

```rust
use radicula_context::Context;

struct Request(u64);
let root = Context::root();
let request = root.with(Request(7));

let value = request.run(|| Context::current().get::<Request>().unwrap().0);
assert_eq!(value, 7);
assert!(Context::current().get::<Request>().is_none());

let future = request.carry(async {
    Context::current().get::<Request>().unwrap().0
});
```

The caller polls or awaits `future` through its own executor. Constructing it
starts no task and reads no environment. A synchronous `run` returning a future
activates context only during construction; use `carry` for execution.

## Values

`root` and `Default` create an empty context. `with<T>` returns a derived
context; `get<T>` returns `Option<Arc<T>>`. Values must be `Send + Sync + 'static`.
Each type names one slot, and the nearest value shadows earlier values of the
same type. Derivation and cloning leave every earlier view intact and share
values without requiring `T: Clone`. Lookup walks the retained parent chain.
Data mutation and synchronization remain the value owner's responsibility.

Use distinct wrapper types for distinct slots. An `Option<T>` stored as a value
can deliberately shadow an inherited `Some` with `None`; absence of the slot
and a present empty value remain distinct. Domain payloads keep their own
identity, validation, merging and serialization laws outside this library.

## Synchronous execution

`current` captures the active context of the calling thread. `run` activates the
selected context for exactly its synchronous closure, then restores the prior
one on normal return, a returned business error, or panic unwinding. Output and
errors pass through unchanged. Calls may nest and re-enter `current` or `run`.
No public enter guard can escape or be released out of order.

Threads and tasks do not automatically inherit context. Capture it explicitly
and call `run` in the other thread, or wrap the task with `carry` before spawning.

## Futures and destruction

`carry<F: Future>` returns a concrete `Carry<F>` without boxing or a task runtime.
Every poll activates the captured context and restores the caller's context
before returning `Pending`, `Ready`, or unwinding. A completed inner future is
destroyed immediately, under that captured context, before its output returns.
The output then belongs to the caller; its later destruction is not scoped by
this wrapper.

Dropping an unfinished wrapper also activates the captured context while
its inner future is destroyed, including when it was never polled. Cancellation
is just one reason a future may be dropped; this library assigns no business
outcome to it. Pinned inner futures are destroyed in place. A completed inner
future is never destroyed twice, even if its destructor unwinds. Polling the
wrapper again after completion panics.

`Context` is `Send + Sync`. `Carry<F>` follows the actual `Send`, `Sync`, and
`Unpin` properties of `F`; it does not impose extra bounds on the future's output.
The wrapper can move between executor threads when those types permit it.

Ordinary user panics propagate after restoring the prior context. User
panic hooks, destructors and payload behavior keep Rust's own rules; this is
not a panic sandbox. Abort, process exit, machine loss, and forgotten values do
not run language cleanup.

## Identity and scope

Use one resolved package instance in a cooperating application. Separate
versions or dynamic-library boundaries do not promise one TLS or compatible
Rust type identity. There is no global engine, automatic baggage discovery,
string-key registry, deadline, wire format or cross-process propagation.

Pinned destruction uses `pin-project-lite` 0.2.17 or a compatible later version.
The requirement is a minimum, not an exact version pin. The package shares
Radicula's repository version and release. It has no domain
or sibling-capability dependencies, including its development and platform
branches. It provides no old Plumb path, alias or compatibility facade.
