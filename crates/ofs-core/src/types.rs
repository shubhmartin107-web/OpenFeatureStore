use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::value_type::{FeastType, ValueType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceType {
    Invalid = 0,
    BatchFile = 1,
    BatchBigQuery = 2,
    StreamKafka = 3,
    StreamKinesis = 4,
    BatchRedshift = 5,
    CustomSource = 6,
    RequestSource = 7,
    BatchSnowflake = 8,
    PushSource = 9,
    BatchTrino = 10,
    BatchSpark = 11,
    BatchAthena = 12,
}

impl SourceType {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Invalid),
            1 => Some(Self::BatchFile),
            2 => Some(Self::BatchBigQuery),
            3 => Some(Self::StreamKafka),
            4 => Some(Self::StreamKinesis),
            5 => Some(Self::BatchRedshift),
            6 => Some(Self::CustomSource),
            7 => Some(Self::RequestSource),
            8 => Some(Self::BatchSnowflake),
            9 => Some(Self::PushSource),
            10 => Some(Self::BatchTrino),
            11 => Some(Self::BatchSpark),
            12 => Some(Self::BatchAthena),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileFormat {
    Parquet,
    Csv,
    Arrow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataSourceOptions {
    File {
        path: String,
        file_format: FileFormat,
        s3_endpoint_override: Option<String>,
    },
    BigQuery {
        table: Option<String>,
        query: Option<String>,
    },
    Redshift {
        table: Option<String>,
        query: Option<String>,
        schema_name: Option<String>,
        database: Option<String>,
    },
    Snowflake {
        table: Option<String>,
        query: Option<String>,
        schema_name: Option<String>,
        database: Option<String>,
    },
    Kafka {
        bootstrap_servers: String,
        topic: String,
    },
    Kinesis {
        region: String,
        stream_name: String,
    },
    Push {
        batch_source: Option<Box<DataSource>>,
    },
    Request {
        schema: Vec<Feature>,
    },
    Custom {
        class_type: String,
        config: Vec<u8>,
    },
    Spark {
        table: Option<String>,
        query: Option<String>,
        path: Option<String>,
        file_format: Option<String>,
    },
    Trino {
        table: Option<String>,
        query: Option<String>,
    },
    Athena {
        table: Option<String>,
        query: Option<String>,
        database: Option<String>,
        data_source: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataSource {
    pub name: String,
    pub project: String,
    pub source_type: SourceType,
    pub timestamp_field: Option<String>,
    pub created_timestamp_column: Option<String>,
    pub field_mapping: HashMap<String, String>,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub owner: String,
    pub date_partition_column: Option<String>,
    pub timestamp_field_type: Option<String>,
    pub options: DataSourceOptions,
}

impl DataSource {
    pub fn new(name: &str, options: DataSourceOptions) -> Self {
        let source_type = match &options {
            DataSourceOptions::File { .. } => SourceType::BatchFile,
            DataSourceOptions::BigQuery { .. } => SourceType::BatchBigQuery,
            DataSourceOptions::Redshift { .. } => SourceType::BatchRedshift,
            DataSourceOptions::Snowflake { .. } => SourceType::BatchSnowflake,
            DataSourceOptions::Kafka { .. } => SourceType::StreamKafka,
            DataSourceOptions::Kinesis { .. } => SourceType::StreamKinesis,
            DataSourceOptions::Push { .. } => SourceType::PushSource,
            DataSourceOptions::Request { .. } => SourceType::RequestSource,
            DataSourceOptions::Custom { .. } => SourceType::CustomSource,
            DataSourceOptions::Spark { .. } => SourceType::BatchSpark,
            DataSourceOptions::Trino { .. } => SourceType::BatchTrino,
            DataSourceOptions::Athena { .. } => SourceType::BatchAthena,
        };
        Self {
            name: name.to_string(),
            project: String::new(),
            source_type,
            timestamp_field: None,
            created_timestamp_column: None,
            field_mapping: HashMap::new(),
            description: String::new(),
            tags: HashMap::new(),
            owner: String::new(),
            date_partition_column: None,
            timestamp_field_type: None,
            options,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub name: String,
    pub value_type: ValueType,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub vector_index: bool,
    pub vector_search_metric: Option<String>,
    pub vector_length: i32,
}

impl Feature {
    pub fn new(name: &str, value_type: ValueType) -> Self {
        Self {
            name: name.to_string(),
            value_type,
            description: String::new(),
            tags: HashMap::new(),
            vector_index: false,
            vector_search_metric: None,
            vector_length: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub dtype: FeastType,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub vector_index: bool,
    pub vector_search_metric: Option<String>,
    pub vector_length: i32,
}

impl Field {
    pub fn new(name: &str, dtype: FeastType) -> Self {
        Self {
            name: name.to_string(),
            dtype,
            description: String::new(),
            tags: HashMap::new(),
            vector_index: false,
            vector_search_metric: None,
            vector_length: 0,
        }
    }
}

impl From<Feature> for Field {
    fn from(f: Feature) -> Self {
        Self {
            name: f.name,
            dtype: FeastType::from_value_type(f.value_type),
            description: f.description,
            tags: f.tags,
            vector_index: f.vector_index,
            vector_search_metric: f.vector_search_metric,
            vector_length: f.vector_length,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureViewState {
    StateUnspecified = 0,
    Created = 1,
    Generated = 2,
    Materializing = 3,
    AvailableOnline = 4,
}

impl FeatureViewState {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::StateUnspecified),
            1 => Some(Self::Created),
            2 => Some(Self::Generated),
            3 => Some(Self::Materializing),
            4 => Some(Self::AvailableOnline),
            _ => None,
        }
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (_, Self::StateUnspecified) => false,
            (Self::StateUnspecified, Self::Created) => true,
            (Self::StateUnspecified, Self::Materializing) => true,
            (Self::Created, Self::Materializing) => true,
            (Self::Created, Self::Generated) => true,
            (Self::Generated, Self::Materializing) => true,
            (Self::Materializing, Self::AvailableOnline) => true,
            (Self::AvailableOnline, Self::Materializing) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entity {
    pub name: String,
    pub project: String,
    pub join_keys: Vec<String>,
    pub value_type: ValueType,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub owner: String,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub last_updated_timestamp: Option<DateTime<Utc>>,
}

impl Entity {
    pub fn new(name: &str, join_keys: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            project: String::new(),
            join_keys,
            value_type: ValueType::String,
            description: String::new(),
            tags: HashMap::new(),
            owner: String::new(),
            created_timestamp: None,
            last_updated_timestamp: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureViewProjection {
    pub feature_view_name: String,
    pub feature_view_name_alias: Option<String>,
    pub feature_columns: Vec<Feature>,
    pub join_key_map: HashMap<String, String>,
    pub timestamp_field: Option<String>,
    pub date_partition_column: Option<String>,
    pub created_timestamp_column: Option<String>,
    pub batch_source: Option<DataSource>,
    pub stream_source: Option<DataSource>,
    pub view_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureView {
    pub name: String,
    pub project: String,
    pub entities: Vec<String>,
    pub features: Vec<Feature>,
    pub tags: HashMap<String, String>,
    pub ttl: Option<Duration>,
    pub batch_source: Option<DataSource>,
    pub stream_source: Option<DataSource>,
    pub online: bool,
    pub offline: bool,
    pub description: String,
    pub owner: String,
    pub org: String,
    pub mode: Option<String>,
    pub enable_validation: bool,
    pub version: String,
    pub disabled: bool,
    pub entity_columns: Vec<Feature>,
    pub materialization_intervals: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub last_updated_timestamp: Option<DateTime<Utc>>,
    pub state: FeatureViewState,
}

impl FeatureView {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            project: String::new(),
            entities: Vec::new(),
            features: Vec::new(),
            tags: HashMap::new(),
            ttl: None,
            batch_source: None,
            stream_source: None,
            online: true,
            offline: false,
            description: String::new(),
            owner: String::new(),
            org: String::new(),
            mode: None,
            enable_validation: false,
            version: "latest".to_string(),
            disabled: false,
            entity_columns: Vec::new(),
            materialization_intervals: Vec::new(),
            created_timestamp: None,
            last_updated_timestamp: None,
            state: FeatureViewState::StateUnspecified,
        }
    }

    pub fn most_recent_end_time(&self) -> Option<DateTime<Utc>> {
        self.materialization_intervals
            .iter()
            .map(|(_, end)| *end)
            .max()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OnDemandFeatureView {
    pub name: String,
    pub project: String,
    pub features: Vec<Feature>,
    pub sources: HashMap<String, OnDemandSource>,
    pub feature_transformation: Option<FeatureTransformation>,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub owner: String,
    pub mode: String,
    pub write_to_online_store: bool,
    pub entities: Vec<String>,
    pub entity_columns: Vec<Feature>,
    pub singleton: bool,
    pub version: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnDemandSource {
    FeatureView(FeatureView),
    FeatureViewProjection(FeatureViewProjection),
    RequestDataSource(DataSource),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureTransformation {
    pub udf: Option<UserDefinedFunction>,
    pub substrait_plan: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserDefinedFunction {
    pub name: String,
    pub body: Vec<u8>,
    pub body_text: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FeatureService {
    pub name: String,
    pub project: String,
    pub features: Vec<FeatureViewProjection>,
    pub tags: HashMap<String, String>,
    pub description: String,
    pub owner: String,
    pub precompute_online: bool,
    pub logging_config: Option<LoggingConfig>,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub last_updated_timestamp: Option<DateTime<Utc>>,
}

impl FeatureService {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            project: String::new(),
            features: Vec::new(),
            tags: HashMap::new(),
            description: String::new(),
            owner: String::new(),
            precompute_online: false,
            logging_config: None,
            created_timestamp: None,
            last_updated_timestamp: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoggingDestination {
    File {
        path: String,
        s3_endpoint_override: Option<String>,
        partition_by: Vec<String>,
    },
    BigQuery {
        table_ref: String,
    },
    Redshift {
        table_name: String,
    },
    Snowflake {
        table_name: String,
    },
    Athena {
        table_name: String,
    },
    Custom {
        kind: String,
        config: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoggingConfig {
    pub sample_rate: f32,
    pub destination: LoggingDestination,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,
            destination: LoggingDestination::File {
                path: String::new(),
                s3_endpoint_override: None,
                partition_by: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntityKey {
    pub join_keys: Vec<String>,
    pub entity_values: Vec<Vec<u8>>,
    pub value_types: Vec<ValueType>,
}

impl EntityKey {
    pub fn new(join_keys: Vec<String>) -> Self {
        Self {
            join_keys,
            entity_values: Vec::new(),
            value_types: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoConfig {
    pub project: String,
    pub provider: String,
    pub registry: RegistryConfig,
    pub online_store: OnlineStoreConfig,
    pub offline_store: OfflineStoreConfig,
    pub entity_key_serialization_version: i32,
    pub cache_ttl_seconds: i64,
    pub cache_mode: String,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            project: "default".to_string(),
            provider: "local".to_string(),
            registry: RegistryConfig::file("data/registry.db"),
            online_store: OnlineStoreConfig::Sqlite {
                path: "data/online.db".to_string(),
            },
            offline_store: OfflineStoreConfig::DuckDb { path: None },
            entity_key_serialization_version: 3,
            cache_ttl_seconds: 600,
            cache_mode: "sync".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub registry_type: String,
    pub path: String,
    pub cache_ttl_seconds: i64,
    pub cache_mode: String,
}

impl RegistryConfig {
    pub fn file(path: &str) -> Self {
        Self {
            registry_type: "file".to_string(),
            path: path.to_string(),
            cache_ttl_seconds: 600,
            cache_mode: "sync".to_string(),
        }
    }

    pub fn sql(path: &str) -> Self {
        Self {
            registry_type: "sql".to_string(),
            path: path.to_string(),
            cache_ttl_seconds: 600,
            cache_mode: "sync".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OnlineStoreConfig {
    Sqlite {
        path: String,
    },
    Redis {
        connection_string: String,
        key_ttl_seconds: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub enum OfflineStoreConfig {
    DuckDb { path: Option<String> },
}

#[derive(Debug, Clone)]
pub struct FeatureViewWithProjection {
    pub feature_view: FeatureView,
    pub projection: FeatureViewProjection,
}

#[derive(Debug, Clone)]
pub struct OnlineWriteRecord {
    pub entity_key: EntityKey,
    pub values: HashMap<String, Vec<u8>>,
    pub timestamp: DateTime<Utc>,
    pub feature_view_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct BackfillJob {
    pub id: String,
    pub feature_view_name: String,
    pub project: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub status: BackfillStatus,
    pub progress: f64,
    pub chunk_size_seconds: i64,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_new() {
        let e = Entity::new("driver", vec!["driver_id".to_string()]);
        assert_eq!(e.name, "driver");
        assert_eq!(e.join_keys, vec!["driver_id"]);
        assert_eq!(e.value_type, ValueType::String);
    }

    #[test]
    fn test_feature_new() {
        let f = Feature::new("conv_rate", ValueType::Double);
        assert_eq!(f.name, "conv_rate");
        assert_eq!(f.value_type, ValueType::Double);
    }

    #[test]
    fn test_field_from_feature() {
        let f = Feature::new("age", ValueType::Int64);
        let field: Field = f.into();
        assert_eq!(
            field.dtype,
            crate::value_type::FeastType::Primitive(crate::value_type::PrimitiveFeastType::Int64)
        );
    }

    #[test]
    fn test_feature_view_new() {
        let fv = FeatureView::new("driver_stats");
        assert_eq!(fv.name, "driver_stats");
        assert!(fv.online);
        assert_eq!(fv.version, "latest");
        assert_eq!(fv.state, FeatureViewState::StateUnspecified);
    }

    #[test]
    fn test_feature_view_most_recent_end_time() {
        let mut fv = FeatureView::new("test");
        let t1 = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t3 = DateTime::parse_from_rfc3339("2024-01-03T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        fv.materialization_intervals.push((t1, t2));
        fv.materialization_intervals.push((t2, t3));
        assert_eq!(fv.most_recent_end_time(), Some(t3));
    }

    #[test]
    fn test_feature_view_most_recent_end_time_empty() {
        let fv = FeatureView::new("test");
        assert_eq!(fv.most_recent_end_time(), None);
    }

    #[test]
    fn test_feature_view_state_transitions() {
        let state = FeatureViewState::StateUnspecified;
        assert!(state.can_transition_to(&FeatureViewState::Created));
        assert!(state.can_transition_to(&FeatureViewState::Materializing));
        assert!(!state.can_transition_to(&FeatureViewState::StateUnspecified));
        assert!(!state.can_transition_to(&FeatureViewState::AvailableOnline));

        let created = FeatureViewState::Created;
        assert!(created.can_transition_to(&FeatureViewState::Generated));
        assert!(created.can_transition_to(&FeatureViewState::Materializing));
    }

    #[test]
    fn test_feature_service_new() {
        let fs = FeatureService::new("model_v1");
        assert_eq!(fs.name, "model_v1");
        assert!(!fs.precompute_online);
    }

    #[test]
    fn test_data_source_new_file() {
        let ds = DataSource::new(
            "features",
            DataSourceOptions::File {
                path: "data/features.parquet".to_string(),
                file_format: FileFormat::Parquet,
                s3_endpoint_override: None,
            },
        );
        assert_eq!(ds.name, "features");
        assert_eq!(ds.source_type, SourceType::BatchFile);
    }

    #[test]
    fn test_source_type_from_i32() {
        assert_eq!(SourceType::from_i32(0), Some(SourceType::Invalid));
        assert_eq!(SourceType::from_i32(1), Some(SourceType::BatchFile));
        assert_eq!(SourceType::from_i32(9), Some(SourceType::PushSource));
        assert_eq!(SourceType::from_i32(99), None);
    }

    #[test]
    fn test_repo_config_default() {
        let cfg = RepoConfig::default();
        assert_eq!(cfg.project, "default");
        assert_eq!(cfg.entity_key_serialization_version, 3);
        assert_eq!(cfg.provider, "local");
    }
}
