use async_trait::async_trait;
use irongate_core::{errors::StoreError, MagicLink};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqliteMagicLinkRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_magic_link(row: &sqlx::sqlite::SqliteRow) -> Result<MagicLink, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let user_id_str: String = row.try_get("user_id").map_err(map_row_err)?;
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| StoreError::Database("bad uuid: user_id".into()))?;

    Ok(MagicLink {
        id,
        tenant_id,
        user_id,
        token_hash: row.try_get("token_hash").map_err(map_row_err)?,
        expires_at: row.try_get("expires_at").map_err(map_row_err)?,
        used_at: row.try_get("used_at").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::MagicLinkRepository for SqliteMagicLinkRepo {
    async fn create(&self, link: MagicLink) -> Result<MagicLink, StoreError> {
        sqlx::query(
            "INSERT INTO magic_links
             (id, tenant_id, user_id, token_hash, expires_at, used_at, created_at)
             VALUES (?,?,?,?,?,?,?)",
        )
        .bind(link.id.to_string())
        .bind(link.tenant_id.to_string())
        .bind(link.user_id.to_string())
        .bind(&link.token_hash)
        .bind(link.expires_at)
        .bind(link.used_at)
        .bind(link.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(link)
    }

    async fn get_by_token_hash(
        &self,
        token_hash: &str,
        tenant_id: Uuid,
    ) -> Result<MagicLink, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM magic_links WHERE token_hash = ? AND tenant_id = ? AND used_at IS NULL",
        )
        .bind(token_hash)
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_magic_link(&r),
            None => Err(StoreError::NotFound("magic link".into())),
        }
    }

    async fn mark_used(&self, id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let rows_affected = sqlx::query(
            "UPDATE magic_links SET used_at = ? WHERE id = ? AND used_at IS NULL",
        )
        .bind(now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?
        .rows_affected();

        if rows_affected == 0 {
            return Err(StoreError::NotFound(format!("magic link {id}")));
        }
        Ok(())
    }

    async fn delete_expired(&self, tenant_id: Uuid) -> Result<u64, StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let rows_affected = sqlx::query(
            "DELETE FROM magic_links WHERE tenant_id = ? AND expires_at < ?",
        )
        .bind(tenant_id.to_string())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?
        .rows_affected();
        Ok(rows_affected)
    }
}
