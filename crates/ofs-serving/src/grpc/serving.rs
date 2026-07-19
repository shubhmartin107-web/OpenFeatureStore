use std::sync::Arc;

use ofs_core::types::{EntityKey, FeatureViewProjection, FeatureViewWithProjection};
use ofs_proto::core_proto::FeatureList;
use ofs_proto::serving_proto::get_online_features_request;
use ofs_proto::serving_proto::{
    GetFeastServingInfoRequest, GetFeastServingInfoResponse, GetOnlineFeaturesRequest,
    GetOnlineFeaturesResponse, serving_service_server,
};
use ofs_proto::types_proto::Value;
use prost::Message;

use crate::ServerState;

pub struct ServingService {
    pub state: Arc<ServerState>,
}

impl ServingService {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl serving_service_server::ServingService for ServingService {
    async fn get_feast_serving_info(
        &self,
        _request: tonic::Request<GetFeastServingInfoRequest>,
    ) -> Result<tonic::Response<GetFeastServingInfoResponse>, tonic::Status> {
        let resp = GetFeastServingInfoResponse {
            version: env!("CARGO_PKG_VERSION").into(),
        };
        Ok(tonic::Response::new(resp))
    }

    async fn get_online_features(
        &self,
        request: tonic::Request<GetOnlineFeaturesRequest>,
    ) -> Result<tonic::Response<GetOnlineFeaturesResponse>, tonic::Status> {
        let req = request.into_inner();
        self.state
            .metrics
            .record_feature_request("grpc", "requested");

        let online_store = self
            .state
            .online_store
            .get()
            .ok_or_else(|| tonic::Status::unavailable("online store not configured"))?;
        let registry = self
            .state
            .registry
            .get()
            .ok_or_else(|| tonic::Status::unavailable("registry not configured"))?;

        // Project is extracted from request context or defaults to "default"
        let project = req
            .request_context
            .get("project")
            .and_then(|v| v.val.first())
            .and_then(|val| match val {
                ofs_proto::types_proto::Value {
                    val: Some(ofs_proto::types_proto::value::Val::StringVal(s)),
                } => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "default".to_string());

        // Determine feature service or feature list
        let (feature_service_name, feature_names) = match req.kind {
            Some(get_online_features_request::Kind::FeatureService(ref name)) => {
                (Some(name.clone()), Vec::new())
            }
            Some(get_online_features_request::Kind::Features(ref list)) => (None, list.val.clone()),
            None => {
                return Err(tonic::Status::invalid_argument(
                    "must specify feature_service or features",
                ));
            }
        };

        // Build entity keys from request entities
        let entity_count = req
            .entities
            .values()
            .next()
            .map(|v| v.val.len())
            .unwrap_or(0);
        let mut entity_keys = Vec::with_capacity(entity_count);
        for i in 0..entity_count {
            let mut ek = EntityKey::new(Vec::new());
            for (key_name, repeated_val) in &req.entities {
                ek.join_keys.push(key_name.clone());
                if i < repeated_val.val.len() {
                    // Serialize the protobuf Value to bytes
                    let mut buf = Vec::new();
                    if let Some(val) = repeated_val.val.get(i)
                        && val.encode(&mut buf).is_err()
                    {
                        return Err(tonic::Status::internal("failed to encode entity value"));
                    }
                    ek.entity_values.push(buf);
                }
            }
            entity_keys.push(ek);
        }

        if entity_keys.is_empty() {
            return Err(tonic::Status::invalid_argument("no entity keys provided"));
        }

        // Resolve feature views
        let mut feature_fvps = Vec::new();
        if let Some(ref fs_name) = feature_service_name {
            let fs = registry
                .get_feature_service(fs_name, &project)
                .await
                .map_err(|e| tonic::Status::internal(e.to_string()))?
                .ok_or_else(|| {
                    tonic::Status::not_found(format!("feature service '{}' not found", fs_name))
                })?;

            for fvp_in in &fs.features {
                let fv = registry
                    .get_feature_view(&fvp_in.feature_view_name, &project)
                    .await
                    .map_err(|e| tonic::Status::internal(e.to_string()))?
                    .ok_or_else(|| {
                        tonic::Status::not_found(format!(
                            "feature view '{}' not found",
                            fvp_in.feature_view_name
                        ))
                    })?;

                feature_fvps.push(FeatureViewWithProjection {
                    feature_view: fv,
                    projection: fvp_in.clone(),
                });
            }
        } else {
            let all_fvs = registry
                .list_feature_views(&project)
                .await
                .map_err(|e| tonic::Status::internal(e.to_string()))?;

            for fv in &all_fvs {
                let mut matched_features = Vec::new();
                for feature_name in &feature_names {
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
            return Err(tonic::Status::not_found("no matching feature views found"));
        }

        let response = online_store
            .online_read(entity_keys, &feature_fvps, &project)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        // Convert to protobuf response
        let mut pb_results = Vec::new();
        for fv in &response.results {
            let mut pb_values = Vec::new();
            let mut pb_statuses = Vec::new();
            let mut pb_timestamps = Vec::new();

            for (i, val) in fv.values.iter().enumerate() {
                pb_values.push(Value {
                    val: Some(ofs_proto::types_proto::value::Val::StringVal(
                        String::from_utf8_lossy(val).to_string(),
                    )),
                });
                pb_statuses.push(match fv.statuses.get(i) {
                    Some(ofs_core::traits::FieldStatus::Present) => 1,
                    Some(ofs_core::traits::FieldStatus::NullValue) => 2,
                    Some(ofs_core::traits::FieldStatus::NotFound) => 3,
                    Some(ofs_core::traits::FieldStatus::OutsideMaxAge) => 4,
                    _ => 0,
                });
                pb_timestamps.push(match fv.event_timestamps.get(i).and_then(|t| *t) {
                    Some(dt) => prost_types::Timestamp {
                        seconds: dt.timestamp(),
                        nanos: dt.timestamp_subsec_nanos() as i32,
                    },
                    None => prost_types::Timestamp {
                        seconds: 0,
                        nanos: 0,
                    },
                });
            }

            pb_results.push(ofs_proto::serving_proto::FeatureVector {
                values: pb_values,
                statuses: pb_statuses,
                event_timestamps: pb_timestamps,
            });
        }

        let pb_metadata = ofs_proto::serving_proto::GetOnlineFeaturesResponseMetadata {
            feature_names: Some(FeatureList {
                val: response.metadata.feature_names.clone(),
            }),
            feature_view_metadata: Vec::new(),
        };

        let pb_response = GetOnlineFeaturesResponse {
            metadata: Some(pb_metadata),
            results: pb_results,
            status: true,
        };

        Ok(tonic::Response::new(pb_response))
    }
}
