# radicula-document

One public JSON document protocol: an owned value tree, exact path addresses,
explicit presence and ordered atomic edits. Values are JSON as `serde_json`
defines them; this library adds addressing and mutation laws rather than a
second syntax, query language, storage, authorization or business evaluation.

```rust
use radicula_document::{Document, Operation, Path, Presence};

let mut held = Document::parse(r#"{"a":1,"list":[10,20]}"#)?;
let list = Path::parse("/list/1")?;
assert_eq!(held.read(&list)?, Presence::Present(&20.into()));

held.set(&Path::parse("/b")?, true.into())?;
held.remove(&Path::parse("/a")?)?;
assert_eq!(held.text(), r#"{"b":true,"list":[10,20]}"#);

let steps = [
    Operation::Set { path: Path::parse("/b")?, value: 2.into() },
    Operation::Remove { path: Path::parse("/gone")? },
];
let fault = held.apply(&steps).unwrap_err();
assert_eq!(fault.index, 1);
assert_eq!(held.text(), r#"{"b":true,"list":[10,20]}"#);
# Ok::<(), radicula_document::Error>(())
```

## Values

`Value` is re-exported `serde_json::Value`: null, boolean, string, number,
array and object. `Document::parse` refuses invalid JSON with `Error::Syntax`
and `Document::new` accepts an already built value. Both refuse values outside
the exact number domain. `root`, `text` and `take` read the whole value. `text`
is `serde_json`'s compact serialization: it round-trips this crate's own values,
but it is not a canonical byte form across other `serde_json` feature sets or
languages, because key order and float printing follow the build's map type and
number handling.

Documents are values, not handles. `Clone` yields an independent tree, and no
call shares hidden state with another document or process.

## Paths

`Path` is an RFC 6901 JSON Pointer over the document itself. `""` addresses the
root, `"/a/b"` the key `b` of the object under `a`, and `"/"` the single empty
key. Segments are literal: a dot carries no meaning, so `x.y` addresses the key
`x.y` and never descends. `~0` spells `~`, `~1` spells `/`, and any other `~`
sequence is `Error::Escape`. Text that is not empty and does not start with `/`
is `Error::Pointer`.

An array segment is a canonical decimal index: no sign, no leading zero unless
it is `0`, and no `-` append spelling. It is `Error::Index` otherwise. Because a
path addresses exactly one existing location, this protocol has no wildcard,
filter, projection or relative form. Query expressions belong to a caller's own
language and never appear here.

## Presence and reading

`read` returns `Presence::Present(&Value)` or `Presence::Missing`. A present
`null` is `Present(null)`, so absence and an explicit null never merge; the same
separation holds for `false`, `0`, `""`, `[]` and `{}`. `Presence::value` keeps
them apart too: `None` means absent, while an explicit null arrives as
`Some(&Value::Null)`, never as `None`.

A path that continues through a scalar, or through `null`, is
`Error::Kind`. An object key that is absent, and an array index that is
canonical but not addressable — at or past the length, or too large for any
`usize` — are both `Missing`. Syntax and extent stay separate: a segment that is
not a canonical index is `Error::Index` whatever the array holds, so no platform
word size changes the protocol.

## Writing

`set` binds an existing location or replaces its value, including with `null`.
An absent object key at the final segment is created. An absent intermediate
segment, or an array index that is canonical but not addressable, is
`Error::Missing`: this
protocol never invents containers or extends arrays, so a caller states the
shape it means. A final segment over a scalar is `Error::Kind`. The empty path
replaces the whole document.

`remove` deletes an existing object key or array element and shifts later array
elements down. An absent location, including a canonical but unaddressable array
index, is `Error::Missing`; a location under a scalar or `null` is
`Error::Kind`; the root is `Error::Root`. Removing a key is distinct from
writing `null` there.

Writing and reading treat keys as data, not as JavaScript properties: `set`
stores an own property, so `"/__proto__"` and `"/constructor"` are ordinary keys
and never reach a prototype.

`apply` runs operations in the given order against the same document. It
returns the zero-based `Fault { index, error }` of the first refusal and leaves
the document at its pre-call value, so a batch is all or nothing. An empty
batch succeeds unchanged. Operations are absolute: each path is read against
the document as earlier operations left it, and no operation compensates an
earlier one. A `Set` on the empty path replaces the value every later operation
addresses, so the remainder of a batch sees the new root.

## Numbers

The exact domain is every integer in `|n| <= 9007199254740991`: the largest
value at which IEEE-754 binary64 and JavaScript still add and remove one. This is
the safe-integer bound, not the largest representable binary64 integer.
Integers outside it are `Error::Number`, however they are spelled:
`9007199254740992`, `1e20`, `9007199254740992.0` and `-1e20` all refuse. An
integral value inside the domain written with a fraction or exponent, such as
`9007199254740991.0`, is kept.

Values with a fractional part are kept as the binary64 number the text denotes,
which is what `serde_json` parses and prints, so they are not kept as written:
`1e-400` underflows to `0.0`, and `1E2` becomes `100.0`. `1e400` overflows
binary64 and is `Error::Syntax`. Fractional precision beyond binary64 remains
the caller's concern; this protocol promises the integer domain, not decimal
arithmetic.

`Document::parse`, `Document::new`, `set` and each applied `Set` enforce this
domain, so no accepted document or accepted write holds an integer whose value a
web consumer would silently change.

## Errors

`Error` has the stable codes `escape`, `index`, `kind`, `missing`, `number`,
`pointer`, `root` and `syntax`, exposed by `code`. `Display` adds prose.
Callers match the variant; they never parse text. `apply` distinguishes
positions by `Fault::index`.

## Conformance

`spec/conformance.json` is the shared case file: name, document text,
operation, and expected presence, value, error, fault index, document or text.
Expectations stay on semantics a second implementation can meet: presence,
values, error codes, the fault index and the resulting document tree. Byte-exact
`text` is compared only where the case names the serialization it fixes.
`tests/spec.rs` runs every case through this crate and refuses a case name,
declared error code or expectation the implementation does not meet.

The identity of this surface is public and implementation-free: the protocol is
`radicula.document.conformance/v1`, and `spec/conformance.json` in the released
crate is the case file any implementation can bind to, whatever language it is
written in. Binders cite that name and that file; they do not need a library
from this repository, and this crate does not depend on any binder.

For this stage, cross-language evidence is a Node run of the same 53 cases
against an independent JavaScript implementation of these laws. Its result and
the case file digest are recorded as evidence on the owning Issue, alongside the
Rust run of the same file. That run is evidence about the protocol, not about an
application: it claims no product consumption, no packaged JavaScript artifact
and no front-end adoption. Web work that consumes Folio operation results is a
consumer concern with its own acceptance, not a condition of this crate.

## Compatibility

The package shares Radicula's repository version and release. The wire file is
`radicula.document.conformance/v1`; a future version that changes accepted
values, path decoding, presence, refusal codes or batch atomicity publishes a
new name rather than reinterpreting this one. `Error::code` values are part of
that surface.

No dependency on Keel, Folio or another product defines this root protocol. It
adds no storage, transaction, permission, schema, diff, patch, CLI, service or
compatibility facade, and it does not read the environment.
