use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use irongate_core::{
    errors::AuthzError,
    repositories::{ClaimDefinitionRepository, GroupClaimRepository, UserClaimRepository},
    types::ClaimType,
};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct AuthzService {
    claim_defs: Arc<dyn ClaimDefinitionRepository>,
    group_claims: Arc<dyn GroupClaimRepository>,
    user_claims: Arc<dyn UserClaimRepository>,
}

impl AuthzService {
    pub fn new(
        claim_defs: Arc<dyn ClaimDefinitionRepository>,
        group_claims: Arc<dyn GroupClaimRepository>,
        user_claims: Arc<dyn UserClaimRepository>,
    ) -> Self {
        Self {
            claim_defs,
            group_claims,
            user_claims,
        }
    }

    /// Resolve every custom claim that should appear in a token minted for
    /// `user` against `application`. Returns a map keyed by
    /// `<claim_prefix>:<claim.key>`.
    ///
    /// Rules (mirrors the model in `migrations/postgres/0024_..0027_*`):
    ///   * `scalar` — user-direct value wins. Otherwise the group with the
    ///     highest `priority` wins; ties resolved by `created_at` ascending.
    ///   * `multi`  — values from all sources (groups + user-direct) are
    ///     merged into a deduped JSON array of strings.
    ///   * Claims with no assignment at all are omitted entirely.
    pub async fn resolve_claims_for_app(
        &self,
        user_id: Uuid,
        application_id: Uuid,
        claim_prefix: &str,
    ) -> Result<HashMap<String, Value>, AuthzError> {
        let defs = self
            .claim_defs
            .list_for_app(application_id)
            .await
            .map_err(AuthzError::Store)?;
        if defs.is_empty() {
            return Ok(HashMap::new());
        }

        let group_rows = self
            .group_claims
            .list_for_user_in_app(user_id, application_id)
            .await
            .map_err(AuthzError::Store)?;
        let user_rows = self
            .user_claims
            .list_for_user_in_app(user_id, application_id)
            .await
            .map_err(AuthzError::Store)?;

        let mut out: HashMap<String, Value> = HashMap::with_capacity(defs.len());

        for def in defs {
            let token_key = format!("{claim_prefix}:{}", def.key);
            match def.claim_type {
                ClaimType::Scalar => {
                    // User-direct wins.
                    let user_value = user_rows
                        .iter()
                        .find(|r| r.claim_def_id == def.id)
                        .map(|r| r.value.clone());

                    if let Some(v) = user_value {
                        out.insert(token_key, Value::String(v));
                        continue;
                    }

                    // Group rows are already returned in
                    // (priority DESC, created_at ASC) order by the repo.
                    if let Some(winner) = group_rows.iter().find(|r| r.claim_def_id == def.id) {
                        out.insert(token_key, Value::String(winner.value.clone()));
                    }
                }
                ClaimType::Multi => {
                    let mut bag: BTreeSet<String> = BTreeSet::new();
                    for r in group_rows.iter().filter(|r| r.claim_def_id == def.id) {
                        bag.insert(r.value.clone());
                    }
                    for r in user_rows.iter().filter(|r| r.claim_def_id == def.id) {
                        bag.insert(r.value.clone());
                    }
                    if bag.is_empty() {
                        continue;
                    }
                    let arr: Vec<String> = bag.into_iter().collect();
                    out.insert(token_key, json!(arr));
                }
            }
        }

        Ok(out)
    }
}
