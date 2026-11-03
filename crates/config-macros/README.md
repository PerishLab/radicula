# radicula-config-macros

The `Cascade` derive for the Radicula configuration capability. Normally import
it through `radicula_config::Cascade`; direct imports require the same runtime
package. The macro resolves that package's Cargo alias and generates one
runtime trait implementation, a typed partial and optional clap arguments.

The runtime README owns the public API. Generated code uses explicit lookup
and prefix inputs, reads no process environment and requires no product
identity or other PerishLab library. This crate shares the runtime's release
version and belongs to the same declared root capability.
