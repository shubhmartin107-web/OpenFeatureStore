use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ofs_core::traits::{OfflineStore, OnlineStore, Registry};
use ofs_core::types::{EntityKey, OnlineWriteRecord};
use serde_json::Value;
use sqlx::SqlitePool;
use tower::ServiceExt;

/// Create stores and server, write a feature value, read it back via REST.
#[tokio::test]
async fn test_rest_write_then_read() {
    let dir = tempfile::tempdir().unwrap();
    let reg_path = dir.path().join("registry.db");
    let onl_path = dir.path().join("online.db");

    let config_yaml = format!(
        r#"
version: "1"
server:
  host: "127.0.0.1"
  port: 0
  grpc_port: 0
registry:
  backend: "sqlite"
  path: "{}"
online_store:
  backend: "sqlite"
  path: "{}"
offline_store:
  backend: "duckdb"
lifecycle:
  ttl_default_days: 90
  cleanup_interval_secs: 3600
"#,
        reg_path.display(),
        onl_path.display(),
    );
    let config: ofs_config::OpenFeatureStoreConfig = serde_yaml::from_str(&config_yaml).unwrap();

    let pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", reg_path.display()))
        .await
        .unwrap();
    let registry: Arc<dyn Registry> = Arc::new(ofs_registry::SqlRegistry::new(pool).await.unwrap());

    let online_pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", onl_path.display()))
        .await
        .unwrap();
    let online_store: Arc<dyn OnlineStore> =
        Arc::new(ofs_online_store::SqliteOnlineStore::new(online_pool));

    let offline_store: Arc<dyn OfflineStore> = Arc::new(ofs_offline_store::DuckDbOfflineStore);

    // Register entity
    let mut entity = ofs_core::types::Entity::default();
    entity.name = "driver".into();
    entity.value_type = ofs_core::ValueType::String;
    entity.join_keys = vec!["driver_id".into()];
    registry.apply_entity(&entity, "default").await.unwrap();

    // Register feature view
    let mut fv = ofs_core::types::FeatureView::new("driver_stats");
    fv.entities = vec!["driver".into()];
    fv.features = vec![ofs_core::types::Feature::new(
        "avg_daily_trips",
        ofs_core::ValueType::Int64,
    )];
    fv.ttl = Some(Duration::from_secs(86400));
    registry.apply_feature_view(&fv, "default").await.unwrap();

    // Write feature value
    let ek = EntityKey::new(vec!["driver_id".into()]);
    let mut values = HashMap::new();
    values.insert("avg_daily_trips".to_string(), b"42".to_vec());
    let record = OnlineWriteRecord {
        entity_key: ek,
        values,
        timestamp: chrono::Utc::now(),
        feature_view_name: "driver_stats".to_string(),
    };
    online_store
        .online_write_batch(vec![record], "default")
        .await
        .unwrap();

    // Build server
    let mut server = ofs_serving::FeatureServer::new(config);
    server.with_stores(registry, online_store, offline_store);
    server.start_lifecycle();
    let router = server.rest_router;

    // Request via REST
    let req_body = serde_json::json!({
        "features": ["driver_stats__avg_daily_trips"],
        "entities": { "driver_id": ["1001"] },
        "project": "default"
    });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("http://127.0.0.1:0/v1/features:get-online")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body).to_string();
    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200, got {status}: {body_str}"
    );
    assert!(
        body_str.contains("42"),
        "expected value 42 in response, got: {body_str}"
    );
}

/// Health check works even without stores configured.
#[tokio::test]
async fn test_health_endpoint() {
    let config = ofs_config::OpenFeatureStoreConfig::default();
    let server = ofs_serving::FeatureServer::new(config);
    let router = server.rest_router;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("http://127.0.0.1:0/v1/health")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
}

/// Querying a nonexistent entity still returns 200 with NOT_FOUND statuses.
#[tokio::test]
async fn test_get_features_missing_entity() {
    let dir = tempfile::tempdir().unwrap();
    let reg_path = dir.path().join("registry.db");

    let config_yaml = format!(
        r#"
version: "1"
server:
  host: "127.0.0.1"
  port: 0
  grpc_port: 0
registry:
  backend: "sqlite"
  path: "{}"
online_store:
  backend: "sqlite"
  path: "{}"
offline_store:
  backend: "duckdb"
"#,
        reg_path.display(),
        dir.path().join("online.db").display(),
    );
    let config: ofs_config::OpenFeatureStoreConfig = serde_yaml::from_str(&config_yaml).unwrap();

    let pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", reg_path.display()))
        .await
        .unwrap();
    let registry: Arc<dyn Registry> = Arc::new(ofs_registry::SqlRegistry::new(pool).await.unwrap());

    let mut entity = ofs_core::types::Entity::default();
    entity.name = "driver".into();
    entity.value_type = ofs_core::ValueType::String;
    entity.join_keys = vec!["driver_id".into()];
    registry.apply_entity(&entity, "default").await.unwrap();

    let mut fv = ofs_core::types::FeatureView::new("driver_stats");
    fv.entities = vec!["driver".into()];
    fv.features = vec![ofs_core::types::Feature::new(
        "avg_daily_trips",
        ofs_core::ValueType::Int64,
    )];
    fv.ttl = Some(Duration::from_secs(86400));
    registry.apply_feature_view(&fv, "default").await.unwrap();

    let online_pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    let online_store: Arc<dyn OnlineStore> =
        Arc::new(ofs_online_store::SqliteOnlineStore::new(online_pool));
    let offline_store: Arc<dyn OfflineStore> = Arc::new(ofs_offline_store::DuckDbOfflineStore);

    let mut server = ofs_serving::FeatureServer::new(config);
    server.with_stores(registry, online_store, offline_store);
    let router = server.rest_router;

    let req_body = serde_json::json!({
        "features": ["driver_stats__avg_daily_trips"],
        "entities": { "driver_id": ["missing"] },
        "project": "default"
    });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("http://127.0.0.1:0/v1/features:get-online")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body).to_string();
    assert!(
        status.is_success(),
        "expected success, got {status}: {body_str}"
    );
    let json: Value = serde_json::from_slice(&body).unwrap();
    let features = json["features"].as_array().unwrap();
    for f in features {
        let s = f["status"].as_str().unwrap_or("");
        let ok = s == "NotFound";
        assert!(ok, "expected NotFound status, got: {s}");
    }
}
