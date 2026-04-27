use base64ct::{Base64UrlUnpadded, Encoding};
use rand::Rng;
use sha2::{Digest, Sha256};

const TOKEN_BYTES: usize = 32;

/// Generates a cryptographically-random opaque token (base64url-unpadded, 43 chars).
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    Base64UrlUnpadded::encode_string(&bytes)
}

/// SHA-256 hashes a token for safe storage. Returns hex-encoded digest.
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    bytes_to_hex(&digest)
}

/// Compares a raw token against a stored hash in constant time.
pub fn verify_token(token: &str, stored_hash: &str) -> bool {
    let digest = Sha256::digest(token.as_bytes());
    let expected = bytes_to_hex(&digest);
    constant_time_eq(expected.as_bytes(), stored_hash.as_bytes())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_length() {
        // 32 raw bytes → 43 base64url-unpadded characters
        assert_eq!(generate_token().len(), 43);
    }

    #[test]
    fn token_uniqueness() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let token = generate_token();
        let hash = hash_token(&token);
        assert!(verify_token(&token, &hash));
    }

    #[test]
    fn wrong_token_does_not_verify() {
        let token = generate_token();
        let hash = hash_token(&token);
        assert!(!verify_token("wrong-token", &hash));
    }

    #[test]
    fn token_is_url_safe() {
        let t = generate_token();
        assert!(t.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn known_sha256_vector() {
        // echo -n "abc" | sha256sum → ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469f492c347e001facc5
        let hash = hash_token("abc");
        assert_eq!(hash, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn hash_is_lowercase_hex() {
        let hash = hash_token("test");
        assert!(hash.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
        assert_eq!(hash.len(), 64);
    }
}
