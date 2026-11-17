use super::{Algorithm, Claims, DecodingKey, EncodingKey, Rules};
use crate::{Fault, Secret};
use jsonwebtoken::{Header, Validation, decode, decode_header, encode};

pub struct Signer {
    pub algorithm: Algorithm,
    pub kid: String,
    pub key: EncodingKey,
}

impl Signer {
    pub fn sign(&self, claims: &Claims) -> Result<Secret, Fault> {
        claims.shape()?;
        if self.kid.is_empty() {
            return Err(Fault::Key);
        }
        let mut header = Header::new(self.algorithm.native());
        header.kid = Some(self.kid.clone());
        encode(&header, claims, &self.key)
            .map(Secret::new)
            .map_err(|_| Fault::Signature)
    }
}

pub struct Verifier {
    pub algorithm: Algorithm,
    pub kid: String,
    pub key: DecodingKey,
}

impl Verifier {
    pub fn verify(&self, token: &str, rules: &Rules<'_>) -> Result<Claims, Fault> {
        let header = decode_header(token).map_err(|_| Fault::Format)?;
        if header.alg != self.algorithm.native() {
            return Err(Fault::Algorithm);
        }
        if self.kid.is_empty() || header.kid.as_deref() != Some(self.kid.as_str()) {
            return Err(Fault::Key);
        }
        if header.typ.as_deref() != Some("JWT") {
            return Err(Fault::Format);
        }
        let mut validation = Validation::new(self.algorithm.native());
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        let claims = decode::<Claims>(token, &self.key, &validation)
            .map_err(|_| Fault::Signature)?
            .claims;
        super::check::check(&claims, rules)?;
        Ok(claims)
    }
}
