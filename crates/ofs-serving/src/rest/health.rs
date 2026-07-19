use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use serde_json::json;

use crate::ServerState;

#[utoipa::path(
    get,
    path = "/v1/health",
    responses((status = 200, description = "Service is healthy")),
    tag = "health"
)]
pub async fn health_handler(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let report = state.health_registry.report(state.start_time);
    Json(json!({
        "status": "healthy",
        "version": report.version,
        "uptime_seconds": report.uptime_seconds,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/ready",
    responses(
        (status = 200, description = "Service is ready"),
        (status = 503, description = "Service not ready"),
    ),
    tag = "health"
)]
pub async fn ready_handler(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let report = state.health_registry.report(state.start_time);
    let status_code = if report.status.is_ready() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(json!(report)))
}

#[utoipa::path(
    get,
    path = "/v1/info",
    responses((status = 200, description = "Server info")),
    tag = "health"
)]
pub async fn info_handler() -> Json<serde_json::Value> {
    Json(json!({
        "name": "openfeaturestore",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Offline+Online Feature Store with point-in-time correctness",
        "grpc_api": "feast.serving.ServingService",
        "rest_api_version": "v1",
    }))
}

#[utoipa::path(
    get,
    path = "/v1/metrics",
    responses((status = 200, description = "Prometheus metrics")),
    tag = "health"
)]
pub async fn metrics_handler(
    State(state): State<Arc<ServerState>>,
) -> (axum::http::StatusCode, String) {
    let output = state.metrics.prometheus_output();
    (axum::http::StatusCode::OK, output)
}

pub fn health_router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/health", get(health_handler))
        .route("/v1/ready", get(ready_handler))
        .route("/v1/info", get(info_handler))
        .route("/v1/metrics", get(metrics_handler))
}
