# radicula-crypto

SHA-256, HMAC-SHA256, HKDF-SHA256, ChaCha20, Poly1305, the ChaCha20-Poly1305
AEAD, X25519, AES-128 and AES-256 with GHASH and the GCM AEAD, with the
constant-time helpers they use, written from the specifications.
`no_std`, no dependency of any kind; the only crates under `[dev-dependencies]`
are `sha2`, `hmac`, `hkdf`, `chacha20`, `poly1305`, `chacha20poly1305`, `x25519-dalek`, `aes`, `ghash` and `aes-gcm`, used
as differential oracles in tests, and `serde_json`, which reads the vector
files. The ChaCha20 family types are named `Chacha20`, `Poly1305` and
`Chacha20poly1305` because this repository allows single-word identifiers;
the same holds for `Aes`, `Ghash` and `Aesgcm`.
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
- `Aes` is the FIPS 197 forward cipher only, for 16 and 32 byte keys; any other
  key length, 24 included, is `Error::Length`. It is a portable bitsliced
  implementation: the S-box is a Boyar-Peralta circuit of AND, XOR and NOT on
  64-bit words, four blocks are processed per call, and no table is indexed by
  a key or data byte. `Aes::encrypt` of one block costs a full four-block pass.
- `Ghash` multiplies with a masked carry-less multiply on 64-bit integers
  (Karatsuba over 64-bit halves, then reduction by x^128 + x^7 + x^2 + x + 1);
  it has no table and no data-dependent branch. `finish` zero-pads a trailing
  partial block, `pad` does the same earlier. The multiply uses integer
  multiplication, which is constant time on current x86-64; that is a property
  of the target, not of the language.
- `Aesgcm` takes a 12 byte nonce only and produces or checks a 16 byte tag
  only: other IV lengths and truncated tags are not offered, and a tag of any
  other length is `Error::Auth`. Plaintext beyond 2^36 - 32 bytes is
  `Error::Length`; that limit needs a 64 GiB slice to reach and no test
  exercises it. A nonce must never be reused with a key; this library does not
  manage nonces, and reuse breaks GCM's authentication as well as its secrecy.
- `Aesgcm::open` computes and compares the tag first (`ct::eq`) and decrypts only
  when it matches; on `Error::Auth` the data is still the ciphertext, byte for
  byte. Whether the tag matched is public. `Aes`, `Ghash` and `Aesgcm`
  overwrite their key material with `ct::wipe` when dropped; temporaries on the
  stack that hold AES state or keystream are wiped too, but no test can observe that.
- Throughput of this portable code, measured in a release build on one core of an
  Intel Xeon (the box this was developed on, three runs): AES-128-GCM seal
  88 to 98 MB/s, AES-256-GCM seal 76 to 79 MB/s, GHASH alone 311 to 325 MB/s;
  a 1350 byte seal runs at 85 to 91 MB/s for AES-128 and 70 to 73 MB/s for
  AES-256. That is far below hardware-accelerated AES-GCM. There is no AES-NI or
  PCLMULQDQ path. The numbers come from an ignored test,
  `cargo test --release -p radicula-crypto --test gcm -- --ignored --nocapture`.

## Evidence and its limits

Correctness is checked against RFC 6234, RFC 4231, RFC 5869, RFC 8439 and
RFC 7748 and FIPS 197 vectors, the NIST CAVP SHA-256 ShortMsg, LongMsg and
Monte files and the NIST CAVP GCM encrypt and decrypt files, the Wycheproof
HMAC-SHA256, HKDF-SHA256, ChaCha20-Poly1305, X25519 and AES-GCM files, and the
oracles above. `vectors/` holds the NIST files (public domain) and the
Wycheproof files (Apache-2.0) unchanged. `vectors/rfc8439.json` is transcribed
from the RFC text, section 2 and appendix A, and `vectors/rfc7748.json` from
sections 5.2 and 6.1, including the 1 and 1000 iteration results of section
5.2; the one million iteration result is an ignored test, run with
`cargo test --release -p radicula-crypto --test x25519 -- --ignored`. Every
transcribed entry was re-derived by an independent reference implementation
before it was committed. `vectors/gcm/` holds the AES-GCM material: the Wycheproof
`aes_gcm_test.json` unchanged; the CAVP `gcmEncryptExtIV128`, `gcmEncryptExtIV256`,
`gcmDecrypt128` and `gcmDecrypt256` files reduced to the groups with a 96 bit IV
and a 128 bit tag, each record unchanged (the other groups, and the 192 bit key
files, use IV and tag lengths or a key size this interface does not offer); and
`fips197.json`, transcribed from FIPS 197 appendices B, C.1 and C.3 and checked
against an independent implementation. Of the 316 Wycheproof AES-GCM cases, 133
are run (79 valid, 54 invalid); 103 use a 192 bit key and 80 a non-96-bit IV and are
excluded by the interface, and the test asserts those counts.

Timing is checked separately, because correct output says nothing about
execution time. `tests/timing` is a dudect-style test: interleaved samples of
two input classes, cropped at several percentiles, compared with Welch's t,
for `ct::eq`, `Hmac::verify`, `Poly1305::verify`, the rejection path of
`Chacha20poly1305::open`, the two branches of `Choice::select` and
`Choice::assign`, and the paths that touch secrets: the ChaCha20 block
function and Poly1305 over fixed, random and all-zero keys, and `seal` over
fixed, random and all-zero plaintexts, and `x25519::mul` over fixed, random and
all-zero scalars, `Aes` over fixed, random and all-zero keys (key schedule
included) and blocks for both key sizes, `Ghash` over fixed, random and all-zero
keys and data, and the rejection path of `Aesgcm::open`. It is paired with controls that must report a leak in
deliberately early-exiting comparisons and in a scalar multiplication that does
extra work for some scalars, so a pass means the harness can see an
effect of that size on these paths. The AES-GCM controls are a key-dependent
extra block encryption, a data-dependent extra GHASH block, and an early-exit
tag comparison repeated 50 times after a real `open`; the extra work is large
enough that they report t in the hundreds, so the floor of what the probes can
see is not known more precisely than that. It is statistical, runs in the test
profile, and cannot show the absence of a smaller or different leak. A scalar
multiplication costs milliseconds there, so the scalar probes take fewer
samples in the test profile than in a release build; a ladder swap turned into
a branch is visible in a release run and not in the test profile.
The generated code for `ct::eq`, `Choice::select`, `Choice::assign`,
`ct::wipe`, the comparison in `Hmac::verify`, the ChaCha20 core, Poly1305
absorb, finish and verify, `Chacha20poly1305::open`, and the X25519 field
multiplication, conditional swap and ladder loop, and the AES S-box circuit, the
AES round and key schedule, the GHASH multiply and the tag comparison in
`Aesgcm::open`, was read for x86-64
with the compiler named in the delivering Issues; that review does not carry
over to other targets or compiler versions.

One thing the tests cannot see: replacing the comparison inside
`Hmac::verify`, `Poly1305::verify` or `Chacha20poly1305::open` with a plain
slice equality leaves the output identical. For `Hmac::verify` no timing
difference was visible even at one million samples per class, and none is
expected for the other two, so no test here would notice it.
`ct::eq` itself is probed in isolation and the generated code was read, but the
call sites are held in place only by review.

For AES-GCM the same blind spot holds for the tag comparison inside
`Aesgcm::open`: with an early-exit comparison substituted for it, the
test-profile timing run saw it and the release run did not, because the
comparison is a few nanoseconds inside a call that takes microseconds. The
isolated `ct::eq` probe and the read assembly are what hold that call site. A
data-dependent extra S-box pass that fires on a state bit was seen in the release
run and not in the test profile, because a fixed class and a random class can
share a mean count of extra passes. Neither is evidence of absence of a leak.
