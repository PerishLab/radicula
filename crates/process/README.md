# radicula-process

Explicit tool identity, environment snapshots and owned process execution.
This is an independent library root. It depends on no Git, lock, governance,
identity or observation product. The default `tokio` feature includes the process
runner. Tool-only consumers can disable default features and use synchronous
command construction and inspection without a Tokio dependency.

## Command contexts

`Search::new(root, path, extensions)` captures caller-supplied inputs. It does
not read ambient PATH/PATHEXT or discover a home directory. The search root is
resolved once. `resolve` preserves the executable's launch path rather than
canonicalizing it. A rustup proxy named `cargo` must still launch as `cargo`.
`command` also applies the captured directory and PATH/PATHEXT.

`Tool::at(path)` captures SHA-256 bytes while retaining that exact launch path.
`content()` separately resolves the content location. `verify()` checks current
bytes. The serde projection remains exactly `path` and `digest`, with unknown
fields refused on decode. Native paths can be non-Unicode; a serialization
format unable to represent one must fail instead of substituting lossy text.
A digest does not authenticate an issuer or close the race between verification
and execution.

`Search::bind(program)` retains the lookup context and captured tool. A binding's
`verify` checks both resolution and bytes; `command` verifies before constructing
a standard command with the bound launch path. A caller can mutate a returned
command, so this is an observation boundary, not an execution warrant.

`Environment::new(values)` captures only supplied native name/value pairs. It
selects no language policy, registry, credential or configuration source.
`apply(command)` replaces the child's environment with that snapshot. Apply it
before policy transformations and owned overrides. It deliberately does not
reconstruct past command mutations or preserve earlier explicit overrides;
standard command introspection cannot recover every prior removal after
`env_clear`. Duplicate-key behavior follows the standard command builder.
Debug output gives only the entry count; `values()` is an explicit data access.
The host environment is never modified.

Products compose these mechanisms with their language rules and Git policy
through standard `Command`, `Output`, `OsStr` and `Path`, without capability
backedges. The caller owns final command agreement, credentials, evidence,
locks and mutation authority.

## Execution with `tokio`

```rust
use radicula_process::Cancel;
use radicula_process::asynchronous::{Limits, Runner};
use std::{process::Command, time::Duration};

let runner = Runner::new(Limits { time: Duration::from_secs(5), output: 65536 })?;
let mut command = Command::new("git");
command.arg("--version");
let mut session = runner.capture(command, Vec::new(), Cancel::new());
let observer = session.observer();
let output = session.output().await?;
```

`Runner::start(command, Streams::new(input, stdout, stderr), cancel)` takes owned,
`Send + Unpin + 'static` asynchronous streams. It replaces the command's stdio
with pipes, concurrently copies input and drains both output streams, closes
child stdin at EOF, and flushes output sinks without shutting them down. The
completion returns those same streams, allowing the caller to inspect or reuse
them. Capture uses an owned byte cursor and byte vectors. Borrowed synchronous
streams require an owning adapter or a caller refactor; they are not silently
moved to detached worker threads.

`start` and `capture` require an entered Tokio runtime, like `tokio::spawn`.
Synchronous callers use `blocking`.

The nonzero child exit status is a normal completion, preserving stdout/stderr;
the caller decides business success. The required nonzero time budget starts
when the driven task begins and bounds its process/I/O phase. One shared output
budget counts bytes observed across stdout and stderr, before forwarding to
sinks. Input has no implicit byte limit. Sinks must obey the asynchronous polling
contract. Expiry, cancellation, I/O failure and output overflow stop the owned
child and wait for it. Cleanup is separate from the execution budget and depends
on the OS. No process-tree containment or hard real-time guarantee is made.

`Session::wait` borrows the session. Dropping that waiting future keeps the
session and join result available; it does not imply cancellation or completion.
A cloned `Cancel`, `Session::cancel`, or dropping the session requests cancellation.
A driven Tokio runtime then performs cleanup. Retain an observer and await
`finish()` to distinguish `Settled` from `Uncertain`. Dropping a session does not
synchronously certify that a process was reaped.

An operation failure returns the same stream values, any observed child status,
and a distinct cleanup error. When cleanup fails the child handle is retained
for explicit caller disposition and the observer remains `Uncertain`. A task
panic or runtime shutdown yields an interrupted/lost result, not successful
cleanup; owned child kill-on-drop is best effort there. The caller must keep
its business custody until its required cleanup facts are confirmed. This
library does not coordinate or release another capability's permits.

`blocking(command, bytes, cancel)` drives a private current-thread runtime and
returns the captured standard output. It refuses an already entered Tokio
runtime; asynchronous callers use sessions. Both forms use the same I/O and
cleanup implementation. The runner records no global ledger and retries no
business operation.
