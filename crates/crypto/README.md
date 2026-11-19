# radicula-crypto

SHA-256, HMAC-SHA256 and HKDF-SHA256, with the constant-time helpers they use,
written from the specifications. `no_std`, no dependency of any kind; the only
crates under `[dev-dependencies]` are `sha2`, `hmac` and `hkdf`, used as
differential oracles in tests, and `serde_json`, which reads the vector files.
This is an independent library root; it depends on no other PerishLab library.

This library has not had an external audit. Do not rely on it where a failure
would matter until one has been done.

## Behaviour that the types do not show

- `ct::eq` treats lengths as public: unequal lengths are unequal at once. Equal
  lengths are compared without a data-dependent branch or index. A `Choice` is
  read back only by `flag`, and whoever branches on it owns that decision.
- `Hmac::verify` accepts a full 32 byte tag only. Any other length is false,
  including a valid truncation; compare a prefix of `finish` yourself if a
  protocol truncates.
- `Hkdf::expand` refuses more than 8160 bytes with `Error::Length` and leaves
  the output untouched. `Hkdf::new` refuses a PRK shorter than 32 bytes.
- `Hmac` and `Hkdf` hold keyed state and `Sha256` holds buffered input; all
  three overwrite that state with `ct::wipe` when dropped. Values returned as
  arrays, buffers a caller supplies, and earlier copies left behind by a move
  are not covered.
- Message length is counted in a `u64` of bytes. Input beyond 2^61 bytes is not
  reachable and is not checked.
- SHA-256 takes whole bytes only; bit-granular input is not offered.

## Evidence and its limits

Correctness is checked against RFC 6234, RFC 4231 and RFC 5869 vectors, the
NIST CAVP SHA-256 ShortMsg, LongMsg and Monte files, the Wycheproof HMAC-SHA256
and HKDF-SHA256 files, and the oracles above. `vectors/` holds the NIST files
(public domain) and the Wycheproof files (Apache-2.0) unchanged.

Timing is checked separately, because correct output says nothing about
execution time. `tests/timing` is a dudect-style test: interleaved samples of
two input classes, cropped at several percentiles, compared with Welch's t,
for `ct::eq`, `Hmac::verify`, and the two branches of `Choice::select` and
`Choice::assign`. It is paired with controls that must report a leak in
deliberately early-exiting comparisons, so a pass means the harness can see an
effect of that size on these paths. It is statistical, runs in the test
profile, and cannot show the absence of a smaller or different leak.
The generated code for `ct::eq`, `Choice::select`, `Choice::assign`,
`ct::wipe` and the comparison in `Hmac::verify` was read for x86-64 with the
compiler named in the delivering Issue; that review does not carry over to
other targets or compiler versions.
