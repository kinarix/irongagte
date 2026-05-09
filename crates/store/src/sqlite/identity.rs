use async_trait::async_trait;
use irongate_core::{errors::StoreError, Identity};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_row_err};

pub struct SqliteIdentityRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_identity(row: &sqlx::sqlite::SqliteRow) -> Result<Identity, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let user_id_str: String = row.try_get("user_id").map_err(map_row_err)?;
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| StoreError::Database("bad uuid: user_id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let raw_claims_json: String = row.try_get("raw_claims").map_err(map_row_err)?;
    let raw_claims: serde_json::Value =
        serde_json::from_str(&raw_claims_json).map_err(map_json_err("raw_claims"))?;

    Ok(Identity {
        id,
        user_id,
        tenant_id,
        provider: row.try_get("provider").map_err(map_row_err)?,
        provider_user_id: row.try_get("provider_user_id").map_err(map_row_err)?,
        email: row.try_get("email").map_err(map_row_err)?,
        raw_claims,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::IdentityRepository for SqliteIdentityRepo {
    async fn create(&self, identity: Identity) -> Result<Identity, StoreError> {
        let raw_claims =
            serde_json::to_string(&identity.raw_claims).unwrap_or_else(|_| "{}".into());

        sqlx::query(
            "INSERT INTO identities
             (id, user_id, tenant_id, provider, provider_user_id, email, raw_claims,
              created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?,?)",
        )
        .bind(identity.id.to_string())
        .bind(identity.user_id.to_string())
        .bind(identity.tenant_id.to_string())
        .bind(&identity.provider)
        .bind(&identity.provider_user_id)
        .bind(&identity.email)
        .bind(&raw_claims)
        .bind(identity.created_at)
        .bind(identity.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_provider(&identity.provider, &identity.provider_user_id, identity.tenant_id)
            .await
    }

    async fn get_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
        tenant_id: Uuid,
    ) -> Result<Identity, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM identities
             WHERE provider = ? AND provider_user_id = ? AND tenant_id = ?",
        )
        .bind(provider)
        .bind(provider_user_id)
        .bind(tenant_id.to_string())
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
        let rows = sqlx::query(
            "SELECT * FROM identities WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_identity).collect()
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM identities WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
