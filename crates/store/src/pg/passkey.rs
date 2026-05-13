use async_trait::async_trait;
use irongate_core::{errors::StoreError, PasskeyCredential};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgPasskeyRepo {
    pub(crate) pool: PgPool,
}

fn row_to_passkey(row: &sqlx::postgres::PgRow) -> Result<PasskeyCredential, StoreError> {
    Ok(PasskeyCredential {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        user_id: row.try_get("user_id").map_err(map_row_err)?,
        credential_id: row.try_get("credential_id").map_err(map_row_err)?,
        friendly_name: row.try_get("friendly_name").map_err(map_row_err)?,
        passkey_json: row.try_get("passkey_json").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        last_used_at: row.try_get("last_used_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::PasskeyRepository for PgPasskeyRepo {
    async fn create(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError> {
        let row = sqlx::query(
            "INSERT INTO passkeys
             (id, tenant_id, user_id, credential_id, friendly_name, passkey_json, created_at, last_used_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING *",
        )
        .bind(cred.id)
        .bind(cred.tenant_id)
        .bind(cred.user_id)
        .bind(&cred.credential_id)
        .bind(&cred.friendly_name)
        .bind(&cred.passkey_json)
        .bind(cred.created_at)
        .bind(cred.last_used_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_passkey(&row)
    }

    async fn get_by_credential_id(
        &self,
        credential_id: &str,
        tenant_id: Uuid,
    ) -> Result<PasskeyCredential, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM passkeys WHERE credential_id = $1 AND tenant_id = $2",
        )
        .bind(credential_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_passkey(&r),
            None => Err(StoreError::NotFound(format!("passkey {credential_id}"))),
        }
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<PasskeyCredential>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM passkeys WHERE user_id = $1 AND tenant_id = $2 ORDER BY created_at",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;

        rows.iter().map(row_to_passkey).collect()
    }

    async fn update(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError> {
        let row = sqlx::query(
            "UPDATE passkeys
             SET friendly_name = $1, passkey_json = $2, last_used_at = $3
             WHERE id = $4 AND tenant_id = $5
             RETURNING *",
        )
        .bind(&cred.friendly_name)
        .bind(&cred.passkey_json)
        .bind(cred.last_used_at)
        .bind(cred.id)
        .bind(cred.tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_passkey(&r),
            None => Err(StoreError::NotFound(format!("passkey {}", cred.id))),
        }
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM passkeys WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
