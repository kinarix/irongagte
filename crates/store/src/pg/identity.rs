use async_trait::async_trait;
use irongate_core::{errors::StoreError, Identity};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgIdentityRepo {
    pub(crate) pool: PgPool,
}

fn row_to_identity(row: &sqlx::postgres::PgRow) -> Result<Identity, StoreError> {
    Ok(Identity {
        id: row.try_get("id").map_err(map_row_err)?,
        user_id: row.try_get("user_id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        provider: row.try_get("provider").map_err(map_row_err)?,
        provider_user_id: row.try_get("provider_user_id").map_err(map_row_err)?,
        email: row.try_get("email").map_err(map_row_err)?,
        raw_claims: row.try_get("raw_claims").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::IdentityRepository for PgIdentityRepo {
    async fn create(&self, identity: Identity) -> Result<Identity, StoreError> {
        let raw_claims =
            serde_json::to_string(&identity.raw_claims).unwrap_or_else(|_| "{}".into());
        let row = sqlx::query(
            "INSERT INTO identities
             (id, user_id, tenant_id, provider, provider_user_id, email, raw_claims, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7::jsonb,$8,$9)
             RETURNING *",
        )
        .bind(identity.id)
        .bind(identity.user_id)
        .bind(identity.tenant_id)
        .bind(&identity.provider)
        .bind(&identity.provider_user_id)
        .bind(&identity.email)
        .bind(&raw_claims)
        .bind(identity.created_at)
        .bind(identity.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_identity(&row)
    }

    async fn get_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
        tenant_id: Uuid,
    ) -> Result<Identity, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM identities
             WHERE provider = $1 AND provider_user_id = $2 AND tenant_id = $3",
        )
        .bind(provider)
        .bind(provider_user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_identity(&row)
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Identity>, StoreError> {
        let rows = sqlx::query("SELECT * FROM identities WHERE user_id = $1 AND tenant_id = $2")
            .bind(user_id)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_err)?;
        rows.iter().map(row_to_identity).collect()
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM identities WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
