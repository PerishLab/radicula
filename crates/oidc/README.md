# radicula-oidc

Vendor-neutral OIDC protocol primitives shared by identity issuers and clients.
The library has no product, Keel, provider, HTTP, persistent state or key-store
knowledge. Established `jsonwebtoken` cryptography signs and verifies JWTs;
only explicitly selected ES256 and RS256 are admitted. A verifier uses one
caller-trusted key, exact key identifier and algorithm, and requires the `JWT`
type header. Token headers never select a URL, key source or algorithm policy.

`jwt::Claims` requires issuer, subject, audience, expiry and issuance time.
Audience accepts a string or string array. Additional claims remain opaque.
`jwt::Rules` supplies an exact issuer and audience, optional exact subject,
caller clock, permitted clock skew and exact additional string claims. Clients
must bind token purpose and nonce where their protocol requires them, for
example `required: &[("kind", "id"), ("nonce", expected)]`. Successful signature
verification does not prove a live identity, session, grant or cloud permission.
The caller owns maximum acceptable age/lifetime and multi-audience OIDC `azp`
policy when applicable. Required claims cannot override reserved JWT fields.

`pkce` implements RFC7636 S256 over a caller-generated verifier, admitting only
43 through 128 unreserved ASCII characters. It does not generate entropy or
store authorization state. `exchange` implements the bounded RFC8693 subset
currently needed by consumers: one access-token subject and one resource,
returning a bearer access token. Request validation rejects unsupported token
kinds and grants. This is not a complete OAuth client/server or an Alibaba STS
API. HTTP form encoding and endpoint authorization remain caller-owned.

`Secret` zeroizes its owned string at drop and redacts debug output. Explicit
serialization and `expose` reveal bytes for protocol use; callers own any
copies, encoded buffers, custody and transport. Claims may contain identifying
information and their debug output is not a safe audit record. `Fault` values
contain no input token, key or provider text; callers own serialization errors.

The API neither issues refresh grants nor changes revocation semantics. Use
Ensign for identity and session policy, Runseal for provider credentials and
finite calls, and Hardrig for supplier resource reconciliation. Algorithm and
wire compatibility with an external service requires separate qualification.
