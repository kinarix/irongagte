use async_trait::async_trait;
use irongate_core::{errors::StoreError, PasskeyCredential};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqlitePasskeyRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_passkey(row: &sqlx::sqlite::SqliteRow) -> Result<PasskeyCredential, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let user_id_str: String = row.try_get("user_id").map_err(map_row_err)?;
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| StoreError::Database("bad uuid: user_id".into()))?;

    let passkey_json_str: String = row.try_get("passkey_json").map_err(map_row_err)?;
    let passkey_json: serde_json::Value = serde_json::from_str(&passkey_json_str)
        .map_err(|e| StoreError::Database(format!("bad passkey_json: {e}")))?;

    Ok(PasskeyCredential {
        id,
        tenant_id,
        user_id,
        credential_id: row.try_get("credential_id").map_err(map_row_err)?,
        friendly_name: row.try_get("friendly_name").map_err(map_row_err)?,
        passkey_json,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        last_used_at: row.try_get("last_used_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::PasskeyRepository for SqlitePasskeyRepo {
    async fn create(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError> {
        let passkey_json_str = serde_json::to_string(&cred.passkey_json)
            .map_err(|e| StoreError::Database(format!("serialize passkey_json: {e}")))?;
        sqlx::query(
            "INSERT INTO passkeys
             (id, tenant_id, user_id, credential_id, friendly_name, passkey_json, created_at, last_used_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(cred.id.to_string())
        .bind(cred.tenant_id.to_string())
        .bind(cred.user_id.to_string())
        .bind(&cred.credential_id)
        .bind(&cred.friendly_name)
        .bind(&passkey_json_str)
        .bind(cred.created_at)
        .bind(cred.last_used_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(cred)
    }

    async fn get_by_credential_id(
        &self,
        credential_id: &str,
        tenant_id: Uuid,
    ) -> Result<PasskeyCredential, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM passkeys WHERE credential_id = ? AND tenant_id = ?",
        )
        .bind(credential_id)
        .bind(tenant_id.to_string())
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
            "SELECT * FROM passkeys WHERE user_id = ? AND tenant_id = ? ORDER BY created_at",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;

        rows.iter().map(row_to_passkey).collect()
    }

    async fn update(&self, cred: PasskeyCredential) -> Result<PasskeyCredential, StoreError> {
        let passkey_json_str = serde_json::to_string(&cred.passkey_json)
            .map_err(|e| StoreError::Database(format!("serialize passkey_json: {e}")))?;
        let rows_affected = sqlx::query(
            "UPDATE passkeys
             SET friendly_name = ?, passkey_json = ?, last_used_at = ?
             WHERE id = ? AND tenant_id = ?",
        )
        .bind(&cred.friendly_name)
        .bind(&passkey_json_str)
        .bind(cred.last_used_at)
        .bind(cred.id.to_string())
        .bind(cred.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?
        .rows_affected();

        if rows_affected == 0 {
            return Err(StoreError::NotFound(format!("passkey {}", cred.id)));
        }
        Ok(cred)
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM passkeys WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
