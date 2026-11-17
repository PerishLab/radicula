#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Format,
    Signature,
    Algorithm,
    Key,
    Issuer,
    Audience,
    Subject,
    Expired,
    Premature,
    Claim,
    Proof,
    Grant,
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OIDC {:?} refused", self)
    }
}

impl std::error::Error for Fault {}
