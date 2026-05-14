use base64ct::{Base64UrlUnpadded, Encoding};
use rand::Rng;
use sha2::{Digest, Sha256};

const VERIFIER_BYTES: usize = 32;

/// Generates a (verifier, challenge) PKCE pair (RFC 7636, S256 method).
/// Returns `(code_verifier, code_challenge)`.
pub fn generate_pkce_pair() -> (String, String) {
    let mut bytes = [0u8; VERIFIER_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    let verifier = Base64UrlUnpadded::encode_string(&bytes);
    let challenge = derive_challenge(&verifier);
    (verifier, challenge)
}

/// Verifies that `verifier` produces `challenge` under S256.
pub fn verify_pkce_challenge(verifier: &str, challenge: &str) -> bool {
    derive_challenge(verifier) == challenge
}

fn derive_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    Base64UrlUnpadded::encode_string(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let (v, c) = generate_pkce_pair();
        assert!(verify_pkce_challenge(&v, &c));
    }

    #[test]
    fn wrong_verifier_fails() {
        let (_, c) = generate_pkce_pair();
        assert!(!verify_pkce_challenge("not-the-right-verifier", &c));
    }

    // RFC 7636 Appendix B test vector
    #[test]
    fn rfc_test_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(derive_challenge(verifier), expected);
        assert!(verify_pkce_challenge(verifier, expected));
    }

    #[test]
    fn verifier_is_url_safe() {
        let (v, _) = generate_pkce_pair();
        assert!(v
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn verifier_length() {
        // 32 bytes → 43 base64url-unpadded chars
        let (v, _) = generate_pkce_pair();
        assert_eq!(v.len(), 43);
    }

    #[test]
    fn challenge_has_no_padding() {
        let (_, c) = generate_pkce_pair();
        assert!(!c.contains('='));
    }
}
