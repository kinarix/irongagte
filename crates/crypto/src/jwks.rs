use irongate_core::CryptoError;
use jsonwebtoken::{
    Algorithm,
    EncodingKey,
    jwk::{CommonParameters, Jwk, JwkSet},
};

use crate::keys::{KeyAlgorithm, SigningKeyRecord};

/// Builds a `JwkSet` from a slice of active signing key records.
/// Only the public key material is included — private keys are never published.
pub fn build_jwks(records: &[SigningKeyRecord]) -> Result<JwkSet, CryptoError> {
    let keys = records
        .iter()
        .map(|rec| record_to_jwk(rec))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(JwkSet { keys })
}

/// Serialises a `JwkSet` to a JSON string for serving at `/.well-known/jwks.json`.
pub fn jwks_to_json(jwks: &JwkSet) -> Result<String, CryptoError> {
    serde_json::to_string(jwks).map_err(|e| CryptoError::Signing(e.to_string()))
}

fn record_to_jwk(rec: &SigningKeyRecord) -> Result<Jwk, CryptoError> {
    let algorithm = match rec.algorithm {
        KeyAlgorithm::Rs256 => Algorithm::RS256,
        KeyAlgorithm::Es256 => Algorithm::ES256,
    };

    let encoding_key = encoding_key_for_record(rec)?;
    let mut jwk = Jwk::from_encoding_key(&encoding_key, algorithm)
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    // Set the key ID so clients can find the right key from the JWT header `kid`.
    jwk.common = CommonParameters {
        key_id: Some(rec.id.to_string()),
        ..jwk.common
    };

    Ok(jwk)
}

fn encoding_key_for_record(rec: &SigningKeyRecord) -> Result<EncodingKey, CryptoError> {
    match rec.algorithm {
        KeyAlgorithm::Rs256 => EncodingKey::from_rsa_pem(rec.private_key_pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
        KeyAlgorithm::Es256 => EncodingKey::from_ec_pem(rec.private_key_pem.as_bytes())
            .map_err(|e| CryptoError::InvalidKey(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{generate_ec_key, generate_rsa_key};
    use uuid::Uuid;

    #[test]
    fn rsa_jwks_contains_kid() {
        let rec = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        let kid = rec.id.to_string();
        let jwks = build_jwks(&[rec]).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].common.key_id.as_deref(), Some(kid.as_str()));
    }

    #[test]
    fn ec_jwks_contains_kid() {
        let rec = generate_ec_key(Uuid::new_v4(), 90).unwrap();
        let kid = rec.id.to_string();
        let jwks = build_jwks(&[rec]).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].common.key_id.as_deref(), Some(kid.as_str()));
    }

    #[test]
    fn jwks_json_is_valid() {
        let rsa = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        let ec = generate_ec_key(Uuid::new_v4(), 90).unwrap();
        let jwks = build_jwks(&[rsa, ec]).unwrap();
        let json = jwks_to_json(&jwks).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["keys"].is_array());
        assert_eq!(parsed["keys"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn jwks_find_by_kid() {
        let rsa = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        let target_kid = rsa.id.to_string();
        let ec = generate_ec_key(Uuid::new_v4(), 90).unwrap();
        let jwks = build_jwks(&[rsa, ec]).unwrap();
        assert!(jwks.find(&target_kid).is_some());
        assert!(jwks.find("nonexistent-kid").is_none());
    }

    #[test]
    fn empty_slice_produces_empty_jwks() {
        let jwks = build_jwks(&[]).unwrap();
        assert!(jwks.keys.is_empty());
        let json = jwks_to_json(&jwks).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["keys"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn rsa_jwk_has_no_private_key_d() {
        let rec = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        let jwks = build_jwks(&[rec]).unwrap();
        let json = jwks_to_json(&jwks).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let key = &v["keys"][0];
        assert!(key["d"].is_null(), "RSA private exponent 'd' must not be in JWKS");
        assert!(!key["n"].is_null(), "RSA modulus 'n' should be present");
        assert!(!key["e"].is_null(), "RSA exponent 'e' should be present");
    }

    #[test]
    fn ec_jwk_has_no_private_key_d() {
        let rec = generate_ec_key(Uuid::new_v4(), 90).unwrap();
        let jwks = build_jwks(&[rec]).unwrap();
        let json = jwks_to_json(&jwks).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let key = &v["keys"][0];
        assert!(key["d"].is_null(), "EC private scalar 'd' must not be in JWKS");
        assert!(!key["x"].is_null(), "EC x coordinate should be present");
        assert!(!key["y"].is_null(), "EC y coordinate should be present");
    }
}
