use super::{Claims, Rules};
use crate::Fault;

pub(super) fn check(claims: &Claims, rules: &Rules<'_>) -> Result<(), Fault> {
    claims.shape()?;
    if rules.issuer.is_empty() || claims.iss != rules.issuer {
        return Err(Fault::Issuer);
    }
    if rules.audience.is_empty() || !claims.aud.contains(rules.audience) {
        return Err(Fault::Audience);
    }
    if rules.subject.is_some_and(|subject| subject != claims.sub) {
        return Err(Fault::Subject);
    }
    if rules.now >= claims.exp.saturating_add(rules.leeway) {
        return Err(Fault::Expired);
    }
    let future = rules.now.saturating_add(rules.leeway);
    if claims.iat > future || claims.nbf.is_some_and(|nbf| nbf > future) {
        return Err(Fault::Premature);
    }
    for (key, expected) in rules.required {
        if claims.extra.get(*key).and_then(|v| v.as_str()) != Some(*expected) {
            return Err(Fault::Claim);
        }
    }
    Ok(())
}
