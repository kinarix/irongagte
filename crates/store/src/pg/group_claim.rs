use async_trait::async_trait;
use irongate_core::{
    errors::StoreError,
    repositories::ResolvedGroupClaim,
    types::{ClaimType, GroupClaim},
};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct PgGroupClaimRepo {
    pub(crate) pool: PgPool,
}

fn row_to_group_claim(row: &sqlx::postgres::PgRow) -> Result<GroupClaim, StoreError> {
    Ok(GroupClaim {
        group_id: row.try_get("group_id").map_err(map_row_err)?,
        claim_def_id: row.try_get("claim_def_id").map_err(map_row_err)?,
        value: row.try_get("value").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

fn row_to_resolved(row: &sqlx::postgres::PgRow) -> Result<ResolvedGroupClaim, StoreError> {
    let claim_type_str: String = row.try_get("claim_type").map_err(map_row_err)?;
    let claim_type = ClaimType::from_str(&claim_type_str)
        .map_err(|_| map_parse_err("claim_type"))?;
    Ok(ResolvedGroupClaim {
        claim_def_id: row.try_get("claim_def_id").map_err(map_row_err)?,
        claim_key: row.try_get("claim_key").map_err(map_row_err)?,
        claim_type,
        group_id: row.try_get("group_id").map_err(map_row_err)?,
        group_priority: row.try_get("group_priority").map_err(map_row_err)?,
        group_created_at: row.try_get("group_created_at").map_err(map_row_err)?,
        value: row.try_get("value").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::GroupClaimRepository for PgGroupClaimRepo {
    async fn assign(
        &self,
        group_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<GroupClaim, StoreError> {
        let row = sqlx::query(
            "INSERT INTO group_claims (group_id, claim_def_id, value)
             VALUES ($1,$2,$3)
             ON CONFLICT (group_id, claim_def_id, value) DO UPDATE
                 SET value = EXCLUDED.value
             RETURNING *",
        )
        .bind(group_id)
        .bind(claim_def_id)
        .bind(value)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_group_claim(&row)
    }

    async fn revoke(
        &self,
        group_id: Uuid,
        claim_def_id: Uuid,
        value: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM group_claims
             WHERE group_id = $1 AND claim_def_id = $2 AND value = $3",
        )
        .bind(group_id)
        .bind(claim_def_id)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list_for_group(&self, group_id: Uuid) -> Result<Vec<GroupClaim>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM group_claims WHERE group_id = $1 ORDER BY claim_def_id, value",
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_group_claim).collect()
    }

    async fn list_for_claim_def(
        &self,
        claim_def_id: Uuid,
    ) -> Result<Vec<GroupClaim>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM group_claims WHERE claim_def_id = $1 ORDER BY group_id, value",
        )
        .bind(claim_def_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_group_claim).collect()
    }

    async fn list_for_user_in_app(
        &self,
        user_id: Uuid,
        application_id: Uuid,
    ) -> Result<Vec<ResolvedGroupClaim>, StoreError> {
        let rows = sqlx::query(
            "SELECT
                 cd.id          AS claim_def_id,
                 cd.key         AS claim_key,
                 cd.claim_type  AS claim_type,
                 g.id           AS group_id,
                 g.priority     AS group_priority,
                 g.created_at   AS group_created_at,
                 gc.value       AS value
             FROM group_members gm
             JOIN groups g            ON g.id = gm.group_id
             JOIN group_claims gc     ON gc.group_id = g.id
             JOIN claim_definitions cd ON cd.id = gc.claim_def_id
             WHERE gm.user_id = $1 AND cd.application_id = $2
             ORDER BY cd.key, g.priority DESC, g.created_at ASC, gc.value",
        )
        .bind(user_id)
        .bind(application_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_resolved).collect()
    }
}
