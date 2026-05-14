use async_trait::async_trait;
use irongate_core::{
    errors::StoreError,
    types::{Operator, OperatorCredentials, OperatorStatus},
};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgOperatorRepo {
    pub(crate) pool: PgPool,
}

pub struct PgOperatorCredentialsRepo {
    pub(crate) pool: PgPool,
}

fn row_to_operator(row: &sqlx::postgres::PgRow) -> Result<Operator, StoreError> {
    let status_str: String = row.try_get("status").map_err(map_row_err)?;
    let status: OperatorStatus = status_str
        .parse()
        .map_err(|_| StoreError::Database(format!("unknown operator status '{status_str}'")))?;
    Ok(Operator {
        id: row.try_get("id").map_err(map_row_err)?,
        email: row.try_get("email").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        status,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        last_login_at: row.try_get("last_login_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

fn row_to_creds(row: &sqlx::postgres::PgRow) -> Result<OperatorCredentials, StoreError> {
    Ok(OperatorCredentials {
        operator_id: row.try_get("operator_id").map_err(map_row_err)?,
        password_hash: row.try_get("password_hash").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::OperatorRepository for PgOperatorRepo {
    async fn create(&self, op: Operator) -> Result<Operator, StoreError> {
        let row = sqlx::query(
            "INSERT INTO operators (id, email, name, status, created_at, updated_at, last_login_at, deleted_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING *",
        )
        .bind(op.id)
        .bind(&op.email)
        .bind(&op.name)
        .bind(op.status.as_str())
        .bind(op.created_at)
        .bind(op.updated_at)
        .bind(op.last_login_at)
        .bind(op.deleted_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_operator(&row)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Operator, StoreError> {
        let row = sqlx::query("SELECT * FROM operators WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_err)?;
        match row {
            Some(r) => row_to_operator(&r),
            None => Err(StoreError::NotFound(format!("operator {id}"))),
        }
    }

    async fn get_by_email(&self, email: &str) -> Result<Operator, StoreError> {
        let row = sqlx::query("SELECT * FROM operators WHERE email = $1 AND deleted_at IS NULL")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_err)?;
        match row {
            Some(r) => row_to_operator(&r),
            None => Err(StoreError::NotFound(format!("operator with email {email}"))),
        }
    }

    async fn update(&self, op: Operator) -> Result<Operator, StoreError> {
        let row = sqlx::query(
            "UPDATE operators
             SET email = $1, name = $2, status = $3, updated_at = $4
             WHERE id = $5 AND deleted_at IS NULL
             RETURNING *",
        )
        .bind(&op.email)
        .bind(&op.name)
        .bind(op.status.as_str())
        .bind(op.updated_at)
        .bind(op.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        match row {
            Some(r) => row_to_operator(&r),
            None => Err(StoreError::NotFound(format!("operator {}", op.id))),
        }
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError> {
        sqlx::query("UPDATE operators SET deleted_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Operator>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM operators WHERE deleted_at IS NULL
             ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator).collect()
    }

    async fn touch_last_login(&self, id: Uuid) -> Result<(), StoreError> {
        sqlx::query("UPDATE operators SET last_login_at = $1 WHERE id = $2")
            .bind(OffsetDateTime::now_utc())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}

#[async_trait]
impl irongate_core::repositories::OperatorCredentialsRepository for PgOperatorCredentialsRepo {
    async fn create(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError> {
        let row = sqlx::query(
            "INSERT INTO operator_credentials (operator_id, password_hash, created_at, updated_at)
             VALUES ($1,$2,$3,$4)
             RETURNING *",
        )
        .bind(creds.operator_id)
        .bind(&creds.password_hash)
        .bind(creds.created_at)
        .bind(creds.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_creds(&row)
    }

    async fn get_by_operator_id(
        &self,
        operator_id: Uuid,
    ) -> Result<OperatorCredentials, StoreError> {
        let row = sqlx::query("SELECT * FROM operator_credentials WHERE operator_id = $1")
            .bind(operator_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_err)?;
        match row {
            Some(r) => row_to_creds(&r),
            None => Err(StoreError::NotFound(format!(
                "credentials for operator {operator_id}"
            ))),
        }
    }

    async fn update(&self, creds: OperatorCredentials) -> Result<OperatorCredentials, StoreError> {
        let row = sqlx::query(
            "UPDATE operator_credentials
             SET password_hash = $1, updated_at = $2
             WHERE operator_id = $3
             RETURNING *",
        )
        .bind(&creds.password_hash)
        .bind(creds.updated_at)
        .bind(creds.operator_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        match row {
            Some(r) => row_to_creds(&r),
            None => Err(StoreError::NotFound(format!(
                "credentials for operator {}",
                creds.operator_id
            ))),
        }
    }

    async fn delete(&self, operator_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM operator_credentials WHERE operator_id = $1")
            .bind(operator_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
