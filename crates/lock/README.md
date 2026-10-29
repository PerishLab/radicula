# radicula-lock

Scoped execution coordination over memory, caller-opened files and Redis.
With neither `macros` nor `tokio`, the runtime has no dependencies. Method attributes
add a compile-time adapter. Existing whole-file primitives remain available
with their original signatures.

| API | Behavior |
| --- | --- |
| `lock(&File, Mode)` | Block until a shared or exclusive lock is acquired, or return an I/O error. |
| `attempt(&File, Mode)` | Acquire immediately or return `TryLockError::WouldBlock` or `TryLockError::Error(io::Error)`. |
| `unlock(&File)` | Explicitly release the lock and preserve any I/O error. |

`Mode` is `Shared` or `Exclusive`. `TryLockError` is re-exported from `std::fs`.
These functions retain the standard library's platform semantics and do not
translate failures into product-specific busy errors.

The caller opens the file and owns its path, access mode, permissions, ACL,
contents and lifetime. No API creates, truncates, replaces or removes a file.
A lock protects the opened file object; replacing its path can create another
object that is not protected by that lock. Locking is not a substitute for
checking file identity or securing paths and directories.

An explicit unlock may release the lock while the file remains open. Otherwise,
release follows the standard library's rules for the last duplicate or inherited
handle closing. Do not assume that dropping one clone releases a lock, or that
repeated locking, shared/exclusive conversion or upgrading is portable. This
primitive layer adds no reentrancy, timeout, fairness or crash-recovery guarantee.

On Windows, append-only handles cannot be locked: use a compatible read or
write access mode. Do not infer Unix advisory read/write behavior on Windows.
Filesystem and operating-system restrictions still apply, including network
filesystems. The caller must preserve the access mode required by its workload.

## Execution regions

`Factory` coordinates execution without interpreting business resources. A
memory factory is one domain; clone it to share that domain. Independent memory
factories do not coordinate, even when their keys match. A key is an exact,
nonempty string. Rebinding it with a different `Policy` within the same factory
fails. The factory retains its key bindings until the domain is dropped.

```rust
use radicula_lock::{Factory, Policy, Wait};
use std::time::Duration;

let domain = Factory::memory();
let jobs = domain.gate("background-jobs", Policy::limit(4).unwrap()).unwrap()
    .wait(Wait::timeout(Duration::from_secs(3)));
let result = jobs.guard(|| Ok::<_, &'static str>(42)).unwrap();
assert_eq!(result, Ok(42));
```

`guard` acquires before calling the closure and releases after it returns,
including a returned business error. Unwinding runs the permit's cleanup before
the panic continues. No business error or panic is suppressed or converted into
successful execution. The lock does not roll back business mutations or certify
business invariants after a failed execution.

The outer result reports admission or cleanup failure. `Failure::Release`
retains both the closure's output (including a business error) and the cleanup
error. If cleanup fails during unwinding or cancellation, the gate retains the
failure in `status().fault`; subsequent admission fails with `Error::Uncertain`.
An active count of zero alongside that fault does not confirm backend release.
There is no automatic backend fallback or automatic retry of the business body.

| Policy | Memory | Whole-file |
| --- | --- | --- |
| `Policy::EXCLUSIVE` | One execution | One execution across cooperating processes |
| `Policy::limit(n)` | At most n executions | Only n = 1; larger capacities are rejected |
| `Policy::Readers` | Compatible shared executions, exclusive writer | The same compatibility over an OS whole-file lock |

A readers gate defaults to exclusive admission. `gate.shared()` produces a
shared view of the same gate. Local readers share one OS acquisition; the last
reader releases it. Neither carrier promises FIFO fairness, reentrancy or lock
upgrades. A nested acquisition of the same exclusive key therefore needs an
explicit finite wait or restructuring of the execution regions.

`Wait::immediate()` reports `Error::Busy` without executing the body.
`Wait::timeout(duration)` bounds admission waiting using a monotonic clock; zero
allows one immediate attempt. `Wait::forever()` has no admission deadline. A
`Cancellation` token attached with `wait.cancel(token)` cancels admission, not
an already admitted execution. Waiting cancellation and external file contention
are checked at intervals of at most 10 ms while the thread/task is scheduled.
There is no wall-clock scheduling bound. Gate defaults are set by `gate.wait`;
`gate.with(wait, closure)` overrides them for one call.

An admission timeout is distinct from a lease expiry or an execution budget.
These carriers have no lease, renewal or forced execution timeout. The optional Redis carrier below has its own explicit lease protocol.

Synchronous `guard` protects execution of its closure. Returning a future only
constructs that future; it does not execute its body under protection. Child work
must finish within the region or own a transferred permit until it actually
finishes. Arbitrary detached work is outside the declared region.

## Asynchronous execution

`gate.asynchronous().guard(|| async { ... }).await` acquires before invoking
the closure and protects the future through execution and destruction. The
adapter owns a clone of the gate and works with any executor, including
non-`Send` and non-`Unpin` futures. It does not spawn tasks or require a runtime.

```rust
use radicula_lock::{Factory, Policy, Wait};

# async fn example() {
let domain = Factory::memory();
let jobs = domain.gate("background-jobs", Policy::limit(4).unwrap()).unwrap()
    .wait(Wait::immediate());
let result = jobs.asynchronous().guard(|| async { Ok::<_, &'static str>(42) }).await;
assert_eq!(result.unwrap(), Ok(42));
# }
```

This adapter currently accepts only `Wait::immediate()`. It rejects timeout and
unbounded waits explicitly with `Error::Unsupported`, rather than blocking the
executor. `gate.asynchronous().with(wait, closure)` overrides the gate default.
An already-cancelled admission token prevents execution. A pending body remains
protected until completion or cancellation by dropping its future.

The body future is destroyed before releasing its permit on normal completion,
panic and cancellation. Poll panics release protection before propagating even
when the executor retains the failed wrapper. Dropping an unpolled guard does
not acquire or invoke the closure. Rust abort, process termination and deliberately
forgotten futures do not run this cleanup. Business outputs and release failures
use the same `Failure` protocol as synchronous execution.

Select the explicit feature-gated Tokio adapter below for runtime waiting,
timers and managed blocking workers. Runtime-free immediate whole-file admission
still performs the operating-system file operation; the file resolver is
synchronous and must not perform unbounded work on an executor thread.

## Object lifetime and explicit ownership

`gate.hold(wait, initializer)` returns `Held<T>` and retains admission across the
returned object's lifetime. It destroys the value before releasing admission,
including when the value's destructor unwinds. `Held<T>` dereferences to the
value and offers `edit()` for an exclusive mutable borrow of that wrapper.
This wrapper does not synchronize other independent aliases of a business value.

`gate.acquire(wait)` returns an owned `Permit` for
explicit lifetime transfer. Dropping it performs cleanup automatically; callers
do not need a compensating unlock on errors. `permit.run(closure)` provides the
same scoped completion handling as `guard`, and `permit.attach(value)` ties
its lifetime to a value. `permit.during(|| async { ... })` transfers an already
acquired permit into an asynchronous execution. It invokes the future constructor
immediately under protection, so even dropping this execution without polling
destroys the constructed future before releasing admission.
`permit.release()` is available when the caller needs
an explicit cleanup result. Deliberately leaking/forgetting a permit or held
value prevents its normal destruction.

## Method attributes

Enable the `macros` feature to use the same protocol on a function or method:

```rust
use radicula_lock::{Gate, guard};

#[guard(gate = gate, error = |_| Err("coordination"))]
fn increment(gate: &Gate, value: usize) -> Result<usize, &'static str> {
    Ok(value + 1)
}
```

The gate expression is evaluated once and cloned before admission, retaining
the caller's domain and policy. The macro never constructs a new factory or
capacity. A function's signature and business return type `R` stay intact;
business errors, `?` and early returns remain within its protected body.
The required error handler receives `Failure<R>` and returns `R` only for
admission or release failure. Panic is propagated through normal cleanup.
The handler runs outside the business region and may inspect a retained output
on release failure. Call-site argument evaluation remains outside the body.

An async function uses the selected gate's asynchronous adapter. An ordinary
gate retains runtime-free immediate admission; selecting `gate.tokio()` uses
the feature-gated Tokio waiting and cleanup protocol. Dropping its unpolled future does not evaluate the
gate or run the body. Cancellation destroys its body future before releasing
admission. Const functions cannot use the attribute. The technical macro crate
shares this capability's version and release; its packaged README describes the
argument and evaluation contract.

## Whole-file factory

`Factory::files(resolver)` obtains caller-opened coordination files by key. It
never chooses business paths, opens a path itself, writes metadata into a file,
truncates its contents or removes it. For each key the factory caches one owned
handle. Concurrent first binding may invoke the resolver more than once; unused
handles are dropped. The resolver must safely return independent open handles
for the same stable file object, and distinct keys must not alias one underlying
lock owner through cloned handles. Cloning the factory shares its local state.

All participants must agree on key-to-file mapping and compatible policy. A
whole-file primitive cannot discover another process's policy configuration.
The resolver/adapter owns stable mapping, path/identity checks and safe opening.
External code must not independently lock or unlock a retained clone of the
factory's handle while a region is protected. Ordinary legacy participants
using separate handles can continue to use the existing primitives.

The factory explicitly unlocks on the last local permit, even when another
handle clone remains open. The platform and filesystem restrictions of the
primitive API still apply. Process exit/abort does not run Rust cleanup; recovery
then follows the OS handle and inheritance rules. Successful scoped cleanup is
not a promise that an inherited live handle can never prolong an OS lock.

## Redis execution regions

Enable `redis` for the synchronous `Factory::redis(config)` constructor. This
carrier adds no package dependency. It returns `redis::Factory`; its gates and
permits deliberately expose synchronous execution only. Ordinary method
attributes work with these gates; async method attributes, `asynchronous` and
`during` are unavailable for Redis. Do not call synchronous admission, cleanup
or `guard` on an executor thread. A future returned from a sync body is outside
its protected execution. Moving work to a thread requires that thread to retain
the permit until the body and its destruction finish.

```rust,no_run
use radicula_lock::{Factory, Policy, Wait, redis::Config};
use std::time::Duration;

let config = Config::new("127.0.0.1:6379".parse().unwrap(), "scheduler");
let domain = Factory::redis(config).unwrap();
let jobs = domain.gate("workers", Policy::limit(4).unwrap()).unwrap()
    .wait(Wait::timeout(Duration::from_secs(2)));
let output = jobs.guard(|| Ok::<_, &'static str>(42));
```

All participants must use the same server, database, namespace, key and protocol.
Independent factories and processes then share capacity. `Policy::Readers`
admits shared readers together and excludes an exclusive writer. Each permit
has its own remote identity; local aggregation never bypasses the remote count.
The policy binding and fencing sequence persist even when no lease remains.
Namespaces and keys are byte-exact UTF-8, hex-encoded without hash collisions.
Never delete or mutate the protocol's `radicula:lock:v1:*` keys while its domain
can execute. Caller business data is not stored or interpreted by this library.

The supported backend is Redis 7 or newer, one static primary endpoint, cluster
disabled and `maxmemory-policy noeviction`. The caller supplies an explicit
socket address, not DNS discovery. The transport is plaintext RESP over TCP;
optional password or ACL-user credentials are redacted from Debug. Configure a
trusted transport boundary as appropriate. Sentinel, replication failover,
cluster routing and TLS are unsupported. A single endpoint is not a proof of
safe replication or failover. Server configuration, ACLs and protocol-key
integrity must remain under the cooperating domain's control.

A factory starts one managed renewal worker. Default lease duration is 30 seconds,
renewal interval 5 seconds, complete RPC budget 100 milliseconds and local active
lease budget 32. `Config::lease(duration, renewal)`, `io(duration)` and `slots(n)`
set these independently; `None` disables renewal. Validation requires a lease
at least four complete IO cycles across the local slot budget, and renewal at
most one quarter of the lease. The local slot budget is an additional admission
limit, not the domain's remote capacity. Every RPC budget includes connection,
authentication, database selection, command submission and reply. Replies and
commands are bounded to 64 KiB; arrays and nesting are bounded as well.

Admission waits and cancellation govern retries before execution. One in-flight
RPC can consume its entire IO budget beyond a wait deadline or cancellation;
there is no wall-clock executor scheduling bound. Lease duration governs remote
ownership, not execution time. There is no automatic execution timeout or forced
termination of a Rust closure. Renewals compare server epoch, owner identity and
unexpired membership; they never recreate an expired lease. Correct lease timing
requires the server's wall clock to advance consistently with the client's
monotonic clock. Arbitrary clock jumps, suspend and unbounded scheduling delay
cannot establish execution exclusivity through a time lease alone.

`permit.check()` and `permit.fence()` refuse once the local lease is known lost.
A failed or late renewal marks it lost permanently. The gate retains a fault and
refuses further local admission; it never silently falls back to memory or files.
A submitted admission without a valid reply returns `Error::Unconfirmed` and
never invokes the body. Remote admission may nevertheless have happened; its
lease expires without renewal. Failed release is observable through the guard's
`Failure::Release` and `status().fault`; the business output is retained. Drop
attempts bounded cleanup, including during unwinding, and never renews a permit
once release starts. It cannot turn an unconfirmed remote release into success.

Lease loss does not stop old work. That work may still mutate business state
while another participant has a new lease. A fencing-aware destination must
check the supplied `Fence` if the workload requires rejection of stale writes.
The server generates each identity with its run epoch and an exact integer
sequence; sequences are not converted through Lua floating-point numbers.
`Fence::follows` compares sequences only within one epoch and refuses across
epochs. Redis restart changes its epoch, invalidating existing factories and
permits; recovery requires explicit creation of a new factory after review.
There is no globally monotonic fence across restart, data loss or failover.

The scripts execute atomically relative to other Redis commands, but a script
error does not certify rollback. Protocol corruption or resource exhaustion can
leave partial remote state; such a result is unconfirmed and cannot start work.
Key metadata is persistent and scripts purge expired entries on admission.
Operators must bound participating clients and capacity; this is not a promise
of bounded server execution time for an unlimited shared-reader population.

## Tokio execution

Enable `tokio` and select `gate.tokio()` explicitly. The existing runtime-free
`asynchronous()` adapter retains its immediate-admission contract. The Tokio
adapter supports admission waiting, cancellation and timeout, and uses Tokio
for its timers and blocking business tasks. It rejects admission outside a
running Tokio runtime. Its async `guard` accepts local non-`Send` futures.

Bind file gates before entering the async execution region: factory and gate
configuration remains synchronous. File attempts and cleanup run on a
library-owned carrier worker, so those OS
operations never run on the executor. Dropping an execution destroys its body
before queuing permit cleanup. The library retains the worker and permit until
that cleanup runs, including when the caller cancels or the Tokio runtime has
already gone away. A cancelled in-flight admission cannot later run the body;
any late permit is returned through the same owned cleanup protocol. Destruction
cannot await an OS operation: the gate remains active until actual cleanup, and
release failures remain in `status().fault`. Ordinary successful `guard` awaits
release and retains business output alongside any release failure.

`adapter.blocking(wait, closure).await` transfers the permit into the actual
blocking worker. Cancelling or timing out its outer future does not terminate
that worker and does not release its permit early. The closure and its cleanup
finish before the permit ends. Business panics propagate through normal cleanup.
An admission timeout is separate from a business execution deadline and does
not make an OS operation interruptible.

An acquired Tokio permit supports `during`, `attach` and asynchronous `release`.
An async method attribute can select the same adapter with
`#[guard(gate = gate.tokio(), error = handler)]`; its gate expression is still
evaluated once. Redis remains a separate synchronous carrier and has no Tokio
adapter in this release.
