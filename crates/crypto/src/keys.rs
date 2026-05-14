use irongate_core::CryptoError;
use p256::ecdsa::SigningKey as EcSigningKey;
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rand_core::OsRng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use time::OffsetDateTime;
use uuid::Uuid;

const RSA_BITS: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAlgorithm {
    Rs256,
    Es256,
}

/// An asymmetric signing key with lifecycle metadata.
#[derive(Debug, Clone)]
pub struct SigningKeyRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub algorithm: KeyAlgorithm,
    /// PEM-encoded private key (PKCS#8).
    pub private_key_pem: String,
    /// PEM-encoded public key (SubjectPublicKeyInfo).
    pub public_key_pem: String,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    /// When this key was retired from signing (it may still verify).
    pub retired_at: Option<OffsetDateTime>,
}

impl SigningKeyRecord {
    /// True if this key can still be used for signing new tokens.
    pub fn is_active(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        self.retired_at.is_none() && now < self.expires_at
    }
}

/// Generates a new RSA-2048 signing key record.
pub fn generate_rsa_key(tenant_id: Uuid, ttl_days: i64) -> Result<SigningKeyRecord, CryptoError> {
    let private = RsaPrivateKey::new(&mut OsRng, RSA_BITS)
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;
    let public = RsaPublicKey::from(&private);

    let private_key_pem = private
        .to_pkcs8_pem(LineEnding::LF)
        .map(|s| s.to_string())
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let public_key_pem = public
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let now = OffsetDateTime::now_utc();
    Ok(SigningKeyRecord {
        id: Uuid::new_v4(),
        tenant_id,
        algorithm: KeyAlgorithm::Rs256,
        private_key_pem,
        public_key_pem,
        created_at: now,
        expires_at: now + time::Duration::days(ttl_days),
        retired_at: None,
    })
}

/// Generates a new P-256 (ES256) signing key record.
pub fn generate_ec_key(tenant_id: Uuid, ttl_days: i64) -> Result<SigningKeyRecord, CryptoError> {
    let signing_key = EcSigningKey::random(&mut OsRng);

    let private_key_pem = signing_key
        .to_pkcs8_pem(LineEnding::LF)
        .map(|s| s.to_string())
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let public_key_pem = signing_key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let now = OffsetDateTime::now_utc();
    Ok(SigningKeyRecord {
        id: Uuid::new_v4(),
        tenant_id,
        algorithm: KeyAlgorithm::Es256,
        private_key_pem,
        public_key_pem,
        created_at: now,
        expires_at: now + time::Duration::days(ttl_days),
        retired_at: None,
    })
}

/// Loads an RSA private key from PKCS#8 PEM.
pub fn load_rsa_private_key(pem: &str) -> Result<RsaPrivateKey, CryptoError> {
    RsaPrivateKey::from_pkcs8_pem(pem).map_err(|e| CryptoError::InvalidKey(e.to_string()))
}

/// Loads a P-256 signing key from PKCS#8 PEM.
pub fn load_ec_private_key(pem: &str) -> Result<EcSigningKey, CryptoError> {
    EcSigningKey::from_pkcs8_pem(pem).map_err(|e| CryptoError::InvalidKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsa_key_roundtrip() {
        let tenant = Uuid::new_v4();
        let rec = generate_rsa_key(tenant, 90).unwrap();
        assert_eq!(rec.tenant_id, tenant);
        assert_eq!(rec.algorithm, KeyAlgorithm::Rs256);
        assert!(rec.is_active());
        assert!(rec.private_key_pem.contains("PRIVATE KEY"));
        assert!(rec.public_key_pem.contains("PUBLIC KEY"));

        // Round-trip: load back from PEM
        let _ = load_rsa_private_key(&rec.private_key_pem).unwrap();
    }

    #[test]
    fn ec_key_roundtrip() {
        let tenant = Uuid::new_v4();
        let rec = generate_ec_key(tenant, 90).unwrap();
        assert_eq!(rec.tenant_id, tenant);
        assert_eq!(rec.algorithm, KeyAlgorithm::Es256);
        assert!(rec.is_active());

        let _ = load_ec_private_key(&rec.private_key_pem).unwrap();
    }

    #[test]
    fn expired_key_is_not_active() {
        let mut rec = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        rec.expires_at = OffsetDateTime::now_utc() - time::Duration::seconds(1);
        assert!(!rec.is_active());
    }

    #[test]
    fn retired_key_is_not_active() {
        let mut rec = generate_rsa_key(Uuid::new_v4(), 90).unwrap();
        rec.retired_at = Some(OffsetDateTime::now_utc());
        assert!(!rec.is_active());
    }

    #[test]
    fn each_rsa_generation_has_unique_id() {
        let tenant = Uuid::new_v4();
        let r1 = generate_rsa_key(tenant, 90).unwrap();
        let r2 = generate_rsa_key(tenant, 90).unwrap();
        assert_ne!(r1.id, r2.id);
        assert_ne!(r1.private_key_pem, r2.private_key_pem);
    }

    #[test]
    fn each_ec_generation_has_unique_id() {
        let tenant = Uuid::new_v4();
        let e1 = generate_ec_key(tenant, 90).unwrap();
        let e2 = generate_ec_key(tenant, 90).unwrap();
        assert_ne!(e1.id, e2.id);
        assert_ne!(e1.private_key_pem, e2.private_key_pem);
    }

    #[test]
    fn rsa_key_tenant_id_set_correctly() {
        let tenant = Uuid::new_v4();
        let rec = generate_rsa_key(tenant, 30).unwrap();
        assert_eq!(rec.tenant_id, tenant);
        assert!(rec.expires_at > rec.created_at);
    }

    #[test]
    fn ec_key_tenant_id_set_correctly() {
        let tenant = Uuid::new_v4();
        let rec = generate_ec_key(tenant, 30).unwrap();
        assert_eq!(rec.tenant_id, tenant);
    }
}
