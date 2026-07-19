use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::post;
use chrono::Utc;
use ofs_core::entity_key::deserialize_entity_key_v3;
use ofs_core::types::{
    EntityKey, FeatureViewProjection, FeatureViewWithProjection, OnlineWriteRecord,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ServerState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnlineFeaturesRequestJson {
    pub feature_service: Option<String>,
    pub features: Vec<String>,
    pub entities: std::collections::HashMap<String, Vec<serde_json::Value>>,
    pub project: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeatureValue {
    pub entity_key: String,
    pub feature_name: String,
    pub value: Option<serde_json::Value>,
    pub status: String,
    pub event_timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnlineFeaturesResponseJson {
    pub features: Vec<FeatureValue>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WriteOnlineFeaturesRequestJson {
    pub project: String,
    pub feature_view: String,
    pub entity_key: String,
    pub features: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PushFeaturesRequestJson {
    pub project: Option<String>,
    pub feature_view: String,
    pub entity_key: Vec<String>,
    pub features: std::collections::HashMap<String, serde_json::Value>,
    pub event_timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PushFeaturesResponseJson {
    pub ingested: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/v1/features:get-online",
    request_body = GetOnlineFeaturesRequestJson,
    responses(
        (status = 200, description = "Online features retrieved", body = GetOnlineFeaturesResponseJson),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal error"),
    ),
    tag = "features"
)]
pub async fn get_online_features_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<GetOnlineFeaturesRequestJson>,
) -> Result<Json<GetOnlineFeaturesResponseJson>, (axum::http::StatusCode, Json<serde_json::Value>)>
{
    state.metrics.record_feature_request("rest", "requested");
    let project = req.project.as_deref().unwrap_or("default");

    let online_store = state
        .online_store
        .get()
        .ok_or_else(|| internal_err("online store not configured"))?;

    // Build entity keys from the request entities map
    // The entities map is: join_key_name -> list_of_values (one per row)
    let entity_count = req.entities.values().next().map(|v| v.len()).unwrap_or(0);
    let mut entity_keys = Vec::with_capacity(entity_count);
    for i in 0..entity_count {
        let mut ek = EntityKey::new(Vec::new());
        for (key_name, values) in &req.entities {
            if i < values.len() {
                ek.join_keys.push(key_name.clone());
                let val_str = serde_json::to_string(&values[i]).unwrap_or_default();
                ek.entity_values.push(val_str.into_bytes());
            }
        }
        entity_keys.push(ek);
    }

    if entity_keys.is_empty() {
        return Err(bad_request("no entity keys provided"));
    }

    // Build feature view projections from the registry
    let registry = state
        .registry
        .get()
        .ok_or_else(|| internal_err("registry not configured"))?;

    // If feature_service is specified, look it up
    let mut feature_fvps = Vec::new();
    if let Some(ref fs_name) = req.feature_service {
        let fs = registry
            .get_feature_service(fs_name, project)
            .await
            .map_err(|e| internal_err(&e.to_string()))?
            .ok_or_else(|| bad_request(&format!("feature service '{}' not found", fs_name)))?;

        for fvp_in in &fs.features {
            let fv = registry
                .get_feature_view(&fvp_in.feature_view_name, project)
                .await
                .map_err(|e| internal_err(&e.to_string()))?
                .ok_or_else(|| {
                    bad_request(&format!(
                        "feature view '{}' not found",
                        fvp_in.feature_view_name
                    ))
                })?;

            feature_fvps.push(FeatureViewWithProjection {
                feature_view: fv,
                projection: fvp_in.clone(),
            });
        }
    } else if !req.features.is_empty() {
        // Group features by feature view name (feature names are "fv__feature" or "feature")
        // We need to find which feature view owns each feature
        let all_fvs = registry
            .list_feature_views(project)
            .await
            .map_err(|e| internal_err(&e.to_string()))?;

        for fv in &all_fvs {
            let mut matched_features = Vec::new();
            for feature_name in &req.features {
                // Check for "fv_name__feature_name" format
                let prefix = format!("{}__", fv.name);
                let local_name = if let Some(stripped) = feature_name.strip_prefix(&prefix) {
                    stripped
                } else {
                    feature_name.as_str()
                };

                if let Some(f) = fv.features.iter().find(|f| f.name == local_name) {
                    matched_features.push(f.clone());
                }
            }

            if !matched_features.is_empty() {
                feature_fvps.push(FeatureViewWithProjection {
                    feature_view: fv.clone(),
                    projection: FeatureViewProjection {
                        feature_view_name: fv.name.clone(),
                        feature_view_name_alias: None,
                        feature_columns: matched_features,
                        join_key_map: std::collections::HashMap::new(),
                        timestamp_field: None,
                        date_partition_column: None,
                        created_timestamp_column: None,
                        batch_source: None,
                        stream_source: None,
                        view_type: "FeatureView".to_string(),
                    },
                });
            }
        }
    }

    if feature_fvps.is_empty() {
        return Err(bad_request(
            "no feature views matched for the requested features",
        ));
    }

    let response = online_store
        .online_read(entity_keys, &feature_fvps, project)
        .await
        .map_err(|e| internal_err(&e.to_string()))?;

    state
        .metrics
        .record_store_read("online", &response.metadata.feature_names.len().to_string());

    let mut features = Vec::new();
    for result in &response.results {
        for (i, fname) in response.metadata.feature_names.iter().enumerate() {
            let value = if i < result.values.len() && !result.values[i].is_empty() {
                serde_json::from_slice(&result.values[i]).ok()
            } else {
                None
            };
            let status = match result.statuses.get(i) {
                Some(s) => format!("{:?}", s),
                None => "INVALID".to_string(),
            };
            let ts = result
                .event_timestamps
                .get(i)
                .and_then(|t| t.as_ref().map(|t| t.to_rfc3339()));
            features.push(FeatureValue {
                entity_key: String::new(),
                feature_name: fname.clone(),
                value,
                status,
                event_timestamp: ts,
            });
        }
    }

    Ok(Json(GetOnlineFeaturesResponseJson {
        features,
        metadata: serde_json::json!({
            "feature_names": response.metadata.feature_names,
        }),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/features:write-online",
    request_body = WriteOnlineFeaturesRequestJson,
    responses(
        (status = 200, description = "Feature written"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal error"),
    ),
    tag = "features"
)]
pub async fn write_online_features_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<WriteOnlineFeaturesRequestJson>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    state.metrics.record_store_write("online", "write");

    let online_store = state
        .online_store
        .get()
        .ok_or_else(|| internal_err("online store not configured"))?;

    let ek = parse_entity_key(&req.entity_key)?;
    let mut values = std::collections::HashMap::new();
    for (k, v) in &req.features {
        values.insert(k.clone(), serde_json::to_vec(v).unwrap_or_default());
    }

    let record = OnlineWriteRecord {
        entity_key: ek,
        values,
        timestamp: Utc::now(),
        feature_view_name: req.feature_view.clone(),
    };

    online_store
        .online_write_batch(vec![record], &req.project)
        .await
        .map_err(|e| internal_err(&e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "features written"
    })))
}

#[utoipa::path(
    post,
    path = "/v1/features:push",
    request_body = PushFeaturesRequestJson,
    responses(
        (status = 200, description = "Features pushed", body = PushFeaturesResponseJson),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal error"),
    ),
    tag = "features"
)]
pub async fn push_features_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<PushFeaturesRequestJson>,
) -> Result<Json<PushFeaturesResponseJson>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    state.metrics.record_store_write("online", "push");
    let project = req.project.as_deref().unwrap_or("default");

    let online_store = state
        .online_store
        .get()
        .ok_or_else(|| internal_err("online store not configured"))?;

    let mut ek = EntityKey::new(req.entity_key.clone());
    // Fill entity_values from features (we don't know types, use JSON bytes)
    for val in &req.entity_key {
        ek.join_keys.push(val.clone());
        ek.entity_values.push(val.as_bytes().to_vec());
    }

    let timestamp = req
        .event_timestamp
        .as_ref()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(Utc::now);

    let mut values = std::collections::HashMap::new();
    for (k, v) in &req.features {
        values.insert(k.clone(), serde_json::to_vec(v).unwrap_or_default());
    }

    let record = OnlineWriteRecord {
        entity_key: ek,
        values,
        timestamp,
        feature_view_name: req.feature_view.clone(),
    };

    match online_store.online_write_batch(vec![record], project).await {
        Ok(_) => Ok(Json(PushFeaturesResponseJson {
            ingested: 1,
            failed: 0,
            errors: vec![],
        })),
        Err(e) => Ok(Json(PushFeaturesResponseJson {
            ingested: 0,
            failed: 1,
            errors: vec![e.to_string()],
        })),
    }
}

fn parse_entity_key(
    s: &str,
) -> Result<EntityKey, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Try hex-encoded v3 entity key first
    if let Ok(bytes) = hex::decode(s)
        && let Ok(ek) = deserialize_entity_key_v3(&bytes)
    {
        return Ok(ek);
    }
    // Fall back: treat as a single join key
    Ok(EntityKey::new(vec![s.to_string()]))
}

fn bad_request(msg: &str) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "status": "error",
            "code": "INVALID_ARGUMENT",
            "message": msg,
        })),
    )
}

fn internal_err(msg: &str) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "status": "error",
            "code": "INTERNAL",
            "message": msg,
        })),
    )
}

pub fn features_router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/features:get-online", post(get_online_features_handler))
        .route(
            "/v1/features:write-online",
            post(write_online_features_handler),
        )
        .route("/v1/features:push", post(push_features_handler))
}
