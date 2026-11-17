mod check;
mod claims;
mod codec;

pub use claims::{Audience, Claims};
pub use codec::{Signer, Verifier};
pub use jsonwebtoken::{DecodingKey, EncodingKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    ES256,
    RS256,
}

impl Algorithm {
    fn native(self) -> jsonwebtoken::Algorithm {
        match self {
            Self::ES256 => jsonwebtoken::Algorithm::ES256,
            Self::RS256 => jsonwebtoken::Algorithm::RS256,
        }
    }
}

pub struct Rules<'a> {
    pub issuer: &'a str,
    pub audience: &'a str,
    pub subject: Option<&'a str>,
    pub now: u64,
    pub leeway: u64,
    pub required: &'a [(&'a str, &'a str)],
}
