use radicula_oidc::jwt::{
    Algorithm, Audience, Claims, DecodingKey, EncodingKey, Rules, Signer, Verifier,
};
use rand::SeedableRng;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use serde_json::Map;

#[test]
fn signature() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(73);
    let private = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let public = private.to_public_key();
    let private = private.to_pkcs8_pem(LineEnding::LF).unwrap();
    let public = public.to_public_key_pem(LineEnding::LF).unwrap();
    let signer = Signer {
        algorithm: Algorithm::RS256,
        kid: "inert-rsa-fixture".into(),
        key: EncodingKey::from_rsa_pem(private.as_bytes()).unwrap(),
    };
    let verifier = Verifier {
        algorithm: Algorithm::RS256,
        kid: "inert-rsa-fixture".into(),
        key: DecodingKey::from_rsa_pem(public.as_bytes()).unwrap(),
    };
    let claims = Claims {
        iss: "https://issuer.example".into(),
        sub: "subject".into(),
        aud: Audience::One("client".into()),
        exp: 1100,
        iat: 900,
        nbf: None,
        extra: Map::new(),
    };
    let token = signer.sign(&claims).unwrap();
    let rules = Rules {
        issuer: "https://issuer.example",
        audience: "client",
        subject: Some("subject"),
        now: 1000,
        leeway: 0,
        required: &[],
    };
    assert!(verifier.verify(token.expose(), &rules).is_ok());
}
