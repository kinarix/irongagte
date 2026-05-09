use async_trait::async_trait;
use irongate_core::{errors::StoreError, AuditEvent};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_row_err};

pub struct SqliteAuditRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_audit_event(row: &sqlx::sqlite::SqliteRow) -> Result<AuditEvent, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let actor_id: Option<String> = row.try_get("actor_id").map_err(map_row_err)?;
    let actor_id = actor_id
        .map(|s| {
            Uuid::parse_str(&s).map_err(|_| StoreError::Database("bad uuid: actor_id".into()))
        })
        .transpose()?;

    let target_id: Option<String> = row.try_get("target_id").map_err(map_row_err)?;
    let target_id = target_id
        .map(|s| {
            Uuid::parse_str(&s).map_err(|_| StoreError::Database("bad uuid: target_id".into()))
        })
        .transpose()?;

    let metadata_json: String = row.try_get("metadata").map_err(map_row_err)?;
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_json).map_err(map_json_err("metadata"))?;

    Ok(AuditEvent {
        id,
        tenant_id,
        event_type: row.try_get("event_type").map_err(map_row_err)?,
        actor_id,
        target_id,
        ip_address: row.try_get("ip_address").map_err(map_row_err)?,
        metadata,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::AuditRepository for SqliteAuditRepo {
    async fn record(&self, event: AuditEvent) -> Result<(), StoreError> {
        let metadata =
            serde_json::to_string(&event.metadata).unwrap_or_else(|_| "{}".into());

        sqlx::query(
            "INSERT INTO audit_events
             (id, tenant_id, event_type, actor_id, target_id, ip_address, metadata, created_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(event.id.to_string())
        .bind(event.tenant_id.to_string())
        .bind(&event.event_type)
        .bind(event.actor_id.map(|u| u.to_string()))
        .bind(event.target_id.map(|u| u.to_string()))
        .bind(&event.ip_address)
        .bind(&metadata)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEvent>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM audit_events
             WHERE tenant_id = ?
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(tenant_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_audit_event).collect()
    }
}
