# Radicula

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure.

Radicula publishes orthogonal public libraries directly from `crates/*` and,
when needed, `packages/*`. It has no umbrella library, CLI, API or application.
Keep the repository only as large as the actual libraries require.

Every orthogonal public capability is a strict root of the PerishLab library
dependency graph. A capability may require several technical crates, such as
a runtime library and its procedural macros. Dependencies within that capability
must be explicitly declared and verified by Plumb before they are introduced.
The boundary covers normal, build, dev, proc-macro, optional and platform
branches, including paths through third-party packages and generated-code
requirements. Different capabilities in this repository must not depend on
each other, and no capability may depend on another PerishLab product.
Technical grouping grants no exemption for cycles or unknown closure evidence.
Do not create placeholder crates or declarations for an anticipated grouping.
Execution of released domain CLIs for checking, delivery and distribution is
outside the library graph.

The repository has one shared version and release. Do not introduce independent
package versions or release protocols.

The first real library brings its actual workspace, Plumb declaration and
release configuration. Do not create an empty workspace, placeholder package
or product scaffold to make this initial governance baseline look complete.
