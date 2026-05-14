use std::str::FromStr;

use async_trait::async_trait;
use irongate_core::{errors::StoreError, KeyAlgorithm, SigningKeyRecord};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

/// Stable advisory-lock key used to serialise rotation across replicas.
const ROTATION_LOCK_KEY: i64 = 0x_C0FFEE_C0FFEE;

pub struct PgSigningKeyRepo {
    pub(crate) pool: PgPool,
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> Result<SigningKeyRecord, StoreError> {
    let algorithm: String = row.try_get("algorithm").map_err(map_row_err)?;
    Ok(SigningKeyRecord {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        algorithm: KeyAlgorithm::from_str(&algorithm)
            .map_err(|e| StoreError::Database(e.to_string()))?,
        private_key_pem: row.try_get("private_key_pem").map_err(map_row_err)?,
        public_key_pem: row.try_get("public_key_pem").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        expires_at: row.try_get("expires_at").map_err(map_row_err)?,
        retired_at: row.try_get("retired_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::SigningKeyRepository for PgSigningKeyRepo {
    async fn list_publishable(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<SigningKeyRecord>, StoreError> {
        // Publish every key whose expiry is still in the future, including
        // retired ones, so in-flight tokens keep verifying through the grace
        // window. `tenant_id IS NOT DISTINCT FROM $1` makes the parameter
        // match both NULL (global) and a specific tenant id without needing
        // separate queries.
        let rows = sqlx::query(
            "SELECT * FROM signing_keys
             WHERE tenant_id IS NOT DISTINCT FROM $1
               AND expires_at > now()
             ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_record).collect()
    }

    async fn current(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<SigningKeyRecord>, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM signing_keys
             WHERE tenant_id IS NOT DISTINCT FROM $1
               AND retired_at IS NULL
               AND expires_at > now()
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        row.as_ref().map(row_to_record).transpose()
    }

    async fn create(&self, key: SigningKeyRecord) -> Result<SigningKeyRecord, StoreError> {
        sqlx::query(
            "INSERT INTO signing_keys
             (id, tenant_id, algorithm, private_key_pem, public_key_pem,
              created_at, expires_at, retired_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(key.id)
        .bind(key.tenant_id)
        .bind(key.algorithm.to_string())
        .bind(&key.private_key_pem)
        .bind(&key.public_key_pem)
        .bind(key.created_at)
        .bind(key.expires_at)
        .bind(key.retired_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(key)
    }

    async fn retire(&self, id: Uuid) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE signing_keys
             SET retired_at = now()
             WHERE id = $1 AND retired_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn try_acquire_rotation_lock(&self) -> Result<bool, StoreError> {
        let row = sqlx::query("SELECT pg_try_advisory_lock($1) AS got")
            .bind(ROTATION_LOCK_KEY)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row.try_get::<bool, _>("got").map_err(map_row_err)
    }

    async fn release_rotation_lock(&self) -> Result<(), StoreError> {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(ROTATION_LOCK_KEY)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
