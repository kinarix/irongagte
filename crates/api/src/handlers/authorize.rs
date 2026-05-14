use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    Form,
};
use irongate_core::repositories::AuthCodeData;
use irongate_crypto::token::generate_token;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    state::AppState,
};

const CODE_TTL_SECS: i64 = 600; // 10 minutes

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    // Hidden fields echoed from the authorize params
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub tenant_id: Option<Uuid>,
}

pub async fn get_authorize(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuthorizeParams>,
) -> Result<Html<String>> {
    if params.response_type != "code" {
        return Err(Error::BadRequest(
            "only response_type=code is supported".into(),
        ));
    }
    if params.code_challenge_method.to_uppercase() != "S256" {
        return Err(Error::BadRequest(
            "only code_challenge_method=S256 is supported".into(),
        ));
    }

    // Validate the client + redirect_uri exist.
    let tenant_id = resolve_tenant(&state, params.tenant_id).await?;
    let app = state
        .applications
        .get_by_client_id(&params.client_id, tenant_id)
        .await
        .map_err(|_| Error::BadRequest("unknown client_id".into()))?;

    if !app.redirect_uris.contains(&params.redirect_uri) {
        return Err(Error::BadRequest("redirect_uri not registered".into()));
    }

    let scope = params.scope.unwrap_or_else(|| "openid".into());
    let state_val = params.state.unwrap_or_default();

    Ok(Html(login_form_html(
        &params.client_id,
        &params.redirect_uri,
        &scope,
        &state_val,
        &params.code_challenge,
        &params.code_challenge_method,
        tenant_id,
        None,
    )))
}

pub async fn post_authorize(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect> {
    let tenant_id = resolve_tenant(&state, form.tenant_id).await?;

    // Validate client + redirect_uri.
    let app = state
        .applications
        .get_by_client_id(&form.client_id, tenant_id)
        .await
        .map_err(|_| Error::BadRequest("unknown client_id".into()))?;

    if !app.redirect_uris.contains(&form.redirect_uri) {
        return Err(Error::BadRequest("redirect_uri not registered".into()));
    }

    // Authenticate the user.
    let user = state
        .password_svc
        .authenticate(&form.email, &form.password, tenant_id)
        .await
        .map_err(|_| {
            // Return the login form with an error rather than a plain 401.
            Error::Unauthorized("invalid credentials".into())
        })?;

    // Issue a short-lived auth code.
    let code = generate_token();
    let data = AuthCodeData {
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        scope: form.scope.clone(),
        user_id: user.id,
        tenant_id,
        code_challenge: form.code_challenge.clone(),
        code_challenge_method: form.code_challenge_method.clone(),
    };
    state
        .auth_codes
        .store_code(&code, data, CODE_TTL_SECS)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    // Redirect back to the client.
    let mut url = form.redirect_uri.clone();
    url.push_str(&format!("?code={code}"));
    if let Some(s) = &form.state {
        if !s.is_empty() {
            url.push_str(&format!("&state={s}"));
        }
    }

    Ok(Redirect::to(&url))
}

async fn resolve_tenant(state: &AppState, explicit: Option<Uuid>) -> Result<Uuid> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    // Fall back to the system tenant.
    state
        .tenants
        .get_by_slug("system")
        .await
        .map(|t| t.id)
        .map_err(|_| Error::BadRequest("cannot resolve tenant".into()))
}

#[allow(clippy::too_many_arguments)]
fn login_form_html(
    client_id: &str,
    redirect_uri: &str,
    scope: &str,
    state_val: &str,
    code_challenge: &str,
    code_challenge_method: &str,
    tenant_id: Uuid,
    error: Option<&str>,
) -> String {
    let error_html = match error {
        Some(msg) => {
            format!(r#"<p style="color:#ef4444;font-size:0.875rem;text-align:center">{msg}</p>"#)
        }
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>Sign in — Irongate</title>
  <style>
    *{{box-sizing:border-box;margin:0;padding:0}}
    body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#f8fafc;display:flex;align-items:center;justify-content:center;min-height:100vh}}
    .card{{background:#fff;border-radius:0.75rem;box-shadow:0 4px 24px rgba(0,0,0,.08);padding:2.5rem;width:100%;max-width:22rem}}
    h1{{font-size:1.25rem;font-weight:600;color:#0f172a;margin-bottom:1.5rem;text-align:center}}
    label{{display:block;font-size:0.875rem;font-weight:500;color:#475569;margin-bottom:0.375rem}}
    input[type=text],input[type=password]{{display:block;width:100%;padding:0.625rem 0.75rem;border:1px solid #cbd5e1;border-radius:0.5rem;font-size:0.9375rem;outline:none;transition:border-color .15s}}
    input:focus{{border-color:#6366f1}}
    .field{{margin-bottom:1rem}}
    button{{display:block;width:100%;padding:0.75rem;background:#6366f1;color:#fff;font-size:1rem;font-weight:600;border:none;border-radius:0.5rem;cursor:pointer;margin-top:1.5rem}}
    button:hover{{background:#4f46e5}}
  </style>
</head>
<body>
<div class="card">
  <h1>Sign in</h1>
  {error_html}
  <form method="POST" action="/oauth2/authorize">
    <input type="hidden" name="client_id" value="{client_id}"/>
    <input type="hidden" name="redirect_uri" value="{redirect_uri}"/>
    <input type="hidden" name="scope" value="{scope}"/>
    <input type="hidden" name="state" value="{state_val}"/>
    <input type="hidden" name="code_challenge" value="{code_challenge}"/>
    <input type="hidden" name="code_challenge_method" value="{code_challenge_method}"/>
    <input type="hidden" name="tenant_id" value="{tenant_id}"/>
    <div class="field">
      <label for="email">Username</label>
      <input type="text" id="email" name="email" autocomplete="username" required/>
    </div>
    <div class="field">
      <label for="password">Password</label>
      <input type="password" id="password" name="password" autocomplete="current-password" required/>
    </div>
    <button type="submit">Sign in</button>
  </form>
</div>
</body>
</html>"#,
        client_id = html_escape(client_id),
        redirect_uri = html_escape(redirect_uri),
        scope = html_escape(scope),
        state_val = html_escape(state_val),
        code_challenge = html_escape(code_challenge),
        code_challenge_method = html_escape(code_challenge_method),
        tenant_id = tenant_id,
        error_html = error_html,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
