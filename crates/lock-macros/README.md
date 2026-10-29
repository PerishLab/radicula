# radicula-lock-macros

The procedural adapter for the lock capability. Consume it through
`radicula-lock` with its `macros` feature; both packages share one repository
version and release. This crate owns no runtime or backend protocol.

`#[radicula_lock::guard(gate = expression, error = handler)]` wraps a function
or method in the existing gate's synchronous or asynchronous guard. The
function's signature, business return type and attributes remain intact.
Business errors, `return` and `?` retain their meaning inside the protected
body. Panic continues through the runtime's normal cleanup and is never sent
to the error handler. Argument evaluation remains ordinary call-site work outside
the guarded body.

The gate expression is evaluated once, before admission, and its gate is cloned
to retain the caller's existing domain. The expression must yield a `Gate`;
use its configured policy and wait, or explicitly obtain it from a retained
factory and key. The macro creates no factory, global domain, connection,
runtime or capacity of its own. Methods and direct factory calls coordinate
only when they share the same domain, key and compatible policy.

The error expression is required and evaluated only when admission or release
fails. It receives `Failure<R>` and must return the function's declared `R`.
This explicit mapping leaves refusal and release uncertainty with the caller;
successful business outputs do not pass through it. A release failure retains
the original business output. The handler runs outside the declared business
body; a release failure does not certify backend release.

Async functions use the runtime-independent adapter and can run on Tokio or
another executor. They currently require `Wait::immediate()`; unsupported waits
reach the same explicit error handler. Body future destruction precedes release
on completion, panic and cancellation. Dropping an unpolled method future does
not evaluate its gate or enter its body. Runtime-specific waiting and worker
support remain the runtime library's responsibility. Const functions are rejected.
