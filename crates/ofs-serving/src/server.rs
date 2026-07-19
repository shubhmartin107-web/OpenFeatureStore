use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::Json;
use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{Html, IntoResponse};
use ofs_auth::create_auth_provider;
use ofs_auth::{AuthProvider, AuthRequest, Permission, RbacChecker};
use ofs_config::OpenFeatureStoreConfig;
use ofs_core::traits::{OfflineStore, OnlineStore, Registry};
use ofs_lifecycle::DataLifecycleManager;
use ofs_observability::{HealthRegistry, OfsMetrics};
use serde_json::json;
use tokio::signal;
use tokio::sync::Notify;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::middleware::{RateLimiter, make_request_id};
use crate::rest::{features_router, health_router};

pub struct ServerState {
    pub start_time: Instant,
    pub health_registry: HealthRegistry,
    pub metrics: OfsMetrics,
    pub config: OpenFeatureStoreConfig,
    pub auth_provider: Option<Box<dyn AuthProvider>>,
    pub rbac_checker: Option<RbacChecker>,
    pub registry: OnceLock<Arc<dyn Registry>>,
    pub online_store: OnceLock<Arc<dyn OnlineStore>>,
    pub offline_store: OnceLock<Arc<dyn OfflineStore>>,
}

impl ServerState {
    pub fn new(config: OpenFeatureStoreConfig) -> Self {
        Self {
            start_time: Instant::now(),
            health_registry: HealthRegistry::new(),
            metrics: OfsMetrics::new(),
            config,
            auth_provider: None,
            rbac_checker: None,
            registry: OnceLock::new(),
            online_store: OnceLock::new(),
            offline_store: OnceLock::new(),
        }
    }
}

use utoipa::OpenApi;

async fn swagger_ui_handler() -> Html<&'static str> {
    Html(
        r##"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <title>OpenFeatureStore API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>SwaggerUIBundle({ url: "/api-docs/openapi.json", dom_id: "#swagger-ui" })</script>
</body>
</html>"##,
    )
}

async fn openapi_json_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::api_doc::ApiDoc::openapi())
}

pub struct FeatureServer {
    pub state: Arc<ServerState>,
    pub rest_router: Router,
    pub shutdown: Arc<Notify>,
}

impl FeatureServer {
    pub fn new(config: OpenFeatureStoreConfig) -> Self {
        let auth_provider = config.auth.as_ref().map(|ac| create_auth_provider(ac));
        let rbac_checker = config.auth.as_ref().map(|_| RbacChecker::new());
        let state = Arc::new(ServerState {
            auth_provider,
            rbac_checker,
            ..ServerState::new(config)
        });

        let state_for_auth = state.clone();
        let rest_router =
            Router::new()
                .merge(health_router())
                .merge(features_router())
                .route("/docs", axum::routing::get(swagger_ui_handler))
                .route(
                    "/api-docs/openapi.json",
                    axum::routing::get(openapi_json_handler),
                )
                .layer(axum::middleware::from_fn(
                    move |mut request: Request, next: Next| {
                        let state = state_for_auth.clone();
                        async move {
                            let path = request.uri().path();
                            if path.starts_with("/health") || path.starts_with("/metrics") {
                                return next.run(request).await;
                            }

                            let Some(ref provider) = state.auth_provider else {
                                return next.run(request).await;
                            };
                            let Some(ref rbac) = state.rbac_checker else {
                                return next.run(request).await;
                            };

                            let auth_request = AuthRequest {
                                api_key: request
                                    .headers()
                                    .get("x-api-key")
                                    .and_then(|v| v.to_str().ok())
                                    .map(|s| s.to_string()),
                                bearer_token: request
                                    .headers()
                                    .get(http::header::AUTHORIZATION)
                                    .and_then(|v| v.to_str().ok())
                                    .and_then(|v| v.strip_prefix("Bearer "))
                                    .map(|s| s.to_string()),
                            };

                            let identity =
                                match provider.authenticate(&auth_request).await {
                                    Ok(id) => id,
                                    Err(e) => {
                                        return match e {
                                ofs_core::errors::OfsError::Auth(_) => {
                                    (StatusCode::UNAUTHORIZED, Json(json!({
                                        "error": "unauthorized", "message": e.to_string()
                                    }))).into_response()
                                }
                                _ => {
                                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                                        "error": "internal_error", "message": e.to_string()
                                    }))).into_response()
                                }
                            };
                                    }
                                };

                            let project = request.uri().path();
                            let project = if project.starts_with("/v1/projects/") {
                                project
                                    .strip_prefix("/v1/projects/")
                                    .and_then(|r| r.split('/').next())
                            } else if project.starts_with("/v1/") {
                                Some("default")
                            } else {
                                None
                            };
                            let permission = match request.method().as_str() {
                                "GET" | "HEAD" | "OPTIONS" => Some(Permission::Read),
                                "POST" | "PUT" | "PATCH" | "DELETE" => Some(Permission::Write),
                                _ => None,
                            };

                            if let Some(project) = project
                                && let Some(perm) = permission
                                && rbac.check_access(&identity, project, perm).is_err()
                            {
                                return (
                                    StatusCode::FORBIDDEN,
                                    Json(json!({
                                        "error": "forbidden", "message": "access denied"
                                    })),
                                )
                                    .into_response();
                            }

                            request.extensions_mut().insert(identity);
                            next.run(request).await
                        }
                    },
                ))
                .layer(TraceLayer::new_for_http().make_span_with(
                    |request: &axum::http::Request<_>| {
                        let request_id = request
                            .headers()
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_else(|| Box::leak(make_request_id().into_boxed_str()));
                        tracing::info_span!(
                            "request",
                            request_id = %request_id,
                            method = %request.method(),
                            path = %request.uri().path(),
                        )
                    },
                ))
                .layer(CompressionLayer::new().gzip(true))
                .layer(CorsLayer::permissive())
                .with_state(state.clone());

        Self {
            state,
            rest_router,
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// Attach stores to the server state.
    /// Must be called before `start()` and before any clone of state escapes.
    pub fn with_stores(
        &mut self,
        registry: Arc<dyn Registry>,
        online_store: Arc<dyn OnlineStore>,
        offline_store: Arc<dyn OfflineStore>,
    ) {
        self.state
            .registry
            .set(registry)
            .ok()
            .expect("registry already set");
        self.state
            .online_store
            .set(online_store)
            .ok()
            .expect("online_store already set");
        self.state
            .offline_store
            .set(offline_store)
            .ok()
            .expect("offline_store already set");
    }

    /// Start the data lifecycle manager as a background task.
    pub fn start_lifecycle(&self) {
        let Some(registry) = self.state.registry.get() else {
            return;
        };
        let Some(online_store) = self.state.online_store.get() else {
            return;
        };
        let Some(offline_store) = self.state.offline_store.get() else {
            return;
        };

        let lc = self.state.config.lifecycle.clone().unwrap_or_default();
        let manager = DataLifecycleManager::new(
            registry.clone(),
            online_store.clone(),
            offline_store.clone(),
            lc.ttl_default_days,
            lc.cleanup_interval_secs,
            lc.projects,
        );
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            manager.run(shutdown).await;
        });
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server_cfg = self
            .state
            .config
            .server
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let rest_addr: std::net::SocketAddr =
            format!("{}:{}", server_cfg.host, server_cfg.port).parse()?;
        let grpc_addr: std::net::SocketAddr =
            format!("{}:{}", server_cfg.host, server_cfg.grpc_port).parse()?;
        let enable_rate_limit = server_cfg
            .rate_limit
            .as_ref()
            .map(|r| r.enabled)
            .unwrap_or(false);
        let default_rps = server_cfg
            .rate_limit
            .as_ref()
            .map(|r| r.default_rps)
            .unwrap_or(100);

        let rate_limiter = Arc::new(RateLimiter::new(default_rps));
        let mut rest_router = self.rest_router;
        if enable_rate_limit {
            let limiter = rate_limiter.clone();
            rest_router = rest_router.layer(axum::middleware::from_fn(move |req, next| {
                let l = limiter.clone();
                async move { l.check(req, next).await }
            }));
        }

        tracing::info!("REST API starting on {rest_addr}");

        if let Some(ref tls) = server_cfg.tls {
            if let Some(ref path) = tls.cert_path {
                if std::path::Path::new(path).exists() {
                    tracing::info!("TLS cert found at {}", path);
                } else {
                    tracing::warn!("TLS cert not found at {}", path);
                }
            }
            if let Some(ref path) = tls.key_path {
                if std::path::Path::new(path).exists() {
                    tracing::info!("TLS key found at {}", path);
                } else {
                    tracing::warn!("TLS key not found at {}", path);
                }
            }
        }
        // TLS termination is delegated to the reverse proxy (nginx).
        // The server always serves plain HTTP.
        let rest_handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(rest_addr)
                .await
                .expect("REST bind failed");
            axum::serve(listener, rest_router)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .expect("REST server failed");
        });

        tracing::info!("gRPC API starting on {grpc_addr}");
        let grpc_shutdown = self.shutdown.clone();
        let grpc_state = self.state.clone();
        let grpc_handle = tokio::spawn(async move {
            use ofs_proto::serving_proto::serving_service_server::ServingServiceServer;

            let state_for_auth = grpc_state.clone();
            // gRPC auth via sync interceptor. Auth is async (JWT, JWKS fetch), but
            // the interceptor runs on tonic's multi-threaded runtime where Handle::block_on
            // is safe — it yields the current thread until the spawned auth future completes.
            // For API-key auth (in-memory) this is instant; for JWT the JWKS is cached.
            #[allow(clippy::result_large_err)]
            let grpc_auth = move |req: tonic::Request<()>| {
                let state = state_for_auth.clone();
                let Some(ref provider) = state.auth_provider else {
                    return Ok(req);
                };
                let meta = req.metadata();
                let api_key = meta
                    .get("x-api-key")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let bearer_token = meta
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(|s| s.to_string());
                let auth_request = ofs_auth::AuthRequest {
                    api_key,
                    bearer_token,
                };
                let rt = tokio::runtime::Handle::current();
                match rt.block_on(provider.authenticate(&auth_request)) {
                    Ok(id) => {
                        let mut req = req;
                        req.extensions_mut().insert(id);
                        Ok(req)
                    }
                    Err(e) => Err(tonic::Status::unauthenticated(e.to_string())),
                }
            };

            let serving_service = crate::grpc::ServingService::new(grpc_state);
            let svc = ServingServiceServer::with_interceptor(serving_service, grpc_auth);
            tonic::transport::Server::builder()
                .add_service(svc)
                .serve_with_shutdown(grpc_addr, async move {
                    grpc_shutdown.notified().await;
                })
                .await
                .expect("gRPC server failed");
        });

        tokio::select! {
            _ = rest_handle => {},
            _ = grpc_handle => {},
        }

        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("received SIGTERM, starting graceful shutdown");
        },
    }
}
