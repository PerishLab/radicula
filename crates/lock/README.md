# radicula-lock

Scoped synchronous and asynchronous execution coordination over memory and caller-opened files.
The runtime has no third-party dependencies. Existing
whole-file primitives remain available with their original signatures.

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
These carriers have no lease, renewal or forced execution timeout. A future
Redis carrier must declare its own supported protocol; these APIs do not imply
that arbitrary expiry and renewal settings already work.

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

Runtime-specific waiting, timers and managed blocking workers remain separate
pending capabilities in PerishLab/radicula#7. This package exposes no Tokio
feature yet. Immediate whole-file admission still performs the operating-system
file operation; the file resolver is synchronous and must not perform unbounded
work on an executor thread.

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
