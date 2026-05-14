/// Parses OAuth scopes of the form `"resource:action"` into `(resource, action)` pairs.
///
/// Scopes that don't contain `:` are treated as opaque and skipped.
/// The action `"*"` is a wildcard matching any action on the resource.
pub fn resolve_scopes(scopes: &[String]) -> Vec<(String, String)> {
    scopes
        .iter()
        .filter_map(|s| {
            let (resource, action) = s.split_once(':')?;
            if resource.is_empty() || action.is_empty() {
                return None;
            }
            Some((resource.to_owned(), action.to_owned()))
        })
        .collect()
}

/// Returns `true` if any scope in `scopes` grants `resource:action`.
///
/// A scope `"resource:*"` grants all actions on that resource.
/// A scope `"*:*"` grants all actions on all resources.
pub fn scopes_grant(scopes: &[String], resource: &str, action: &str) -> bool {
    resolve_scopes(scopes)
        .into_iter()
        .any(|(r, a)| (r == resource || r == "*") && (a == action || a == "*"))
}
