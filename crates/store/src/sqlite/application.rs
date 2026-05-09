use async_trait::async_trait;
use irongate_core::{errors::StoreError, AppType, Application};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_parse_err, map_row_err};

pub struct SqliteApplicationRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_application(row: &sqlx::sqlite::SqliteRow) -> Result<Application, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let app_type_str: String = row.try_get("app_type").map_err(map_row_err)?;
    let app_type: AppType = app_type_str.parse().map_err(|_| map_parse_err("app_type"))?;

    let redirect_uris_json: String = row.try_get("redirect_uris").map_err(map_row_err)?;
    let redirect_uris: Vec<String> =
        serde_json::from_str(&redirect_uris_json).map_err(map_json_err("redirect_uris"))?;

    let allowed_scopes_json: String = row.try_get("allowed_scopes").map_err(map_row_err)?;
    let allowed_scopes: Vec<String> =
        serde_json::from_str(&allowed_scopes_json).map_err(map_json_err("allowed_scopes"))?;

    let grant_types_json: String = row.try_get("grant_types").map_err(map_row_err)?;
    let grant_types: Vec<String> =
        serde_json::from_str(&grant_types_json).map_err(map_json_err("grant_types"))?;

    Ok(Application {
        id,
        tenant_id,
        name: row.try_get("name").map_err(map_row_err)?,
        client_id: row.try_get("client_id").map_err(map_row_err)?,
        client_secret_hash: row.try_get("client_secret_hash").map_err(map_row_err)?,
        app_type,
        redirect_uris,
        allowed_scopes,
        grant_types,
        access_token_ttl: row.try_get("access_token_ttl").map_err(map_row_err)?,
        refresh_token_ttl: row.try_get("refresh_token_ttl").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::ApplicationRepository for SqliteApplicationRepo {
    async fn create(&self, app: Application) -> Result<Application, StoreError> {
        let redirect_uris =
            serde_json::to_string(&app.redirect_uris).map_err(map_json_err("redirect_uris"))?;
        let allowed_scopes =
            serde_json::to_string(&app.allowed_scopes).map_err(map_json_err("allowed_scopes"))?;
        let grant_types =
            serde_json::to_string(&app.grant_types).map_err(map_json_err("grant_types"))?;

        sqlx::query(
            "INSERT INTO applications
             (id, tenant_id, name, client_id, client_secret_hash, app_type,
              redirect_uris, allowed_scopes, grant_types,
              access_token_ttl, refresh_token_ttl, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(app.id.to_string())
        .bind(app.tenant_id.to_string())
        .bind(&app.name)
        .bind(&app.client_id)
        .bind(&app.client_secret_hash)
        .bind(app.app_type.to_string())
        .bind(&redirect_uris)
        .bind(&allowed_scopes)
        .bind(&grant_types)
        .bind(app.access_token_ttl)
        .bind(app.refresh_token_ttl)
        .bind(app.created_at)
        .bind(app.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(app.id, app.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Application, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM applications
             WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_application(&row)
    }

    async fn get_by_client_id(
        &self,
        client_id: &str,
        tenant_id: Uuid,
    ) -> Result<Application, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM applications
             WHERE client_id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(client_id)
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_application(&row)
    }

    async fn update(&self, app: Application) -> Result<Application, StoreError> {
        let redirect_uris =
            serde_json::to_string(&app.redirect_uris).map_err(map_json_err("redirect_uris"))?;
        let allowed_scopes =
            serde_json::to_string(&app.allowed_scopes).map_err(map_json_err("allowed_scopes"))?;
        let grant_types =
            serde_json::to_string(&app.grant_types).map_err(map_json_err("grant_types"))?;

        sqlx::query(
            "UPDATE applications
             SET name = ?, client_secret_hash = ?, app_type = ?,
                 redirect_uris = ?, allowed_scopes = ?, grant_types = ?,
                 access_token_ttl = ?, refresh_token_ttl = ?, updated_at = ?
             WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(&app.name)
        .bind(&app.client_secret_hash)
        .bind(app.app_type.to_string())
        .bind(&redirect_uris)
        .bind(&allowed_scopes)
        .bind(&grant_types)
        .bind(app.access_token_ttl)
        .bind(app.refresh_token_ttl)
        .bind(app.updated_at)
        .bind(app.id.to_string())
        .bind(app.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(app.id, app.tenant_id).await
    }

    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE applications SET deleted_at = ?
             WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(id.to_string())
        .bind(tenant_id.to_string())
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
    ) -> Result<Vec<Application>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM applications
             WHERE tenant_id = ? AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(tenant_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_application).collect()
    }
}
