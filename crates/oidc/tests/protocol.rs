use radicula_oidc::jwt::{
    Algorithm, Audience, Claims, DecodingKey, EncodingKey, Rules, Signer, Verifier,
};
use radicula_oidc::{Fault, Secret, exchange, pkce};
use serde_json::{Map, json};

fn keys(seed: u8) -> (Signer, Verifier) {
    use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    let secret = p256::SecretKey::from_slice(&[seed; 32]).unwrap();
    let private = secret.to_pkcs8_pem(LineEnding::LF).unwrap();
    let public = secret
        .public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    (
        Signer {
            algorithm: Algorithm::ES256,
            kid: "fixture".into(),
            key: EncodingKey::from_ec_pem(private.as_bytes()).unwrap(),
        },
        Verifier {
            algorithm: Algorithm::ES256,
            kid: "fixture".into(),
            key: DecodingKey::from_ec_pem(public.as_bytes()).unwrap(),
        },
    )
}

fn claims() -> Claims {
    Claims {
        iss: "https://identity.example/api".into(),
        sub: "stable-subject".into(),
        aud: Audience::One("cloud-client".into()),
        exp: 1100,
        iat: 900,
        nbf: None,
        extra: Map::from_iter([
            ("kind".into(), json!("id")),
            ("nonce".into(), json!("expected")),
        ]),
    }
}

fn rules() -> Rules<'static> {
    Rules {
        issuer: "https://identity.example/api",
        audience: "cloud-client",
        subject: Some("stable-subject"),
        now: 1000,
        leeway: 0,
        required: &[("kind", "id"), ("nonce", "expected")],
    }
}

#[test]
fn signature() {
    let (signer, verifier) = keys(7);
    let token = signer.sign(&claims()).unwrap();
    let result = verifier.verify(token.expose(), &rules()).unwrap();
    assert_eq!(result.sub, "stable-subject");
    assert_eq!(result.extra["nonce"], "expected");
    let (_, foreign) = keys(8);
    assert_eq!(
        foreign.verify(token.expose(), &rules()).unwrap_err(),
        Fault::Signature
    );
}

#[test]
fn binding() {
    let (signer, verifier) = keys(7);
    let token = signer.sign(&claims()).unwrap();
    let mut want = rules();
    want.issuer = "https://foreign.example";
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Issuer
    );
    want = rules();
    want.audience = "other-client";
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Audience
    );
    want = rules();
    want.subject = Some("another-subject");
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Subject
    );
    want = rules();
    want.required = &[("kind", "access")];
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Claim
    );
    want.required = &[("nonce", "foreign")];
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Claim
    );
}

#[test]
fn audience() {
    let (signer, verifier) = keys(7);
    let mut held = claims();
    held.aud = Audience::Many(vec!["another".into(), "cloud-client".into()]);
    let token = signer.sign(&held).unwrap();
    assert!(verifier.verify(token.expose(), &rules()).is_ok());
    held.aud = Audience::Many(vec![]);
    assert_eq!(signer.sign(&held).unwrap_err(), Fault::Format);
}

#[test]
fn clock() {
    let (signer, verifier) = keys(7);
    let token = signer.sign(&claims()).unwrap();
    let mut want = rules();
    want.now = 1100;
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Expired
    );
    want.leeway = 10;
    assert!(verifier.verify(token.expose(), &want).is_ok());
    want.now = 1110;
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Expired
    );
    want = rules();
    want.now = 899;
    assert_eq!(
        verifier.verify(token.expose(), &want).unwrap_err(),
        Fault::Premature
    );
    want.leeway = 1;
    assert!(verifier.verify(token.expose(), &want).is_ok());
    let mut held = claims();
    held.nbf = Some(1001);
    let token = signer.sign(&held).unwrap();
    assert_eq!(
        verifier.verify(token.expose(), &rules()).unwrap_err(),
        Fault::Premature
    );
}

#[test]
fn header() {
    let (signer, mut verifier) = keys(7);
    let token = signer.sign(&claims()).unwrap();
    verifier.algorithm = Algorithm::RS256;
    assert_eq!(
        verifier.verify(token.expose(), &rules()).unwrap_err(),
        Fault::Algorithm
    );
    verifier.algorithm = Algorithm::ES256;
    verifier.kid = "foreign-key".into();
    assert_eq!(
        verifier.verify(token.expose(), &rules()).unwrap_err(),
        Fault::Key
    );
    assert_eq!(
        verifier.verify("not-a-jwt", &rules()).unwrap_err(),
        Fault::Format
    );
    let mut head = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    head.kid = Some("fixture".into());
    head.typ = Some("another-token".into());
    let token = jsonwebtoken::encode(&head, &claims(), &signer.key).unwrap();
    verifier.kid = "fixture".into();
    assert_eq!(
        verifier.verify(&token, &rules()).unwrap_err(),
        Fault::Format
    );
}

#[test]
fn shape() {
    let (signer, verifier) = keys(7);
    let mut held = claims();
    held.extra.insert("iss".into(), json!("foreign"));
    assert_eq!(signer.sign(&held).unwrap_err(), Fault::Claim);
    held = claims();
    held.exp = held.iat;
    assert_eq!(signer.sign(&held).unwrap_err(), Fault::Format);
    let mut head = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    head.kid = Some("fixture".into());
    let missing =
        json!({"iss": "https://identity.example/api", "sub": "s", "aud": "a", "exp": 1100});
    let token = jsonwebtoken::encode(&head, &missing, &signer.key).unwrap();
    assert!(verifier.verify(&token, &rules()).is_err());
}

#[test]
fn proof() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert_eq!(pkce::challenge(verifier).unwrap(), challenge);
    assert!(pkce::verify(verifier, challenge).is_ok());
    assert_eq!(pkce::verify(verifier, "foreign").unwrap_err(), Fault::Proof);
    for value in [
        "x".repeat(42),
        "x".repeat(129),
        "!".repeat(43),
        "é".repeat(43),
    ] {
        assert_eq!(pkce::challenge(&value).unwrap_err(), Fault::Proof);
    }
    assert!(pkce::challenge(&"x".repeat(128)).is_ok());
}

#[test]
fn exchange() {
    let request = exchange::Request::new(Secret::new("private-subject".into()), "cloud".into());
    assert!(request.validate().is_ok());
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["grant_type"], exchange::GRANT);
    assert_eq!(value["subject_token_type"], exchange::ACCESS);
    let mut unsupported = value.clone();
    unsupported["actor_token"] = json!("another-actor");
    assert!(serde_json::from_value::<exchange::Request>(unsupported).is_err());
    let mut wrong: exchange::Request = serde_json::from_value(value).unwrap();
    wrong.sort = "urn:ietf:params:oauth:token-type:id_token".into();
    assert_eq!(wrong.validate().unwrap_err(), Fault::Grant);
    let reply: exchange::Response = serde_json::from_value(json!({
        "access_token": "private-response", "issued_token_type": exchange::ACCESS,
        "token_type": "Bearer", "expires_in": 600,
    }))
    .unwrap();
    assert!(reply.validate().is_ok());
    assert!(!format!("{request:?}{reply:?}").contains("private-"));
}

#[test]
fn refusals() {
    let mut request = exchange::Request::new(Secret::new(String::new()), "cloud".into());
    assert_eq!(request.validate().unwrap_err(), Fault::Format);
    request.grant = "client_credentials".into();
    assert_eq!(request.validate().unwrap_err(), Fault::Grant);
    let mut reply = exchange::Response {
        token: Secret::new("private-response".into()),
        sort: exchange::ACCESS.into(),
        bearer: "Bearer".into(),
        life: 0,
    };
    assert_eq!(reply.validate().unwrap_err(), Fault::Format);
    reply.life = 600;
    reply.bearer = "other".into();
    assert_eq!(reply.validate().unwrap_err(), Fault::Grant);
    assert!(!format!("{:?}", Fault::Signature).contains("private-response"));
}
