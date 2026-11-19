# Radicula Document for JavaScript

A typed document client for ESM and TypeScript. It reads JSON values at RFC 6901
pointers and maintains an immutable confirmed snapshot plus pending changes.
The host supplies document access, identity, transport and the optimistic
projection. The package follows Radicula's shared release version.

```ts
import { connect, type Options, type Transport } from "@perishlab/radicula-document";

function preferences(
  transport: Transport<string>,
  project: Options<string>["project"],
  system: () => string,
) {
  const client = connect({ base_rel_path: "/preferences", transport, project });
  return { client, theme: client.string("/settings/theme", system) };
}
```

Pass a document key and an opaque host identity epoch to
`client.select({ key, identity })`. This loads the matching document.
`client.select(null)` clears the snapshot and pending state. The transport
receives the binding and `base_rel_path`. An omitted base is `/`.
Each client holds one active document. Consumers can create independent clients
for different documents.

`string`, `integer`, `number`, `boolean` and `value` make typed read handles.
A handle's `read()` exposes a distinct loading, failure, uncertainty, missing or
value result. The optional default callback runs when the client is signed out
or when the selected path is missing in a ready document. The callback is lazy
on each read. A host may rerun the read when system preferences change. An empty
string, false, zero and null are legitimate values. A typed handle reports a
type error if a present value or the callback result is incompatible.

`parse`, `document` and `read` are standalone JSON helpers. The pointer `""`
addresses the root. Dots are literal in keys, with `~1` and `~0` for slash and
tilde. Arrays accept canonical decimal indices. `read` distinguishes a missing
path from a present null; extending a path through a scalar raises a fault.
The package checks the same finite number and safe integer domain used by the
Rust document protocol. Accepted snapshots and projections are immutable.

## Updates

Call `client.update(intent)` on a ready document. `project` computes an
optimistic document synchronously. Treat passed intents as immutable. Keep the
projection pure because it is
reapplied when a server response changes the confirmed state. `subscribe`
notifies the host on the next microtask. `client.state.document` is the
optimistic document and `client.state.confirmed` is the last confirmed
`{ revision, document }` pair.

Writes run one at a time using the last confirmed revision. The transport
returns one outcome:

- `saved` supplies the server's new snapshot. It becomes the confirmation and
  pending intents are projected over it.
- `rejected` rolls back that intent and allows queued intents to proceed.
- `conflict` represents a definitive revision conflict. The client reads the
  new snapshot before sending later intents.
- `unknown` retains the queue and pauses writes. A thrown transport error or
  invalid success envelope also enters this state. Re-selecting the same
  binding or calling `refresh` leaves it paused.

A read failure is visible in `client.state`. Call `refresh` to retry a read.
Switching keys or identities creates a fresh binding. Late completions from
the old binding have no effect on the new snapshot. The host chooses how to
present transport and projection errors and when to re-establish a binding.

## Folio adapter

Use Folio's private preferences endpoint in the host transport. Translate a
successful response directly to `Snapshot`, map 409 to `conflict`, and map a
definite refusal to `rejected`. Authenticated requests and device preference
belong to the host. The path `/app/items/theme.tone/effective` reads the
application view, while `/shared/items/theme.tone/effective` reads the shared
view. A host can bind a handle to a declaration's `value` path when it wants
missing declarations to call its local default.

The SDK supplies document state. The Folio adapter decides which declaration
or server-evaluated view to display, and which optimistic projection is suitable
for an intent.
