# radicula-lock

Whole-file locks over a caller-owned `std::fs::File`, using Rust's standard
library primitives. The crate has no dependencies.

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
crate adds no reentrancy, timeout, fairness or crash-recovery guarantee.

On Windows, append-only handles cannot be locked: use a compatible read or
write access mode. Do not infer Unix advisory read/write behavior on Windows.
Filesystem and operating-system restrictions still apply, including network
filesystems. The caller must preserve the access mode required by its workload.
