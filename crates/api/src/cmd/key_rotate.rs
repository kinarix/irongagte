use std::sync::Arc;

use anyhow::{anyhow, Context};
use irongate_api::config::Settings;
use irongate_core::{repositories::SigningKeyRepository, KeyAlgorithm};
use irongate_crypto::keys::{generate_ec_key, generate_rsa_key};
use irongate_store::PgStore;

/// Force a signing-key rotation outside the normal cadence. Generates a fresh
/// key, persists it as the new current key, and retires the previous current
/// key. Running replicas pick up the new key within `refresh_interval_seconds`
/// (default 60s) — no restart required.
///
/// Usage:
///   irongate key rotate                # RS256 (default)
///   irongate key rotate --alg ES256    # ES256 variant
pub async fn run(args: &[String]) -> anyhow::Result<()> {
    let alg_str = flag(args, "--alg").unwrap_or("RS256");
    let algorithm: KeyAlgorithm = alg_str.parse().map_err(|e| anyhow!("invalid --alg: {e}"))?;

    let settings = Settings::load().context("failed to load configuration")?;

    let pg = PgStore::new(&settings.database.url, settings.database.max_connections)
        .await
        .context("failed to connect to database")?;
    pg.migrate().await.context("failed to run migrations")?;

    let repo: Arc<dyn SigningKeyRepository> = Arc::new(pg.signing_keys());

    // Snapshot the current key first so we can retire it after the new one is
    // safely persisted.
    let previous = repo.current(None).await?;

    let ttl_days = settings.signing_keys.ttl_days;
    let fresh = match algorithm {
        KeyAlgorithm::Rs256 => generate_rsa_key(None, ttl_days),
        KeyAlgorithm::Es256 => generate_ec_key(None, ttl_days),
    }
    .map_err(|e| anyhow!("key generation failed: {e}"))?;
    let fresh_id = fresh.id;
    repo.create(fresh)
        .await
        .context("failed to persist new key")?;

    if let Some(prev) = previous {
        repo.retire(prev.id)
            .await
            .context("failed to retire previous key")?;
        println!("retired previous key: {}", prev.id);
    }

    println!("new {} signing key: {}", algorithm, fresh_id);
    println!(
        "running replicas will pick up the new key within {}s",
        settings.signing_keys.refresh_interval_seconds
    );
    Ok(())
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().map(String::as_str);
        }
        if let Some(rest) = arg.strip_prefix(&format!("{name}=")) {
            return Some(rest);
        }
    }
    None
}
