use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use irongate_core::CryptoError;

/// Hashes a plaintext password using Argon2id with a random salt.
/// Returns the PHC string (includes algorithm params + salt + hash).
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| CryptoError::Hashing(e.to_string()))
}

/// Verifies a plaintext password against a stored PHC hash string.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError> {
    let parsed = PasswordHash::new(hash).map_err(|e| CryptoError::Hashing(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn same_password_produces_different_hashes() {
        let h1 = hash_password("test").unwrap();
        let h2 = hash_password("test").unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn invalid_hash_returns_error() {
        assert!(verify_password("anything", "not-a-valid-phc-hash").is_err());
    }

    #[test]
    fn empty_password_is_hashable() {
        let hash = hash_password("").unwrap();
        assert!(verify_password("", &hash).unwrap());
        assert!(!verify_password("notempty", &hash).unwrap());
    }

    #[test]
    fn unicode_password() {
        let pw = "p@$$w0rd_日本語_🔐";
        let hash = hash_password(pw).unwrap();
        assert!(verify_password(pw, &hash).unwrap());
    }

    #[test]
    fn hash_starts_with_argon2id() {
        let hash = hash_password("test").unwrap();
        assert!(hash.starts_with("$argon2id$"));
    }
}
