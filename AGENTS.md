# Radicula

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure.

Radicula publishes orthogonal public libraries directly from `crates/*` and,
when needed, `packages/*`. It has no umbrella library, CLI, API or application.
Keep the repository only as large as the actual libraries require.

Radicula is a closed root workspace in the PerishLab library graph. Its Rust
libraries may depend on one another as a DAG; their public responsibilities
remain distinct. No library may reach another PerishLab product library,
including through third-party packages. Cargo manifests own the real edges;
Plumb verifies their complete closure before delivery. Technical groups describe
capabilities and grant no dependency exceptions.
The boundary covers normal, build, dev, proc-macro, optional and platform
branches, including generated-code requirements. Internal cycles refuse and
unqualified closure evidence remains unknown. Do not add redundant edge lists,
merge unrelated groups or disable root checking to admit an edge.
Execution of released domain CLIs for checking, delivery and distribution is
outside the library graph. This Rust contract claims no unsupported language
or target coverage.

The repository has one shared version and release. Do not introduce independent
package versions or release protocols.

The first real library brings its actual workspace, Plumb declaration and
release configuration. Do not create an empty workspace, placeholder package
or product scaffold to make this initial governance baseline look complete.
