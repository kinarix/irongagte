//! Audit-event recording helper. Every admin write path should call
//! [`record`] after a successful mutation. Failures to write the audit row
//! are logged but never fail the request — audit must not be in the critical
//! path of a user-visible operation.
//!
//! Event-type convention: `<resource>.<action>`, e.g. `users.create`,
//! `claim_definition.delete`, `group.member_added`.

use irongate_core::types::AuditEvent;
use serde_json::Value;
use time::OffsetDateTime;
use tracing::warn;
use uuid::Uuid;

use crate::{claims::OperatorClaims, state::AppState};

/// Record an audit event. `tenant_id` is `None` for system-scoped events
/// (operator, operator-role, operator-permission CRUD). `target_id` is the
/// row the operator acted on, when applicable.
///
/// `metadata` should be a small JSON object identifying the action — never
/// secrets, hashes, or PII like email. Identifiers and enum-like fields are
/// fine.
pub async fn record(
    state: &AppState,
    claims: &OperatorClaims,
    tenant_id: Option<Uuid>,
    event_type: &str,
    target_id: Option<Uuid>,
    metadata: Value,
) {
    let actor_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => Some(id),
        Err(_) => {
            warn!(sub = %claims.sub, "audit: operator sub is not a UUID");
            None
        }
    };

    let event = AuditEvent {
        id: Uuid::new_v4(),
        tenant_id,
        event_type: event_type.into(),
        actor_id,
        target_id,
        ip_address: None,
        metadata,
        created_at: OffsetDateTime::now_utc(),
    };

    metrics::counter!(
        "irongate_audit_events_total",
        "event_type" => event_type.to_string(),
    )
    .increment(1);

    if let Err(e) = state.audit.record(event).await {
        warn!(error = %e, event_type, "audit: failed to record event");
    }
}
