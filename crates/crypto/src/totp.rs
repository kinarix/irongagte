use irongate_core::CryptoError;
use totp_rs::{Algorithm, Secret, TOTP};

/// Generates a new TOTP secret (base32-encoded) and a `otpauth://` provisioning URI.
pub fn generate_totp_secret(
    issuer: &str,
    account_name: &str,
) -> Result<(String, String), CryptoError> {
    let secret = Secret::generate_secret();
    let bytes = secret
        .to_bytes()
        .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some(issuer.to_string()),
        account_name.to_string(),
    )
    .map_err(|e| CryptoError::KeyGeneration(e.to_string()))?;

    let secret_base32 = totp.get_secret_base32();
    let uri = totp.get_url();

    Ok((secret_base32, uri))
}

/// Verifies a TOTP code against a base32-encoded secret.
/// Accepts a ±1 step window to account for clock skew.
pub fn verify_totp(secret_base32: &str, code: &str) -> Result<bool, CryptoError> {
    let totp = build_totp(secret_base32)?;
    totp.check_current(code)
        .map_err(|e| CryptoError::Verification(e.to_string()))
}

/// Generates the current TOTP code (primarily for testing).
pub fn generate_totp_code(secret_base32: &str) -> Result<String, CryptoError> {
    let totp = build_totp(secret_base32)?;
    totp.generate_current()
        .map_err(|e| CryptoError::Signing(e.to_string()))
}

fn build_totp(secret_base32: &str) -> Result<TOTP, CryptoError> {
    let bytes = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| CryptoError::InvalidKey(e.to_string()))?;

    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        None,
        String::new(),
    )
    .map_err(|e| CryptoError::InvalidKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify() {
        let (secret, _uri) = generate_totp_secret("Irongate", "alice@example.com").unwrap();
        let code = generate_totp_code(&secret).unwrap();
        assert!(verify_totp(&secret, &code).unwrap());
    }

    #[test]
    fn wrong_code_fails() {
        let (secret, _) = generate_totp_secret("Irongate", "alice@example.com").unwrap();
        // "000000" is astronomically unlikely to be the current code
        let code = generate_totp_code(&secret).unwrap();
        if code != "000000" {
            assert!(!verify_totp(&secret, "000000").unwrap());
        }
    }

    #[test]
    fn uri_contains_issuer_and_account() {
        let (_secret, uri) = generate_totp_secret("Irongate", "alice").unwrap();
        assert!(uri.contains("Irongate"));
        assert!(uri.contains("alice"));
    }

    #[test]
    fn invalid_base32_secret_returns_err() {
        let result = verify_totp("not!valid!base32!!!", "123456");
        assert!(result.is_err());
    }

    #[test]
    fn generated_code_is_six_digits() {
        let (secret, _) = generate_totp_secret("Test", "user@test.com").unwrap();
        let code = generate_totp_code(&secret).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn each_secret_is_unique() {
        let (s1, _) = generate_totp_secret("App", "a@b.com").unwrap();
        let (s2, _) = generate_totp_secret("App", "a@b.com").unwrap();
        assert_ne!(s1, s2);
    }
}
