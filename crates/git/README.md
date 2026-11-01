# radicula-git

Git contexts and lossless command results, independent of a process runner.
The runtime uses only the standard library. It has no dependency on another
Radicula capability or product library. Queries require Git support for
`--path-format=absolute` and `--end-of-options`; unsupported versions retain
their actual failure. Platform tests qualify the Git installed there.

```rust,no_run
use radicula_git::{Git, Reply};

let git = Git::new("/usr/bin/git", "/work/source");
let head = Reply::new(git.head().output()?).object()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The caller resolves and authenticates the executable, chooses the repository
directory and runs each returned `std::process::Command`. Neither path is
canonicalized or discovered here. Commands preserve the launch path and its
invocation-name semantics. Standard commands compose with Tokio's `Command::from`
without a runtime dependency here.

| Surface | Contract |
| --- | --- |
| `Git::new`, `command` | Explicit executable and working directory; ordinary Git arguments remain available. |
| `common`, `directory`, `bare` | Absolute common-dir, absolute Git directory, and bare-state query commands. Common-dir identity is not worktree, lock or ownership. |
| `head`, `tree`, `ancestor` | Commit/tree identity and ancestry query commands. Revisions follow an option terminator. |
| `changes(Source)` | NUL-framed name/status for staged, worktree or revision-range differences. |
| `Reply::data`, `path`, `object`, `flag`, `changes` | Require success before decoding; failure retains the full exit status and both raw output streams. |
| `Reply::ancestry` | Exit 0 is true, 1 is false; every other exit or signal is an error. |
| `detach`, `detached` | Apply the exact `LOCAL` environment policy to a command, or construct one. |
| `CONTRACT`, `POLICY` | Packaged versioned policy and compatibility vectors for adapters and evidence. |

## Environment

`LOCAL` removes inherited repository/object/index selection, command-level Git
configuration injection, replacement/graft/shallow views, namespace and
repository-discovery overrides. It includes Plumb's former sixteen-variable
policy and additionally clears `GIT_NAMESPACE`, `GIT_CEILING_DIRECTORIES` and
`GIT_DISCOVERY_ACROSS_FILESYSTEM`. This differs from Concord's eight-item list;
migrating consumers must treat it as a behavior change.

Call `detach` before setting owned overrides. It removes every named key,
including an override already placed on the command. A caller can then set a
deliberate index, object directory, alternate store, namespace or configuration
input. The library never clears those inputs again during execution and never
changes the host process.

SSH and credential inputs, author/committer identity, global/system configuration
selection, PATH/PATHEXT and unrelated variables remain inherited. This does not
ignore repository hooks, global Git configuration or credentials. Callers needing
allowlisted environments supply that policy themselves. Detachment is not a
sandbox, authorization, hermetic execution or a trusted tool fingerprint.
Record the exact package and policy in evidence; the policy identifier alone
does not identify every future implementation.

## Bytes and lifecycle

Path decoders remove one output line terminator, preserving embedded newlines
and leading/trailing spaces. Unix paths retain arbitrary non-NUL bytes. Other
platforms require Git's UTF-8 representation and reject invalid UTF-8. No lossy
conversion is performed. NUL diff records retain rename/copy source paths and
optional scores. Empty, malformed, truncated or unrecognized records refuse.
`object` accepts canonical lowercase 40- or 64-digit hexadecimal identities;
product protocols can apply stricter rules.

Error display does not echo raw stderr. The caller owns redaction, diagnostics,
time/output budgets, stdin, cancellation and child reaping. This library does
not acquire locks, construct candidates, authorize writes or recover product
transactions. Preserve those guarantees when composing execution mechanisms.
