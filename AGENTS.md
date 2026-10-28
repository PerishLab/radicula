# Radicula

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure.

Radicula publishes orthogonal public libraries directly from `crates/*` and,
when needed, `packages/*`. It has no umbrella library, CLI, API or application.
Keep the repository only as large as the actual libraries require.

Every public library is a strict root of the PerishLab library dependency graph.
The boundary covers normal, build, dev, proc-macro, optional and platform
branches, including paths through third-party packages. Libraries in this
repository must not depend on each other. Execution of released domain CLIs
for checking, delivery and distribution is outside the library graph.

The repository has one shared version and release. Do not introduce independent
package versions or release protocols.

The first real library brings its actual workspace, Plumb declaration and
release configuration. Do not create an empty workspace, placeholder package
or product scaffold to make this initial governance baseline look complete.
