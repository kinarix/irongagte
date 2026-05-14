use irongate_core::CryptoError;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{de::DeserializeOwned, Serialize};

/// Signs a claims struct into a compact JWT.
/// `key_pem` must be a PKCS#8 PEM private key; `algorithm` must be RS256 or ES256.
pub fn sign<C: Serialize>(
    claims: &C,
    key_pem: &str,
    algorithm: Algorithm,
    key_id: Option<&str>,
) -> Result<String, CryptoError> {
    let encoding_key = encoding_key_from_pem(key_pem, algorithm)?;
    let mut header = Header::new(algorithm);
    header.kid = key_id.map(String::from);

    encode(&header, claims, &encoding_key).map_err(|e| CryptoError::Signing(e.to_string()))
}

/// Verifies a compact JWT and deserializes the claims.
pub fn verify<C: DeserializeOwned>(
    token: &str,
    key_pem: &str,
    algorithm: Algorithm,
    validation: &Validation,
) -> Result<TokenData<C>, CryptoError> {
    let decoding_key = decoding_key_from_pem(key_pem, algorithm)?;
    decode::<C>(token, &decoding_key, validation).map_err(map_jwt_error)
}

fn encoding_key_from_pem(pem: &str, algorithm: Algorithm) -> Result<EncodingKey, CryptoError> {
    match algorithm {
        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => EncodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
        Algorithm::ES256 | Algorithm::ES384 => EncodingKey::from_ec_pem(pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
        _ => Err(CryptoError::InvalidKey("unsupported algorithm".into())),
    }
}

fn decoding_key_from_pem(pem: &str, algorithm: Algorithm) -> Result<DecodingKey, CryptoError> {
    match algorithm {
        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => DecodingKey::from_rsa_pem(pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
        Algorithm::ES256 | Algorithm::ES384 => DecodingKey::from_ec_pem(pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
        _ => Err(CryptoError::InvalidKey("unsupported algorithm".into())),
    }
}

fn map_jwt_error(e: jsonwebtoken::errors::Error) -> CryptoError {
    use jsonwebtoken::errors::ErrorKind;
    match e.kind() {
        ErrorKind::ExpiredSignature => CryptoError::TokenExpired,
        ErrorKind::InvalidToken
        | ErrorKind::InvalidSignature
        | ErrorKind::InvalidAlgorithm
        | ErrorKind::InvalidAudience
        | ErrorKind::InvalidIssuer
        | ErrorKind::InvalidSubject
        | ErrorKind::MissingRequiredClaim(_) => CryptoError::InvalidToken(e.to_string()),
        _ => CryptoError::Verification(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{generate_ec_key, generate_rsa_key};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestClaims {
        sub: String,
        exp: u64,
        iat: u64,
    }

    fn claims(ttl_secs: u64) -> TestClaims {
        let now = jsonwebtoken::get_current_timestamp();
        TestClaims {
            sub: "user-123".into(),
            iat: now,
            exp: now + ttl_secs,
        }
    }

    fn no_validation(alg: Algorithm) -> Validation {
        let mut v = Validation::new(alg);
        v.required_spec_claims.clear();
        v.validate_exp = false;
        v
    }

    #[test]
    fn rsa_sign_and_verify() {
        let rec = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let c = claims(3600);
        let token = sign(&c, &rec.private_key_pem, Algorithm::RS256, Some("key-1")).unwrap();
        let data: TokenData<TestClaims> = verify(
            &token,
            &rec.public_key_pem,
            Algorithm::RS256,
            &no_validation(Algorithm::RS256),
        )
        .unwrap();
        assert_eq!(data.claims.sub, c.sub);
        assert_eq!(data.header.kid.as_deref(), Some("key-1"));
    }

    #[test]
    fn ec_sign_and_verify() {
        let rec = generate_ec_key(Some(Uuid::new_v4()), 90).unwrap();
        let c = claims(3600);
        let token = sign(&c, &rec.private_key_pem, Algorithm::ES256, None).unwrap();
        let data: TokenData<TestClaims> = verify(
            &token,
            &rec.public_key_pem,
            Algorithm::ES256,
            &no_validation(Algorithm::ES256),
        )
        .unwrap();
        assert_eq!(data.claims.sub, c.sub);
    }

    #[test]
    fn expired_token_returns_token_expired_error() {
        let rec = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let c = TestClaims {
            sub: "u".into(),
            iat: 0,
            exp: 1, // expired in the past
        };
        let token = sign(&c, &rec.private_key_pem, Algorithm::RS256, None).unwrap();
        let mut v = Validation::new(Algorithm::RS256);
        v.required_spec_claims.clear();
        let err =
            verify::<TestClaims>(&token, &rec.public_key_pem, Algorithm::RS256, &v).unwrap_err();
        assert!(matches!(err, CryptoError::TokenExpired));
    }

    #[test]
    fn wrong_key_fails_verification() {
        let rec1 = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let rec2 = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let token = sign(&claims(3600), &rec1.private_key_pem, Algorithm::RS256, None).unwrap();
        let err = verify::<TestClaims>(
            &token,
            &rec2.public_key_pem,
            Algorithm::RS256,
            &no_validation(Algorithm::RS256),
        )
        .unwrap_err();
        assert!(matches!(err, CryptoError::InvalidToken(_)));
    }

    #[test]
    fn tampered_payload_rejected() {
        let rec = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let token = sign(&claims(3600), &rec.private_key_pem, Algorithm::RS256, None).unwrap();

        // Flip a character in the payload section (middle part of header.payload.sig)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let mut bad_payload = parts[1].to_string();
        // Replace the last char with something different
        let last = bad_payload.pop().unwrap();
        let replacement = if last == 'A' { 'B' } else { 'A' };
        bad_payload.push(replacement);
        let tampered = format!("{}.{}.{}", parts[0], bad_payload, parts[2]);

        let result = verify::<TestClaims>(
            &tampered,
            &rec.public_key_pem,
            Algorithm::RS256,
            &no_validation(Algorithm::RS256),
        );
        assert!(result.is_err());
    }

    #[test]
    fn audience_validation_rejects_wrong_aud() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct AudClaims {
            sub: String,
            aud: String,
            exp: u64,
        }

        let rec = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let now = jsonwebtoken::get_current_timestamp();
        let c = AudClaims {
            sub: "u".into(),
            aud: "my-app".into(),
            exp: now + 3600,
        };
        let token = sign(&c, &rec.private_key_pem, Algorithm::RS256, None).unwrap();

        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&["wrong-app"]);
        v.validate_exp = false;

        let err =
            verify::<AudClaims>(&token, &rec.public_key_pem, Algorithm::RS256, &v).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidToken(_)));
    }

    #[test]
    fn kid_is_preserved_in_header() {
        let rec = generate_rsa_key(Some(Uuid::new_v4()), 90).unwrap();
        let c = claims(3600);
        let token = sign(&c, &rec.private_key_pem, Algorithm::RS256, Some("my-kid")).unwrap();
        let data: TokenData<TestClaims> = verify(
            &token,
            &rec.public_key_pem,
            Algorithm::RS256,
            &no_validation(Algorithm::RS256),
        )
        .unwrap();
        assert_eq!(data.header.kid.as_deref(), Some("my-kid"));
    }
}
