use async_trait::async_trait;
use irongate_core::{errors::StoreError, AppType, Application};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_parse_err, map_row_err};

pub struct PgApplicationRepo {
    pub(crate) pool: PgPool,
}

fn row_to_application(row: &sqlx::postgres::PgRow) -> Result<Application, StoreError> {
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
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        client_id: row.try_get("client_id").map_err(map_row_err)?,
        client_secret_hash: row.try_get("client_secret_hash").map_err(map_row_err)?,
        app_type,
        redirect_uris,
        allowed_scopes,
        grant_types,
        access_token_ttl: row.try_get("access_token_ttl").map_err(map_row_err)?,
        refresh_token_ttl: row.try_get("refresh_token_ttl").map_err(map_row_err)?,
        claim_prefix: row.try_get("claim_prefix").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::ApplicationRepository for PgApplicationRepo {
    async fn create(&self, app: Application) -> Result<Application, StoreError> {
        let redirect_uris = serde_json::to_string(&app.redirect_uris)
            .map_err(map_json_err("redirect_uris"))?;
        let allowed_scopes = serde_json::to_string(&app.allowed_scopes)
            .map_err(map_json_err("allowed_scopes"))?;
        let grant_types = serde_json::to_string(&app.grant_types)
            .map_err(map_json_err("grant_types"))?;

        let row = sqlx::query(
            "INSERT INTO applications
             (id, tenant_id, name, client_id, client_secret_hash, app_type,
              redirect_uris, allowed_scopes, grant_types,
              access_token_ttl, refresh_token_ttl, claim_prefix, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
             RETURNING *",
        )
        .bind(app.id)
        .bind(app.tenant_id)
        .bind(&app.name)
        .bind(&app.client_id)
        .bind(&app.client_secret_hash)
        .bind(app.app_type.to_string())
        .bind(&redirect_uris)
        .bind(&allowed_scopes)
        .bind(&grant_types)
        .bind(app.access_token_ttl)
        .bind(app.refresh_token_ttl)
        .bind(&app.claim_prefix)
        .bind(app.created_at)
        .bind(app.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_application(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Application, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM applications
             WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(tenant_id)
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
             WHERE client_id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(client_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_application(&row)
    }

    async fn update(&self, app: Application) -> Result<Application, StoreError> {
        let redirect_uris = serde_json::to_string(&app.redirect_uris)
            .map_err(map_json_err("redirect_uris"))?;
        let allowed_scopes = serde_json::to_string(&app.allowed_scopes)
            .map_err(map_json_err("allowed_scopes"))?;
        let grant_types = serde_json::to_string(&app.grant_types)
            .map_err(map_json_err("grant_types"))?;

        let row = sqlx::query(
            "UPDATE applications
             SET name = $1, client_secret_hash = $2, app_type = $3,
                 redirect_uris = $4, allowed_scopes = $5, grant_types = $6,
                 access_token_ttl = $7, refresh_token_ttl = $8,
                 claim_prefix = $9, updated_at = $10
             WHERE id = $11 AND tenant_id = $12 AND deleted_at IS NULL
             RETURNING *",
        )
        .bind(&app.name)
        .bind(&app.client_secret_hash)
        .bind(app.app_type.to_string())
        .bind(&redirect_uris)
        .bind(&allowed_scopes)
        .bind(&grant_types)
        .bind(app.access_token_ttl)
        .bind(app.refresh_token_ttl)
        .bind(&app.claim_prefix)
        .bind(app.updated_at)
        .bind(app.id)
        .bind(app.tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_application(&row)
    }

    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE applications SET deleted_at = $1
             WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(id)
        .bind(tenant_id)
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
             WHERE tenant_id = $1 AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_application).collect()
    }
}
