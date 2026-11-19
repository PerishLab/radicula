# radicula-crypto

SHA-256, HMAC-SHA256, HKDF-SHA256, ChaCha20, Poly1305 and the ChaCha20-Poly1305
AEAD, with the constant-time helpers they use, written from the specifications.
`no_std`, no dependency of any kind; the only crates under `[dev-dependencies]`
are `sha2`, `hmac`, `hkdf`, `chacha20`, `poly1305` and `chacha20poly1305`, used
as differential oracles in tests, and `serde_json`, which reads the vector
files. The ChaCha20 family types are named `Chacha20`, `Poly1305` and
`Chacha20poly1305` because this repository allows single-word identifiers.
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
- `Chacha20` carries a 32-bit block counter that never wraps: once the
  keystream is used up, `apply` returns `Error::Length` and leaves the data
  alone. `Chacha20poly1305::seal` and `open` start the cipher at counter 1 and
  refuse more than 2^38 - 64 bytes with `Error::Length`; that limit needs a
  256 GiB slice to reach and no test exercises it.
- `Chacha20poly1305::open` checks the tag first and decrypts only when it
  matches. On `Error::Auth` the data is still the ciphertext, byte for byte.
  Whether the tag matched is public: an accepted message does more work than a
  rejected one, and the timing tests do not compare the two. They compare
  rejected tags that differ in the first, middle and last byte.
- `Poly1305` is a one-time authenticator. Using one key for two messages
  breaks it; the AEAD derives a fresh key per nonce, and a nonce must never be
  reused with a key. This library does not manage nonces.
- `Poly1305::verify` accepts a 16 byte tag only.
- `Chacha20`, `Poly1305` and `Chacha20poly1305` overwrite their state with
  `ct::wipe` when dropped, and the AEAD wipes the derived Poly1305 key and the
  keystream block it came from before returning.

## Evidence and its limits

Correctness is checked against RFC 6234, RFC 4231, RFC 5869 and RFC 8439
vectors, the NIST CAVP SHA-256 ShortMsg, LongMsg and Monte files, the
Wycheproof HMAC-SHA256, HKDF-SHA256 and ChaCha20-Poly1305 files, and the
oracles above. `vectors/` holds the NIST files (public domain) and the
Wycheproof files (Apache-2.0) unchanged. `vectors/rfc8439.json` is transcribed
from the RFC text, section 2 and appendix A, and every entry was re-derived by
an independent reference implementation before it was committed.

Timing is checked separately, because correct output says nothing about
execution time. `tests/timing` is a dudect-style test: interleaved samples of
two input classes, cropped at several percentiles, compared with Welch's t,
for `ct::eq`, `Hmac::verify`, `Poly1305::verify`, the rejection path of
`Chacha20poly1305::open`, the two branches of `Choice::select` and
`Choice::assign`, and the paths that touch secrets: the ChaCha20 block
function and Poly1305 over fixed, random and all-zero keys, and `seal` over
fixed, random and all-zero plaintexts. It is paired with controls that must report a leak in
deliberately early-exiting comparisons, so a pass means the harness can see an
effect of that size on these paths. It is statistical, runs in the test
profile, and cannot show the absence of a smaller or different leak.
The generated code for `ct::eq`, `Choice::select`, `Choice::assign`,
`ct::wipe`, the comparison in `Hmac::verify`, the ChaCha20 core, Poly1305
absorb, finish and verify, and `Chacha20poly1305::open` was read for x86-64
with the compiler named in the delivering Issues; that review does not carry
over to other targets or compiler versions.

One thing the tests cannot see: replacing the comparison inside
`Hmac::verify`, `Poly1305::verify` or `Chacha20poly1305::open` with a plain
slice equality leaves the output identical. For `Hmac::verify` no timing
difference was visible even at one million samples per class, and none is
expected for the other two, so no test here would notice it.
`ct::eq` itself is probed in isolation and the generated code was read, but the
call sites are held in place only by review.
