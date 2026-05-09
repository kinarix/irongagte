use async_trait::async_trait;
use irongate_core::{errors::StoreError, AuditEvent};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgAuditRepo {
    pub(crate) pool: PgPool,
}

fn row_to_audit_event(row: &sqlx::postgres::PgRow) -> Result<AuditEvent, StoreError> {
    Ok(AuditEvent {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        event_type: row.try_get("event_type").map_err(map_row_err)?,
        actor_id: row.try_get("actor_id").map_err(map_row_err)?,
        target_id: row.try_get("target_id").map_err(map_row_err)?,
        ip_address: row.try_get("ip_address").map_err(map_row_err)?,
        metadata: row.try_get("metadata").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::AuditRepository for PgAuditRepo {
    async fn record(&self, event: AuditEvent) -> Result<(), StoreError> {
        let metadata =
            serde_json::to_string(&event.metadata).unwrap_or_else(|_| "{}".into());
        sqlx::query(
            "INSERT INTO audit_events
             (id, tenant_id, event_type, actor_id, target_id, ip_address, metadata, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7::jsonb,$8)",
        )
        .bind(event.id)
        .bind(event.tenant_id)
        .bind(&event.event_type)
        .bind(event.actor_id)
        .bind(event.target_id)
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
             WHERE tenant_id = $1
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_audit_event).collect()
    }
}
