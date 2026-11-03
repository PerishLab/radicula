# radicula-template

Two explicit text substitution protocols over caller-owned variables.
The library uses only the standard library and has no dependencies, features,
IO, environment reads or expression execution. It is independent of every
other Radicula capability and domain product.

```rust
use radicula_template::{fill, marked};
use std::collections::BTreeMap;

let vars = BTreeMap::from([("name", "world".to_string())]);
assert_eq!(fill("hello {name}", &vars)?, "hello world");
assert_eq!(marked("${{ matrix.os }} {@name}", &vars)?, "${{ matrix.os }} world");
# Ok::<(), radicula_template::Error>(())
```

`fill` replaces `{name}`, turns `{{` and `}}` into literal braces, and refuses
unknown names, an unclosed placeholder, an empty name or a bare closing brace.
`marked` replaces only `{@name}` and leaves other characters, braces and
expression languages untouched. It refuses unknown, empty or unclosed marked
placeholders. Names match exactly, including whitespace and Unicode.

Both operations substitute each input placeholder once. Inserted values are
literal text, including empty strings and braces; they are never parsed again.
Callers own template selection, variables, resource names, escaping for their
output medium and any interpretation of the rendered text.

`Error` retains the existing bounded protocol categories and the unknown name.
The library adds no template discovery, file loader, recursion, scripting,
product defaults or compatibility facade.
