//! Signing-key lifecycle helpers: load-or-create on boot, periodic rotation,
//! and per-replica hot refresh.
//!
//! Why this lives in `api` and not in `crypto`: it composes the
//! `SigningKeyRepository` (in core/store) with the key generators (in crypto).
//! Neither bottom layer should know about the other; the orchestration
//! belongs at the binary boundary.

use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use irongate_core::{
    errors::StoreError, repositories::SigningKeyRepository, KeyAlgorithm, SigningKeyRecord,
};
use irongate_crypto::keys::{generate_ec_key, generate_rsa_key};
use time::OffsetDateTime;
use tracing::{info, warn};

/// Rotation policy. Times are evaluated against the *current* key's
/// `created_at` and `expires_at`.
#[derive(Debug, Clone, Copy)]
pub struct RotationPolicy {
    /// Rotate when the current key's age exceeds this value.
    pub max_age: time::Duration,
    /// Rotate when the current key's expiry is within this window.
    pub expiry_grace: time::Duration,
    /// Lifetime to assign to a newly generated key.
    pub new_key_ttl: time::Duration,
}

/// Ensures a usable global signing key exists in the database. If one is found,
/// it is returned. Otherwise a fresh RSA-2048 key is generated, persisted, and
/// returned. The on-disk row is the source of truth — the in-memory cache is
/// derived from it.
pub async fn load_or_create(
    repo: &Arc<dyn SigningKeyRepository>,
    ttl_days: i64,
) -> Result<SigningKeyRecord, StoreError> {
    if let Some(existing) = repo.current(None).await? {
        info!(kid = %existing.id, "loaded existing signing key from store");
        return Ok(existing);
    }
    let fresh = generate_rsa_key(None, ttl_days)
        .map_err(|e| StoreError::Database(format!("key generation failed: {e}")))?;
    let stored = repo.create(fresh).await?;
    info!(kid = %stored.id, "generated and persisted new signing key");
    Ok(stored)
}

/// Decides whether the given key should be rotated according to the policy.
fn needs_rotation(current: &SigningKeyRecord, policy: &RotationPolicy) -> bool {
    let now = OffsetDateTime::now_utc();
    let age = now - current.created_at;
    let until_expiry = current.expires_at - now;
    age >= policy.max_age || until_expiry <= policy.expiry_grace
}

/// One pass of the rotation loop. Acquires the cross-replica advisory lock,
/// re-checks the policy under the lock (another replica may have rotated
/// already), and rotates if needed. Errors are logged but not returned — the
/// loop must keep running.
async fn try_rotate_once(
    repo: &Arc<dyn SigningKeyRepository>,
    policy: &RotationPolicy,
    algorithm: KeyAlgorithm,
) {
    let got = match repo.try_acquire_rotation_lock().await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "signing key: failed to acquire rotation lock");
            return;
        }
    };
    if !got {
        return;
    }

    let result = (async {
        let current = match repo.current(None).await? {
            Some(k) => k,
            None => {
                // No key at all — load_or_create will materialise one on next boot.
                return Ok::<_, StoreError>(());
            }
        };
        if !needs_rotation(&current, policy) {
            return Ok(());
        }
        let ttl_days = policy.new_key_ttl.whole_days();
        let fresh = match algorithm {
            KeyAlgorithm::Rs256 => generate_rsa_key(None, ttl_days),
            KeyAlgorithm::Es256 => generate_ec_key(None, ttl_days),
        }
        .map_err(|e| StoreError::Database(format!("key generation failed: {e}")))?;
        let stored = repo.create(fresh).await?;
        repo.retire(current.id).await?;
        info!(
            old_kid = %current.id,
            new_kid = %stored.id,
            "rotated signing key"
        );
        Ok(())
    })
    .await;

    if let Err(e) = result {
        warn!(error = %e, "signing key: rotation pass failed");
    }
    if let Err(e) = repo.release_rotation_lock().await {
        warn!(error = %e, "signing key: failed to release rotation lock");
    }
}

/// Long-running task: every `tick` interval, checks policy and rotates the
/// global signing key if needed. Designed to run on every replica concurrently;
/// the advisory lock serialises actual rotation to one replica at a time.
pub async fn rotation_loop(
    repo: Arc<dyn SigningKeyRepository>,
    policy: RotationPolicy,
    algorithm: KeyAlgorithm,
    tick: Duration,
) {
    let mut interval = tokio::time::interval(tick);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        try_rotate_once(&repo, &policy, algorithm).await;
    }
}

/// Long-running task: every `tick`, refreshes the cached current key from the
/// store and atomically swaps it into `cache`. This is how a rotation triggered
/// by another replica (or by the CLI) propagates to the hot path of *this*
/// replica without a restart.
pub async fn refresh_loop(
    repo: Arc<dyn SigningKeyRepository>,
    cache: Arc<ArcSwap<SigningKeyRecord>>,
    tick: Duration,
) {
    let mut interval = tokio::time::interval(tick);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        match repo.current(None).await {
            Ok(Some(latest)) => {
                let cached = cache.load();
                if cached.id != latest.id {
                    info!(
                        old_kid = %cached.id,
                        new_kid = %latest.id,
                        "signing key cache: picked up rotation"
                    );
                    cache.store(Arc::new(latest));
                }
            }
            Ok(None) => {
                warn!("signing key cache: no current key in store");
            }
            Err(e) => {
                warn!(error = %e, "signing key cache: refresh failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(created_offset_days: i64, ttl_days: i64) -> SigningKeyRecord {
        let now = OffsetDateTime::now_utc();
        SigningKeyRecord {
            id: uuid::Uuid::new_v4(),
            tenant_id: None,
            algorithm: KeyAlgorithm::Rs256,
            private_key_pem: String::new(),
            public_key_pem: String::new(),
            created_at: now - time::Duration::days(created_offset_days),
            expires_at: now - time::Duration::days(created_offset_days)
                + time::Duration::days(ttl_days),
            retired_at: None,
        }
    }

    fn policy() -> RotationPolicy {
        RotationPolicy {
            max_age: time::Duration::days(30),
            expiry_grace: time::Duration::days(7),
            new_key_ttl: time::Duration::days(365),
        }
    }

    #[test]
    fn fresh_key_does_not_need_rotation() {
        assert!(!needs_rotation(&make_key(1, 365), &policy()));
    }

    #[test]
    fn old_key_triggers_max_age() {
        assert!(needs_rotation(&make_key(31, 365), &policy()));
    }

    #[test]
    fn key_near_expiry_triggers_grace() {
        // Created 360 days ago with a 365-day TTL — expires in 5 days, inside grace window.
        assert!(needs_rotation(&make_key(360, 365), &policy()));
    }
}
