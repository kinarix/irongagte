use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::types::*;

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

// ── UserStatus ────────────────────────────────────────────────────────────────

#[test]
fn user_status_roundtrip_serde() {
    for status in [
        UserStatus::Active,
        UserStatus::Suspended,
        UserStatus::Pending,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let back: UserStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&status),
            std::mem::discriminant(&back)
        );
    }
}

#[test]
fn user_status_display_and_parse() {
    assert_eq!(UserStatus::Active.to_string(), "active");
    assert_eq!(UserStatus::Suspended.to_string(), "suspended");
    assert_eq!(UserStatus::Pending.to_string(), "pending");

    assert!(matches!(
        UserStatus::from_str("active"),
        Ok(UserStatus::Active)
    ));
    assert!(matches!(
        UserStatus::from_str("suspended"),
        Ok(UserStatus::Suspended)
    ));
    assert!(matches!(
        UserStatus::from_str("pending"),
        Ok(UserStatus::Pending)
    ));
    assert!(UserStatus::from_str("unknown").is_err());
}

// ── AppType ───────────────────────────────────────────────────────────────────

#[test]
fn app_type_display_and_parse() {
    for (variant, s) in [
        (AppType::Web, "web"),
        (AppType::Spa, "spa"),
        (AppType::Native, "native"),
        (AppType::Machine, "machine"),
    ] {
        assert_eq!(variant.to_string(), s);
        assert!(AppType::from_str(s).is_ok());
    }
    assert!(AppType::from_str("invalid").is_err());
}

#[test]
fn app_type_roundtrip_serde() {
    for app_type in [
        AppType::Web,
        AppType::Spa,
        AppType::Native,
        AppType::Machine,
    ] {
        let json = serde_json::to_string(&app_type).unwrap();
        let back: AppType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&app_type),
            std::mem::discriminant(&back)
        );
    }
}

// ── IdpType ───────────────────────────────────────────────────────────────────

#[test]
fn idp_type_display_and_parse() {
    for (variant, s) in [
        (IdpType::Local, "local"),
        (IdpType::Oidc, "oidc"),
        (IdpType::Oauth2, "oauth2"),
        (IdpType::Ldap, "ldap"),
    ] {
        assert_eq!(variant.to_string(), s);
        assert!(IdpType::from_str(s).is_ok());
    }
    assert!(IdpType::from_str("saml").is_err());
}

// ── User ──────────────────────────────────────────────────────────────────────

fn make_user() -> User {
    User {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        email: "alice@example.com".into(),
        email_verified: true,
        name: Some("Alice".into()),
        given_name: Some("Alice".into()),
        family_name: Some("Smith".into()),
        picture_url: None,
        status: UserStatus::Active,
        attributes: serde_json::json!({}),
        created_at: now(),
        updated_at: now(),
        last_login_at: None,
        deleted_at: None,
    }
}

#[test]
fn user_roundtrip_serde() {
    let user = make_user();
    let json = serde_json::to_string(&user).unwrap();
    let back: User = serde_json::from_str(&json).unwrap();
    assert_eq!(user.id, back.id);
    assert_eq!(user.email, back.email);
    assert_eq!(user.email_verified, back.email_verified);
    assert!(matches!(back.status, UserStatus::Active));
}

// ── Tenant ────────────────────────────────────────────────────────────────────

#[test]
fn tenant_roundtrip_serde() {
    let tenant = Tenant {
        id: Uuid::new_v4(),
        name: "Acme Corp".into(),
        slug: "acme-corp".into(),
        settings: serde_json::json!({"branding": {"logo": "https://acme.example.com/logo.png"}}),
        created_at: now(),
        updated_at: now(),
        deleted_at: None,
    };
    let json = serde_json::to_string(&tenant).unwrap();
    let back: Tenant = serde_json::from_str(&json).unwrap();
    assert_eq!(tenant.id, back.id);
    assert_eq!(tenant.slug, back.slug);
}

// ── Application ───────────────────────────────────────────────────────────────

#[test]
fn application_roundtrip_serde() {
    let app = Application {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        name: "My App".into(),
        client_id: "my-app-client-id".into(),
        client_secret_hash: Some("hash-goes-here".into()),
        app_type: AppType::Web,
        redirect_uris: vec!["https://app.example.com/callback".into()],
        allowed_scopes: vec!["openid".into(), "email".into(), "profile".into()],
        grant_types: vec!["authorization_code".into(), "refresh_token".into()],
        access_token_ttl: 3600,
        refresh_token_ttl: 2592000,
        claim_prefix: "my-app".into(),
        created_at: now(),
        updated_at: now(),
        deleted_at: None,
    };
    let json = serde_json::to_string(&app).unwrap();
    let back: Application = serde_json::from_str(&json).unwrap();
    assert_eq!(app.id, back.id);
    assert_eq!(app.redirect_uris, back.redirect_uris);
    assert!(matches!(back.app_type, AppType::Web));
    assert_eq!(back.claim_prefix, "my-app");
}

// ── Claim definitions & validation ────────────────────────────────────────────

#[test]
fn claim_type_display_and_parse() {
    assert_eq!(ClaimType::Scalar.as_str(), "scalar");
    assert_eq!(ClaimType::Multi.as_str(), "multi");
    assert!(matches!(
        ClaimType::from_str("scalar"),
        Ok(ClaimType::Scalar)
    ));
    assert!(matches!(ClaimType::from_str("multi"), Ok(ClaimType::Multi)));
    assert!(ClaimType::from_str("other").is_err());
}

#[test]
fn claim_definition_roundtrip_serde() {
    let def = ClaimDefinition {
        id: Uuid::new_v4(),
        application_id: Uuid::new_v4(),
        key: "roles".into(),
        claim_type: ClaimType::Multi,
        description: Some("User roles for billing app".into()),
        created_at: now(),
        updated_at: now(),
    };
    let json = serde_json::to_string(&def).unwrap();
    let back: ClaimDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(def.id, back.id);
    assert_eq!(back.claim_type, ClaimType::Multi);
}

#[test]
fn validate_claim_prefix_rejects_reserved_and_invalid() {
    assert!(validate_claim_prefix("").is_err());
    assert!(validate_claim_prefix("sub").is_err());
    assert!(validate_claim_prefix("iss").is_err());
    assert!(validate_claim_prefix("has space").is_err());
    assert!(validate_claim_prefix("colon:bad").is_err());
    assert!(validate_claim_prefix("ok-prefix_1").is_ok());
}

#[test]
fn validate_claim_key_rules() {
    assert!(validate_claim_key("").is_err());
    assert!(validate_claim_key("bad space").is_err());
    assert!(validate_claim_key("plan").is_ok());
    assert!(validate_claim_key("roles_v2").is_ok());
}

// ── Session ───────────────────────────────────────────────────────────────────

#[test]
fn session_is_valid_checks_expiry_and_revocation() {
    let expires_at = OffsetDateTime::now_utc() + time::Duration::hours(1);
    let session = Session {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        idp_id: None,
        ip_address: None,
        user_agent: None,
        created_at: now(),
        expires_at,
        revoked_at: None,
    };
    assert!(session.is_valid());

    let expired = Session {
        expires_at: OffsetDateTime::now_utc() - time::Duration::seconds(1),
        ..session.clone()
    };
    assert!(!expired.is_valid());

    let revoked = Session {
        revoked_at: Some(OffsetDateTime::now_utc()),
        ..session.clone()
    };
    assert!(!revoked.is_valid());
}

#[test]
fn session_roundtrip_serde() {
    let session = Session {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        idp_id: Some("google".into()),
        ip_address: Some("127.0.0.1".into()),
        user_agent: Some("Mozilla/5.0".into()),
        created_at: now(),
        expires_at: OffsetDateTime::now_utc() + time::Duration::hours(24),
        revoked_at: None,
    };
    let json = serde_json::to_string(&session).unwrap();
    let back: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(session.id, back.id);
}

// ── IdpConfig ─────────────────────────────────────────────────────────────────

#[test]
fn idp_config_roundtrip_serde() {
    let config = IdpConfig {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        provider_type: IdpType::Oidc,
        name: "Google".into(),
        enabled: true,
        config: serde_json::json!({
            "client_id": "google-client-id",
            "client_secret": "secret",
            "discovery_url": "https://accounts.google.com/.well-known/openid-configuration"
        }),
        created_at: now(),
        updated_at: now(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: IdpConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.id, back.id);
    assert!(matches!(back.provider_type, IdpType::Oidc));
}

// ── AuditEvent ────────────────────────────────────────────────────────────────

#[test]
fn audit_event_roundtrip_serde() {
    let event = AuditEvent {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        event_type: "user.login".into(),
        actor_id: Some(Uuid::new_v4()),
        target_id: None,
        ip_address: Some("10.0.0.1".into()),
        metadata: serde_json::json!({"method": "password"}),
        created_at: now(),
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AuditEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event.id, back.id);
    assert_eq!(event.event_type, back.event_type);
}

// ── FederatedIdentity ─────────────────────────────────────────────────────────

#[test]
fn federated_identity_roundtrip_serde() {
    let fi = FederatedIdentity {
        provider_user_id: "google|1234567890".into(),
        email: "alice@gmail.com".into(),
        email_verified: true,
        name: Some("Alice Smith".into()),
        picture: Some("https://lh3.googleusercontent.com/photo.jpg".into()),
        raw_claims: serde_json::json!({"sub": "1234567890", "hd": "gmail.com"}),
    };
    let json = serde_json::to_string(&fi).unwrap();
    let back: FederatedIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(fi.email, back.email);
    assert_eq!(fi.email_verified, back.email_verified);
}

// ── Identity ──────────────────────────────────────────────────────────────────

#[test]
fn identity_roundtrip_serde() {
    let identity = Identity {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        provider: "google".into(),
        provider_user_id: "google|1234567890".into(),
        email: "alice@gmail.com".into(),
        raw_claims: serde_json::json!({"sub": "1234567890", "email": "alice@gmail.com"}),
        created_at: now(),
        updated_at: now(),
    };
    let json = serde_json::to_string(&identity).unwrap();
    let back: Identity = serde_json::from_str(&json).unwrap();
    assert_eq!(identity.id, back.id);
    assert_eq!(identity.provider, back.provider);
    assert_eq!(identity.provider_user_id, back.provider_user_id);
    assert_eq!(identity.email, back.email);
}

#[test]
fn identity_clone_equality() {
    let identity = Identity {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        provider: "github".into(),
        provider_user_id: "gh|999".into(),
        email: "bob@github.com".into(),
        raw_claims: serde_json::json!({}),
        created_at: now(),
        updated_at: now(),
    };
    assert_eq!(identity, identity.clone());
}

// ── RefreshToken ──────────────────────────────────────────────────────────────

#[test]
fn refresh_token_roundtrip_serde() {
    let token = RefreshToken {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        application_id: Uuid::new_v4(),
        token_hash: "sha256hashgoeshere".into(),
        scope: "openid email profile".into(),
        previous_id: None,
        created_at: now(),
        expires_at: now() + time::Duration::days(30),
        revoked_at: None,
    };
    let json = serde_json::to_string(&token).unwrap();
    let back: RefreshToken = serde_json::from_str(&json).unwrap();
    assert_eq!(token.id, back.id);
    assert_eq!(token.token_hash, back.token_hash);
    assert_eq!(token.scope, back.scope);
    assert!(back.revoked_at.is_none());
}

#[test]
fn refresh_token_with_rotation_chain() {
    let parent_id = Uuid::new_v4();
    let token = RefreshToken {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        application_id: Uuid::new_v4(),
        token_hash: "newhash".into(),
        scope: "openid".into(),
        previous_id: Some(parent_id),
        created_at: now(),
        expires_at: now() + time::Duration::days(30),
        revoked_at: None,
    };
    let json = serde_json::to_string(&token).unwrap();
    let back: RefreshToken = serde_json::from_str(&json).unwrap();
    assert_eq!(back.previous_id, Some(parent_id));
}

// ── IdpType (serde) ───────────────────────────────────────────────────────────

#[test]
fn idp_type_roundtrip_serde() {
    for idp_type in [IdpType::Local, IdpType::Oidc, IdpType::Oauth2, IdpType::Ldap] {
        let json = serde_json::to_string(&idp_type).unwrap();
        let back: IdpType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&idp_type),
            std::mem::discriminant(&back)
        );
    }
}

// ── UserStatus (extra edge cases) ─────────────────────────────────────────────

#[test]
fn user_status_case_sensitive_parse() {
    assert!(UserStatus::from_str("Active").is_err());
    assert!(UserStatus::from_str("ACTIVE").is_err());
    assert!(UserStatus::from_str("").is_err());
}

// ── Session (extra) ───────────────────────────────────────────────────────────

#[test]
fn session_both_expired_and_revoked_is_invalid() {
    let session = Session {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        idp_id: None,
        ip_address: None,
        user_agent: None,
        created_at: now(),
        expires_at: now() - time::Duration::seconds(1),
        revoked_at: Some(now()),
    };
    assert!(!session.is_valid());
}
