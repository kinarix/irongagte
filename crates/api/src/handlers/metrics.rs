use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
};

use crate::state::AppState;

/// `GET /metrics` — renders the Prometheus exposition format. Scraped by
/// Prometheus / Grafana Agent / OpenTelemetry collector. Cheap: just dumps the
/// current snapshot of every metric the recorder has seen.
pub async fn render(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let body = state.metrics.render();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}
