use async_trait::async_trait;
use irongate_core::{errors::StoreError, repositories::OperatorPermissionRepository, OperatorPermission};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgOperatorPermissionRepo {
    pub(crate) pool: PgPool,
}

fn row_to_operator_permission(row: &sqlx::postgres::PgRow) -> Result<OperatorPermission, StoreError> {
    Ok(OperatorPermission {
        id: row.try_get("id").map_err(map_row_err)?,
        resource: row.try_get("resource").map_err(map_row_err)?,
        action: row.try_get("action").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl OperatorPermissionRepository for PgOperatorPermissionRepo {
    async fn create(&self, p: OperatorPermission) -> Result<OperatorPermission, StoreError> {
        let row = sqlx::query(
            "INSERT INTO operator_permissions (id, resource, action, description, created_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(p.id)
        .bind(&p.resource)
        .bind(&p.action)
        .bind(&p.description)
        .bind(p.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_operator_permission(&row)
    }

    async fn list(&self) -> Result<Vec<OperatorPermission>, StoreError> {
        let rows = sqlx::query("SELECT * FROM operator_permissions ORDER BY resource, action")
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_permission).collect()
    }

    async fn get_by_id(&self, id: Uuid) -> Result<OperatorPermission, StoreError> {
        let row =
            sqlx::query("SELECT * FROM operator_permissions WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_err)?;
        row_to_operator_permission(&row)
    }

    async fn get_by_resource_action(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<OperatorPermission, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM operator_permissions WHERE resource = $1 AND action = $2",
        )
        .bind(resource)
        .bind(action)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_operator_permission(&row)
    }
}
