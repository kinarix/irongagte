use async_trait::async_trait;
use irongate_core::{
    errors::StoreError,
    repositories::ResolvedUserClaim,
    types::{ClaimType, UserClaim},
};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct PgUserClaimRepo {
    pub(crate) pool: PgPool,
}

fn row_to_user_claim(row: &sqlx::postgres::PgRow) -> Result<UserClaim, StoreError> {
    Ok(UserClaim {
        user_id: row.try_get("user_id").map_err(map_row_err)?,
        claim_def_id: row.try_get("claim_def_id").map_err(map_row_err)?,
        value: row.try_get("value").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

fn row_to_resolved(row: &sqlx::postgres::PgRow) -> Result<ResolvedUserClaim, StoreError> {
    let claim_type_str: String = row.try_get("claim_type").map_err(map_row_err)?;
    let claim_type = ClaimType::from_str(&claim_type_str)
        .map_err(|_| map_parse_err("claim_type"))?;
    Ok(ResolvedUserClaim {
        claim_def_id: row.try_get("claim_def_id").map_err(map_row_err)?,
        claim_key: row.try_get("claim_key").map_err(map_row_err)?,
        claim_type,
        value: row.try_get("value").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::UserClaimRepository for PgUserClaimRepo {
    async fn assign(
        &self,
        user_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<UserClaim, StoreError> {
        let row = sqlx::query(
            "INSERT INTO user_claims (user_id, claim_def_id, value)
             VALUES ($1,$2,$3)
             ON CONFLICT (user_id, claim_def_id, value) DO UPDATE
                 SET value = EXCLUDED.value
             RETURNING *",
        )
        .bind(user_id)
        .bind(claim_def_id)
        .bind(value)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user_claim(&row)
    }

    async fn revoke(
        &self,
        user_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM user_claims
             WHERE user_id = $1 AND claim_def_id = $2 AND value = $3",
        )
        .bind(user_id)
        .bind(claim_def_id)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<UserClaim>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM user_claims WHERE user_id = $1 ORDER BY claim_def_id, value",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_user_claim).collect()
    }

    async fn list_for_user_in_app(
        &self,
        user_id: Uuid,
        application_id: Uuid,
    ) -> Result<Vec<ResolvedUserClaim>, StoreError> {
        let rows = sqlx::query(
            "SELECT
                 cd.id         AS claim_def_id,
                 cd.key        AS claim_key,
                 cd.claim_type AS claim_type,
                 uc.value      AS value
             FROM user_claims uc
             JOIN claim_definitions cd ON cd.id = uc.claim_def_id
             WHERE uc.user_id = $1 AND cd.application_id = $2
             ORDER BY cd.key, uc.value",
        )
        .bind(user_id)
        .bind(application_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_resolved).collect()
    }
}
