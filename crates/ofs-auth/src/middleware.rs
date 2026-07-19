use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ofs_core::errors::OfsError;
use serde_json::json;
use std::sync::Arc;

use crate::authn::{AuthIdentity, AuthProvider, AuthRequest};
use crate::rbac::{Permission, RbacChecker};

/// Axum middleware state for auth.
pub struct AuthMiddlewareState {
    pub auth_provider: Box<dyn AuthProvider>,
    pub rbac_checker: RbacChecker,
    /// Path prefixes to exclude from auth (e.g., `/health`, `/metrics`).
    pub exclude_paths: Vec<String>,
}

impl AuthMiddlewareState {
    pub fn new(auth_provider: Box<dyn AuthProvider>, rbac_checker: RbacChecker) -> Self {
        Self {
            auth_provider,
            rbac_checker,
            exclude_paths: vec!["/health".to_string(), "/metrics".to_string()],
        }
    }
}

/// Axum middleware that performs authentication and authorization.
///
/// Extracts credentials from headers, authenticates via the configured provider,
/// and checks RBAC based on the request path.
pub async fn auth_middleware(
    state: Arc<AuthMiddlewareState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Skip auth for excluded paths (health checks, metrics, etc.)
    let path = request.uri().path();
    if state.exclude_paths.iter().any(|p| path.starts_with(p)) {
        return next.run(request).await;
    }

    let auth_request = extract_auth_request(&request);

    let identity = match state.auth_provider.authenticate(&auth_request).await {
        Ok(id) => id,
        Err(e) => {
            return match e {
                OfsError::Auth(_) => (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "unauthorized",
                        "message": e.to_string()
                    })),
                )
                    .into_response(),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "internal_error",
                        "message": e.to_string()
                    })),
                )
                    .into_response(),
            };
        }
    };

    // Check RBAC based on project extracted from path
    let project = extract_project_from_path(request.uri().path());
    let permission = extract_permission_from_method(request.method());

    if let Some(project) = project
        && let Some(perm) = permission
        && let Err(e) = state.rbac_checker.check_access(&identity, &project, perm)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "forbidden",
                "message": e.to_string()
            })),
        )
            .into_response();
    }

    // Attach identity to request extensions for downstream handlers
    request.extensions_mut().insert(identity);

    next.run(request).await
}

/// Extract the auth request from incoming HTTP headers.
fn extract_auth_request(request: &Request) -> AuthRequest {
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bearer_token = request
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    AuthRequest {
        api_key,
        bearer_token,
    }
}

/// Extract project name from URL path (e.g., `/v1/projects/my-project/features:get-online`).
fn extract_project_from_path(path: &str) -> Option<String> {
    // Patterns: /v1/features... (implicit "default" project)
    //           /v1/projects/{project}/features...
    if path.starts_with("/v1/projects/") {
        let rest = path.strip_prefix("/v1/projects/")?;
        rest.split('/').next().map(|s| s.to_string())
    } else if path.starts_with("/v1/") {
        Some("default".to_string())
    } else {
        None
    }
}

/// Extract required permission from HTTP method.
fn extract_permission_from_method(method: &axum::http::Method) -> Option<Permission> {
    match method.as_str() {
        "GET" | "HEAD" | "OPTIONS" => Some(Permission::Read),
        "POST" | "PUT" | "PATCH" | "DELETE" => Some(Permission::Write),
        _ => None,
    }
}

/// Helper to extract AuthIdentity from request extensions in handlers.
pub fn extract_identity(request: &Request) -> Option<&AuthIdentity> {
    request.extensions().get::<AuthIdentity>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_project_from_path_default() {
        let path = "/v1/features:get-online";
        assert_eq!(extract_project_from_path(path), Some("default".to_string()));
    }

    #[test]
    fn test_extract_project_from_path_explicit() {
        let path = "/v1/projects/my-project/features:get-online";
        assert_eq!(
            extract_project_from_path(path),
            Some("my-project".to_string())
        );
    }

    #[test]
    fn test_extract_project_from_path_no_match() {
        let path = "/health";
        assert_eq!(extract_project_from_path(path), None);
    }

    #[test]
    fn test_permission_from_get() {
        assert_eq!(
            extract_permission_from_method(&axum::http::Method::GET),
            Some(Permission::Read)
        );
    }

    #[test]
    fn test_permission_from_post() {
        assert_eq!(
            extract_permission_from_method(&axum::http::Method::POST),
            Some(Permission::Write)
        );
    }
}
