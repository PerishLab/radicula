# radicula-crypto

SHA-256, HMAC-SHA256, HKDF-SHA256, ChaCha20, Poly1305, the ChaCha20-Poly1305
AEAD and X25519, with the constant-time helpers they use, written from the
specifications.
`no_std`, no dependency of any kind; the only crates under `[dev-dependencies]`
are `sha2`, `hmac`, `hkdf`, `chacha20`, `poly1305`, `chacha20poly1305` and `x25519-dalek`, used
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
- `x25519::mul` is the raw RFC 7748 function: it clamps a copy of the scalar
  (`k[0] &= 248`, `k[31] &= 127`, `k[31] |= 64`), clears bit 255 of the
  u-coordinate, accepts a non-canonical u, and returns whatever the ladder
  gives, including the all-zero value for a point of small order. It does not
  check the result.
- `x25519::agree` is `mul` followed by a check that the shared secret is not
  all zero; on all zero it wipes the value and returns `Error::Zero`. RFC 8446
  section 7.4.2 requires a TLS 1.3 implementation to make this check and
  abort, and RFC 7748 section 6.1 allows it. Protocol code must call `agree`,
  not `mul`. The check compares with `ct::eq`; that the secret is zero is a
  public reason to abort, so branching on it leaks nothing the peer does not
  learn anyway. `x25519::base` is `mul` against the base point.
- The Montgomery ladder swaps with `Choice`-based masks and reads each scalar
  bit with `Choice::bit`; no branch or index depends on a scalar bit. The
  ladder state and the clamped scalar copy are wiped before `mul` returns, but
  that wiping is on the stack and no test can observe it.
- X25519 takes a 32 byte private scalar and does not generate keys or random
  numbers. The 64-bit multiplications in the field code are constant time on
  current x86-64; that is a property of the target, not of the language.

## Evidence and its limits

Correctness is checked against RFC 6234, RFC 4231, RFC 5869, RFC 8439 and
RFC 7748 vectors, the NIST CAVP SHA-256 ShortMsg, LongMsg and Monte files, the
Wycheproof HMAC-SHA256, HKDF-SHA256, ChaCha20-Poly1305 and X25519 files, and the
oracles above. `vectors/` holds the NIST files (public domain) and the
Wycheproof files (Apache-2.0) unchanged. `vectors/rfc8439.json` is transcribed
from the RFC text, section 2 and appendix A, and `vectors/rfc7748.json` from
sections 5.2 and 6.1, including the 1 and 1000 iteration results of section
5.2; the one million iteration result is an ignored test, run with
`cargo test --release -p radicula-crypto --test x25519 -- --ignored`. Every
transcribed entry was re-derived by an independent reference implementation
before it was committed.

Timing is checked separately, because correct output says nothing about
execution time. `tests/timing` is a dudect-style test: interleaved samples of
two input classes, cropped at several percentiles, compared with Welch's t,
for `ct::eq`, `Hmac::verify`, `Poly1305::verify`, the rejection path of
`Chacha20poly1305::open`, the two branches of `Choice::select` and
`Choice::assign`, and the paths that touch secrets: the ChaCha20 block
function and Poly1305 over fixed, random and all-zero keys, and `seal` over
fixed, random and all-zero plaintexts, and `x25519::mul` over fixed, random and
all-zero scalars. It is paired with controls that must report a leak in
deliberately early-exiting comparisons and in a scalar multiplication that does
extra work for some scalars, so a pass means the harness can see an
effect of that size on these paths. It is statistical, runs in the test
profile, and cannot show the absence of a smaller or different leak. A scalar
multiplication costs milliseconds there, so the scalar probes take fewer
samples in the test profile than in a release build; a ladder swap turned into
a branch is visible in a release run and not in the test profile.
The generated code for `ct::eq`, `Choice::select`, `Choice::assign`,
`ct::wipe`, the comparison in `Hmac::verify`, the ChaCha20 core, Poly1305
absorb, finish and verify, `Chacha20poly1305::open`, and the X25519 field
multiplication, conditional swap and ladder loop was read for x86-64
with the compiler named in the delivering Issues; that review does not carry
over to other targets or compiler versions.

One thing the tests cannot see: replacing the comparison inside
`Hmac::verify`, `Poly1305::verify` or `Chacha20poly1305::open` with a plain
slice equality leaves the output identical. For `Hmac::verify` no timing
difference was visible even at one million samples per class, and none is
expected for the other two, so no test here would notice it.
`ct::eq` itself is probed in isolation and the generated code was read, but the
call sites are held in place only by review.
