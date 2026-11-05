# radicula-trace-macros

Optional span instrumentation for `radicula-trace`, sharing Radicula's version.
Use the runtime's `macros` feature and its `span` export. Generated code calls
`radicula_trace::run` or `radicula_trace::carry`; the runtime owns all Context,
span completion and callback behavior. This crate has no runtime dependency on
Context or Trace and grants no application observation or persistence policy.
