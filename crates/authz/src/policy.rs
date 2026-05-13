use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ne,
    In,
    NotIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Check a named attribute in the evaluation context.
    UserAttribute {
        attribute: String,
        operator: Operator,
        value: serde_json::Value,
    },
    /// Allow only within a window of hours (UTC, [start_hour, end_hour)).
    TimeRange {
        start_hour: u8,
        end_hour: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacPolicy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    /// The resource this policy applies to ("*" matches any).
    pub resource: String,
    /// The action this policy applies to ("*" matches any).
    pub action: String,
    pub effect: PolicyEffect,
    /// All conditions must hold for the effect to apply.
    pub conditions: Vec<Condition>,
}

/// Runtime context passed to the policy evaluator.
pub struct EvaluationContext {
    pub user_attributes: HashMap<String, serde_json::Value>,
    pub request_time: OffsetDateTime,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self {
            user_attributes: HashMap::new(),
            request_time: OffsetDateTime::now_utc(),
        }
    }
}

/// Returns `true` if *all* conditions in `policy` hold in `ctx`.
///
/// When `effect` is `Allow`, returns `true` means the policy grants access.
/// When `effect` is `Deny`,  returns `true` means the policy blocks access.
/// Callers decide how to combine multiple policies (deny-overrides is typical).
pub fn evaluate(policy: &AbacPolicy, ctx: &EvaluationContext) -> bool {
    policy.conditions.iter().all(|cond| check_condition(cond, ctx))
}

fn check_condition(cond: &Condition, ctx: &EvaluationContext) -> bool {
    match cond {
        Condition::UserAttribute { attribute, operator, value } => {
            let actual = ctx.user_attributes.get(attribute.as_str());
            match operator {
                Operator::Eq => actual == Some(value),
                Operator::Ne => actual != Some(value),
                Operator::In => {
                    if let (Some(actual), Some(arr)) = (actual, value.as_array()) {
                        arr.contains(actual)
                    } else {
                        false
                    }
                }
                Operator::NotIn => {
                    if let Some(arr) = value.as_array() {
                        !actual.is_some_and(|v| arr.contains(v))
                    } else {
                        true
                    }
                }
            }
        }
        Condition::TimeRange { start_hour, end_hour } => {
            let hour = ctx.request_time.hour();
            hour >= *start_hour && hour < *end_hour
        }
    }
}

/// Returns `true` if the effect of `policy` applies and is `Allow`.
/// Returns `false` for `Deny` effects — callers check deny separately if needed.
pub fn policy_allows(policy: &AbacPolicy, ctx: &EvaluationContext) -> bool {
    policy.effect == PolicyEffect::Allow && evaluate(policy, ctx)
}

/// Returns `true` if the effect of `policy` applies and is `Deny`.
pub fn policy_denies(policy: &AbacPolicy, ctx: &EvaluationContext) -> bool {
    policy.effect == PolicyEffect::Deny && evaluate(policy, ctx)
}
