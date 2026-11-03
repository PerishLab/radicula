# radicula-config

Typed configuration layers with caller-supplied prefixes and string lookup.
This crate and `radicula-config-macros` form one configuration capability and
share Radicula's version. They depend on no other Radicula capability or
PerishLab product.

```rust
use radicula_config::Cascade;

#[derive(Default, Cascade)]
#[cascade(strict)]
struct Settings {
    port: u16,
}

let get = |key: &str| (key == "SERVICE_PORT").then(|| "8080".into());
let held = Settings::resolve(None, "SERVICE", &get)?;
assert_eq!(held.port, 8080);
# Ok::<(), radicula_config::Error>(())
```

`lookup(prefix, get)` returns the generated `SettingsPartial` and `merge`
overlays only present fields. `resolve(file, prefix, get)` combines defaults,
an optional TOML file and lookup. `layered(file, prefix, get, partial)` adds a
final explicit overlay. File reads always use the supplied path. Neither the
configuration runtime nor generated lookup captures process environment,
selects an identity or discovers directories.

Nested `#[cascade(section)]` fields merge recursively. A section struct has
no application convenience methods or argument fields. `#[cascade(strict)]`
rejects unknown TOML fields; `#[cascade(name = "external_name")]` controls the
TOML key, uppercase lookup suffix and argument spelling.

`#[cascade(arg)]` generates a separate `SettingsArgs` with optional clap
arguments and a `partial()` conversion. Only consumers using that attribute
need their own clap dependency with derive support. An absent argument leaves
the lower layer intact. A partial `Some(None)` explicitly clears an optional
value. Lookup trims strings and ignores missing or blank values; invalid
supported values retain their full lookup key in `Error::Env`. Fields without
an `Env` implementation are ignored by lookup and remain available to file or
explicit layers. Consumers can implement `Env` for their own types.

The macro uses Cargo build metadata to resolve renamed runtime dependencies
and shares the runtime's serde export and trait identity. The `layer` module
defines the same core trait independently of its derive reexport. Using the
macro crate directly still requires the runtime dependency. Applications own
prefix selection, environment capture, admission, configuration discovery and
resource policy.
