use chrono::{DateTime, Utc};
use ofs_core::types::{
    DataSource, DataSourceOptions, Entity, Feature, FeatureService, FeatureTransformation,
    FeatureView, FeatureViewProjection, FeatureViewState, FileFormat, LoggingConfig,
    LoggingDestination, OnDemandFeatureView, OnDemandSource, SourceType, UserDefinedFunction,
};
use ofs_core::value_type::ValueType;
use ofs_proto::feast::core::{self as proto};
use prost_types::{Duration, Timestamp};

// ---------------------------------------------------------------------------
//  Timestamp helpers
// ---------------------------------------------------------------------------

fn ts_to_dt(ts: &Option<Timestamp>) -> Option<DateTime<Utc>> {
    ts.as_ref()
        .and_then(|t| DateTime::from_timestamp(t.seconds, t.nanos as u32))
}

fn dt_to_ts(dt: &Option<DateTime<Utc>>) -> Option<Timestamp> {
    dt.map(|d| Timestamp {
        seconds: d.timestamp(),
        nanos: d.timestamp_subsec_nanos() as i32,
    })
}

fn dur_to_std(d: &Option<Duration>) -> Option<std::time::Duration> {
    d.as_ref()
        .map(|d| std::time::Duration::new(d.seconds as u64, d.nanos as u32))
}

fn std_to_dur(d: &Option<std::time::Duration>) -> Option<Duration> {
    d.map(|d| Duration {
        seconds: d.as_secs() as i64,
        nanos: d.subsec_nanos() as i32,
    })
}

// ---------------------------------------------------------------------------
//  ValueType conversion
// ---------------------------------------------------------------------------

fn vt_from_proto(v: i32) -> ValueType {
    ValueType::from_i32(v).unwrap_or(ValueType::Invalid)
}

fn vt_to_proto(v: ValueType) -> i32 {
    v as i32
}

// ---------------------------------------------------------------------------
//  FeatureSpecV2 <-> Feature
// ---------------------------------------------------------------------------

fn feature_from_proto(f: &proto::FeatureSpecV2) -> Feature {
    Feature {
        name: f.name.clone(),
        value_type: vt_from_proto(f.value_type),
        description: f.description.clone(),
        tags: f.tags.clone(),
        vector_index: f.vector_index,
        vector_search_metric: if f.vector_search_metric.is_empty() {
            None
        } else {
            Some(f.vector_search_metric.clone())
        },
        vector_length: f.vector_length,
    }
}

fn feature_to_proto(f: &Feature) -> proto::FeatureSpecV2 {
    proto::FeatureSpecV2 {
        name: f.name.clone(),
        value_type: vt_to_proto(f.value_type),
        tags: f.tags.clone(),
        description: f.description.clone(),
        vector_index: f.vector_index,
        vector_search_metric: f.vector_search_metric.clone().unwrap_or_default(),
        vector_length: f.vector_length,
    }
}

// ---------------------------------------------------------------------------
//  FileFormat conversion
// ---------------------------------------------------------------------------

fn file_format_from_proto(f: &proto::FileFormat) -> FileFormat {
    use proto::file_format::Format;
    match &f.format {
        Some(Format::ParquetFormat(_)) => FileFormat::Parquet,
        None => FileFormat::Parquet,
    }
}

fn file_format_to_proto(f: &FileFormat) -> proto::FileFormat {
    use proto::file_format::Format;
    match f {
        FileFormat::Parquet => proto::FileFormat {
            format: Some(Format::ParquetFormat(proto::file_format::ParquetFormat {})),
        },
        // Csv and Arrow map to Parquet in proto — the protobuf schema only defines
        // a ParquetFormat variant. Lossless round-trip requires extending the proto.
        FileFormat::Csv | FileFormat::Arrow => proto::FileFormat {
            format: Some(Format::ParquetFormat(proto::file_format::ParquetFormat {})),
        },
    }
}

// ---------------------------------------------------------------------------
//  DataSource <-> proto DataSource
// ---------------------------------------------------------------------------

fn source_type_from_proto(t: i32) -> SourceType {
    SourceType::from_i32(t).unwrap_or(SourceType::Invalid)
}

fn data_source_options_from_proto(ds: &proto::DataSource) -> DataSourceOptions {
    use proto::data_source::Options;
    match &ds.options {
        Some(Options::FileOptions(o)) => DataSourceOptions::File {
            path: o.uri.clone(),
            file_format: file_format_from_proto(
                o.file_format
                    .as_ref()
                    .unwrap_or(&proto::FileFormat { format: None }),
            ),
            s3_endpoint_override: if o.s3_endpoint_override.is_empty() {
                None
            } else {
                Some(o.s3_endpoint_override.clone())
            },
        },
        Some(Options::BigqueryOptions(o)) => DataSourceOptions::BigQuery {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
        },
        Some(Options::RedshiftOptions(o)) => DataSourceOptions::Redshift {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
            schema_name: if o.schema.is_empty() {
                None
            } else {
                Some(o.schema.clone())
            },
            database: if o.database.is_empty() {
                None
            } else {
                Some(o.database.clone())
            },
        },
        Some(Options::SnowflakeOptions(o)) => DataSourceOptions::Snowflake {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
            schema_name: if o.schema.is_empty() {
                None
            } else {
                Some(o.schema.clone())
            },
            database: if o.database.is_empty() {
                None
            } else {
                Some(o.database.clone())
            },
        },
        Some(Options::KafkaOptions(o)) => DataSourceOptions::Kafka {
            bootstrap_servers: o.kafka_bootstrap_servers.clone(),
            topic: o.topic.clone(),
        },
        Some(Options::KinesisOptions(o)) => DataSourceOptions::Kinesis {
            region: o.region.clone(),
            stream_name: o.stream_name.clone(),
        },
        Some(Options::RequestDataOptions(o)) => DataSourceOptions::Request {
            schema: o.schema.iter().map(feature_from_proto).collect(),
        },
        Some(Options::CustomOptions(o)) => DataSourceOptions::Custom {
            class_type: String::new(),
            config: o.configuration.to_vec(),
        },
        Some(Options::PushOptions(_)) => DataSourceOptions::Push { batch_source: None },
        Some(Options::SparkOptions(o)) => DataSourceOptions::Spark {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
            path: if o.path.is_empty() {
                None
            } else {
                Some(o.path.clone())
            },
            file_format: if o.file_format.is_empty() {
                None
            } else {
                Some(o.file_format.clone())
            },
        },
        Some(Options::TrinoOptions(o)) => DataSourceOptions::Trino {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
        },
        Some(Options::AthenaOptions(o)) => DataSourceOptions::Athena {
            table: if o.table.is_empty() {
                None
            } else {
                Some(o.table.clone())
            },
            query: if o.query.is_empty() {
                None
            } else {
                Some(o.query.clone())
            },
            database: if o.database.is_empty() {
                None
            } else {
                Some(o.database.clone())
            },
            data_source: if o.data_source.is_empty() {
                None
            } else {
                Some(o.data_source.clone())
            },
        },
        None => DataSourceOptions::File {
            path: String::new(),
            file_format: FileFormat::Parquet,
            s3_endpoint_override: None,
        },
    }
}

fn data_source_options_to_proto(
    o: &DataSourceOptions,
) -> (i32, Option<proto::data_source::Options>) {
    use proto::data_source;
    match o {
        DataSourceOptions::File {
            path,
            file_format,
            s3_endpoint_override,
        } => (
            proto::data_source::SourceType::BatchFile as i32,
            Some(data_source::Options::FileOptions(
                data_source::FileOptions {
                    file_format: Some(file_format_to_proto(file_format)),
                    uri: path.clone(),
                    s3_endpoint_override: s3_endpoint_override.clone().unwrap_or_default(),
                },
            )),
        ),
        DataSourceOptions::BigQuery { table, query } => (
            proto::data_source::SourceType::BatchBigquery as i32,
            Some(data_source::Options::BigqueryOptions(
                data_source::BigQueryOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                },
            )),
        ),
        DataSourceOptions::Redshift {
            table,
            query,
            schema_name,
            database,
        } => (
            proto::data_source::SourceType::BatchRedshift as i32,
            Some(data_source::Options::RedshiftOptions(
                data_source::RedshiftOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                    schema: schema_name.clone().unwrap_or_default(),
                    database: database.clone().unwrap_or_default(),
                },
            )),
        ),
        DataSourceOptions::Snowflake {
            table,
            query,
            schema_name,
            database,
        } => (
            proto::data_source::SourceType::BatchSnowflake as i32,
            Some(data_source::Options::SnowflakeOptions(
                data_source::SnowflakeOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                    schema: schema_name.clone().unwrap_or_default(),
                    database: database.clone().unwrap_or_default(),
                },
            )),
        ),
        DataSourceOptions::Kafka {
            bootstrap_servers,
            topic,
        } => (
            proto::data_source::SourceType::StreamKafka as i32,
            Some(data_source::Options::KafkaOptions(
                data_source::KafkaOptions {
                    kafka_bootstrap_servers: bootstrap_servers.clone(),
                    topic: topic.clone(),
                    message_format: None,
                    watermark_delay_threshold: None,
                },
            )),
        ),
        DataSourceOptions::Kinesis {
            region,
            stream_name,
        } => (
            proto::data_source::SourceType::StreamKinesis as i32,
            Some(data_source::Options::KinesisOptions(
                data_source::KinesisOptions {
                    region: region.clone(),
                    stream_name: stream_name.clone(),
                    record_format: None,
                },
            )),
        ),
        DataSourceOptions::Request { schema } => (
            proto::data_source::SourceType::RequestSource as i32,
            Some(data_source::Options::RequestDataOptions(
                data_source::RequestDataOptions {
                    schema: schema.iter().map(feature_to_proto).collect(),
                },
            )),
        ),
        DataSourceOptions::Custom {
            class_type: _,
            config,
        } => (
            proto::data_source::SourceType::CustomSource as i32,
            Some(data_source::Options::CustomOptions(
                data_source::CustomSourceOptions {
                    configuration: config.clone(),
                },
            )),
        ),
        DataSourceOptions::Push { batch_source: _ } => (
            proto::data_source::SourceType::PushSource as i32,
            Some(data_source::Options::PushOptions(
                data_source::PushOptions {},
            )),
        ),
        DataSourceOptions::Spark {
            table,
            query,
            path,
            file_format,
        } => (
            proto::data_source::SourceType::BatchSpark as i32,
            Some(data_source::Options::SparkOptions(
                data_source::SparkOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                    path: path.clone().unwrap_or_default(),
                    file_format: file_format.clone().unwrap_or_default(),
                    date_partition_column_format: String::new(),
                    table_format: None,
                },
            )),
        ),
        DataSourceOptions::Trino { table, query } => (
            proto::data_source::SourceType::BatchTrino as i32,
            Some(data_source::Options::TrinoOptions(
                data_source::TrinoOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                },
            )),
        ),
        DataSourceOptions::Athena {
            table,
            query,
            database,
            data_source,
        } => (
            proto::data_source::SourceType::BatchAthena as i32,
            Some(data_source::Options::AthenaOptions(
                data_source::AthenaOptions {
                    table: table.clone().unwrap_or_default(),
                    query: query.clone().unwrap_or_default(),
                    database: database.clone().unwrap_or_default(),
                    data_source: data_source.clone().unwrap_or_default(),
                },
            )),
        ),
    }
}

pub fn data_source_from_proto(pds: &proto::DataSource) -> DataSource {
    let options = data_source_options_from_proto(pds);
    DataSource {
        name: pds.name.clone(),
        project: pds.project.clone(),
        source_type: source_type_from_proto(pds.r#type),
        timestamp_field: if pds.timestamp_field.is_empty() {
            None
        } else {
            Some(pds.timestamp_field.clone())
        },
        created_timestamp_column: if pds.created_timestamp_column.is_empty() {
            None
        } else {
            Some(pds.created_timestamp_column.clone())
        },
        field_mapping: pds.field_mapping.clone(),
        description: pds.description.clone(),
        tags: pds.tags.clone(),
        owner: pds.owner.clone(),
        date_partition_column: if pds.date_partition_column.is_empty() {
            None
        } else {
            Some(pds.date_partition_column.clone())
        },
        timestamp_field_type: if pds.timestamp_field_type.is_empty() {
            None
        } else {
            Some(pds.timestamp_field_type.clone())
        },
        options,
    }
}

pub fn data_source_to_proto(ds: &DataSource) -> proto::DataSource {
    let (st, opts) = data_source_options_to_proto(&ds.options);
    proto::DataSource {
        name: ds.name.clone(),
        project: ds.project.clone(),
        description: ds.description.clone(),
        tags: ds.tags.clone(),
        owner: ds.owner.clone(),
        r#type: st,
        field_mapping: ds.field_mapping.clone(),
        timestamp_field: ds.timestamp_field.clone().unwrap_or_default(),
        date_partition_column: ds.date_partition_column.clone().unwrap_or_default(),
        created_timestamp_column: ds.created_timestamp_column.clone().unwrap_or_default(),
        timestamp_field_type: ds.timestamp_field_type.clone().unwrap_or_default(),
        data_source_class_type: String::new(),
        batch_source: None,
        meta: None,
        options: opts,
    }
}

// ---------------------------------------------------------------------------
//  Entity <-> proto Entity
// ---------------------------------------------------------------------------

pub fn entity_from_proto(pe: &proto::Entity) -> Entity {
    let spec = match pe.spec.as_ref() {
        Some(s) => s,
        None => return Entity::default(),
    };
    let meta = pe.meta.as_ref();
    Entity {
        name: spec.name.clone(),
        project: spec.project.clone(),
        join_keys: if spec.join_key.is_empty() {
            Vec::new()
        } else {
            vec![spec.join_key.clone()]
        },
        value_type: vt_from_proto(spec.value_type),
        description: spec.description.clone(),
        tags: spec.tags.clone(),
        owner: spec.owner.clone(),
        created_timestamp: meta.and_then(|m| ts_to_dt(&m.created_timestamp)),
        last_updated_timestamp: meta.and_then(|m| ts_to_dt(&m.last_updated_timestamp)),
    }
}

pub fn entity_to_proto(e: &Entity) -> proto::Entity {
    proto::Entity {
        spec: Some(proto::EntitySpecV2 {
            name: e.name.clone(),
            project: e.project.clone(),
            value_type: vt_to_proto(e.value_type),
            description: e.description.clone(),
            join_key: e.join_keys.first().cloned().unwrap_or_default(),
            tags: e.tags.clone(),
            owner: e.owner.clone(),
        }),
        meta: Some(proto::EntityMeta {
            created_timestamp: dt_to_ts(&e.created_timestamp),
            last_updated_timestamp: dt_to_ts(&e.last_updated_timestamp),
        }),
    }
}

// ---------------------------------------------------------------------------
//  FeatureViewState
// ---------------------------------------------------------------------------

fn state_from_proto(s: i32) -> FeatureViewState {
    FeatureViewState::from_i32(s).unwrap_or(FeatureViewState::StateUnspecified)
}

fn state_to_proto(s: FeatureViewState) -> i32 {
    s as i32
}

// ---------------------------------------------------------------------------
//  MaterializationInterval
// ---------------------------------------------------------------------------

pub fn intervals_from_proto(
    intervals: &[proto::MaterializationInterval],
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    intervals
        .iter()
        .filter_map(|mi| {
            let start = ts_to_dt(&mi.start_time)?;
            let end = ts_to_dt(&mi.end_time)?;
            Some((start, end))
        })
        .collect()
}

fn intervals_to_proto(
    intervals: &[(DateTime<Utc>, DateTime<Utc>)],
) -> Vec<proto::MaterializationInterval> {
    intervals
        .iter()
        .map(|(start, end)| proto::MaterializationInterval {
            start_time: dt_to_ts(&Some(*start)),
            end_time: dt_to_ts(&Some(*end)),
        })
        .collect()
}

// ---------------------------------------------------------------------------
//  LoggingConfig
// ---------------------------------------------------------------------------

fn logging_from_proto(lc: &proto::LoggingConfig) -> LoggingConfig {
    use proto::logging_config::Destination;
    let destination = match &lc.destination {
        Some(Destination::FileDestination(f)) => LoggingDestination::File {
            path: f.path.clone(),
            s3_endpoint_override: if f.s3_endpoint_override.is_empty() {
                None
            } else {
                Some(f.s3_endpoint_override.clone())
            },
            partition_by: f.partition_by.clone(),
        },
        Some(Destination::BigqueryDestination(b)) => LoggingDestination::BigQuery {
            table_ref: b.table_ref.clone(),
        },
        Some(Destination::RedshiftDestination(r)) => LoggingDestination::Redshift {
            table_name: r.table_name.clone(),
        },
        Some(Destination::SnowflakeDestination(s)) => LoggingDestination::Snowflake {
            table_name: s.table_name.clone(),
        },
        Some(Destination::AthenaDestination(a)) => LoggingDestination::Athena {
            table_name: a.table_name.clone(),
        },
        Some(Destination::CustomDestination(c)) => LoggingDestination::Custom {
            kind: c.kind.clone(),
            config: c.config.clone(),
        },
        None => LoggingDestination::File {
            path: String::new(),
            s3_endpoint_override: None,
            partition_by: Vec::new(),
        },
    };
    LoggingConfig {
        sample_rate: lc.sample_rate,
        destination,
    }
}

fn logging_to_proto(lc: &LoggingConfig) -> proto::LoggingConfig {
    use proto::logging_config::Destination;
    let destination = match &lc.destination {
        LoggingDestination::File {
            path,
            s3_endpoint_override,
            partition_by,
        } => Some(Destination::FileDestination(
            proto::logging_config::FileDestination {
                path: path.clone(),
                s3_endpoint_override: s3_endpoint_override.clone().unwrap_or_default(),
                partition_by: partition_by.clone(),
            },
        )),
        LoggingDestination::BigQuery { table_ref } => Some(Destination::BigqueryDestination(
            proto::logging_config::BigQueryDestination {
                table_ref: table_ref.clone(),
            },
        )),
        LoggingDestination::Redshift { table_name } => Some(Destination::RedshiftDestination(
            proto::logging_config::RedshiftDestination {
                table_name: table_name.clone(),
            },
        )),
        LoggingDestination::Snowflake { table_name } => Some(Destination::SnowflakeDestination(
            proto::logging_config::SnowflakeDestination {
                table_name: table_name.clone(),
            },
        )),
        LoggingDestination::Athena { table_name } => Some(Destination::AthenaDestination(
            proto::logging_config::AthenaDestination {
                table_name: table_name.clone(),
            },
        )),
        LoggingDestination::Custom { kind, config } => Some(Destination::CustomDestination(
            proto::logging_config::CustomDestination {
                kind: kind.clone(),
                config: config.clone(),
            },
        )),
    };
    proto::LoggingConfig {
        sample_rate: lc.sample_rate,
        destination,
    }
}

// ---------------------------------------------------------------------------
//  FeatureViewProjection <-> proto FeatureViewProjection
// ---------------------------------------------------------------------------

pub fn projection_from_proto(pp: &proto::FeatureViewProjection) -> FeatureViewProjection {
    FeatureViewProjection {
        feature_view_name: pp.feature_view_name.clone(),
        feature_view_name_alias: if pp.feature_view_name_alias.is_empty() {
            None
        } else {
            Some(pp.feature_view_name_alias.clone())
        },
        feature_columns: pp.feature_columns.iter().map(feature_from_proto).collect(),
        join_key_map: pp.join_key_map.clone(),
        timestamp_field: if pp.timestamp_field.is_empty() {
            None
        } else {
            Some(pp.timestamp_field.clone())
        },
        date_partition_column: if pp.date_partition_column.is_empty() {
            None
        } else {
            Some(pp.date_partition_column.clone())
        },
        created_timestamp_column: if pp.created_timestamp_column.is_empty() {
            None
        } else {
            Some(pp.created_timestamp_column.clone())
        },
        batch_source: pp.batch_source.as_ref().map(data_source_from_proto),
        stream_source: pp.stream_source.as_ref().map(data_source_from_proto),
        view_type: pp.view_type.clone(),
    }
}

pub fn projection_to_proto(p: &FeatureViewProjection) -> proto::FeatureViewProjection {
    proto::FeatureViewProjection {
        feature_view_name: p.feature_view_name.clone(),
        feature_view_name_alias: p.feature_view_name_alias.clone().unwrap_or_default(),
        feature_columns: p.feature_columns.iter().map(feature_to_proto).collect(),
        join_key_map: p.join_key_map.clone(),
        timestamp_field: p.timestamp_field.clone().unwrap_or_default(),
        date_partition_column: p.date_partition_column.clone().unwrap_or_default(),
        created_timestamp_column: p.created_timestamp_column.clone().unwrap_or_default(),
        batch_source: p.batch_source.as_ref().map(data_source_to_proto),
        stream_source: p.stream_source.as_ref().map(data_source_to_proto),
        version_tag: None,
        view_type: p.view_type.clone(),
    }
}

// ---------------------------------------------------------------------------
//  FeatureView <-> proto FeatureView
// ---------------------------------------------------------------------------

pub fn feature_view_from_proto(pfv: &proto::FeatureView) -> FeatureView {
    let spec = pfv.spec.as_ref();
    let meta = pfv.meta.as_ref();
    FeatureView {
        name: spec.map(|s| s.name.clone()).unwrap_or_default(),
        project: spec.map(|s| s.project.clone()).unwrap_or_default(),
        entities: spec.map(|s| s.entities.clone()).unwrap_or_default(),
        features: spec
            .map(|s| s.features.iter().map(feature_from_proto).collect())
            .unwrap_or_default(),
        tags: spec.map(|s| s.tags.clone()).unwrap_or_default(),
        ttl: spec.and_then(|s| dur_to_std(&s.ttl)),
        batch_source: spec.and_then(|s| s.batch_source.as_ref().map(data_source_from_proto)),
        stream_source: spec.and_then(|s| s.stream_source.as_ref().map(data_source_from_proto)),
        online: spec.map(|s| s.online).unwrap_or(true),
        offline: spec.map(|s| s.offline).unwrap_or(false),
        description: spec.map(|s| s.description.clone()).unwrap_or_default(),
        owner: spec.map(|s| s.owner.clone()).unwrap_or_default(),
        org: spec.map(|s| s.org.clone()).unwrap_or_default(),
        mode: spec.and_then(|s| {
            if s.mode.is_empty() {
                None
            } else {
                Some(s.mode.clone())
            }
        }),
        enable_validation: spec.map(|s| s.enable_validation).unwrap_or(false),
        version: spec
            .map(|s| s.version.clone())
            .unwrap_or_else(|| "latest".to_string()),
        disabled: spec.map(|s| s.disabled).unwrap_or(false),
        entity_columns: spec
            .map(|s| s.entity_columns.iter().map(feature_from_proto).collect())
            .unwrap_or_default(),
        materialization_intervals: meta
            .map(|m| intervals_from_proto(&m.materialization_intervals))
            .unwrap_or_default(),
        created_timestamp: meta.and_then(|m| ts_to_dt(&m.created_timestamp)),
        last_updated_timestamp: meta.and_then(|m| ts_to_dt(&m.last_updated_timestamp)),
        state: meta
            .map(|m| state_from_proto(m.state))
            .unwrap_or(FeatureViewState::StateUnspecified),
    }
}

pub fn feature_view_to_proto(fv: &FeatureView) -> proto::FeatureView {
    proto::FeatureView {
        spec: Some(proto::FeatureViewSpec {
            name: fv.name.clone(),
            project: fv.project.clone(),
            entities: fv.entities.clone(),
            features: fv.features.iter().map(feature_to_proto).collect(),
            tags: fv.tags.clone(),
            ttl: std_to_dur(&fv.ttl),
            batch_source: fv.batch_source.as_ref().map(data_source_to_proto),
            online: fv.online,
            stream_source: fv.stream_source.as_ref().map(data_source_to_proto),
            description: fv.description.clone(),
            owner: fv.owner.clone(),
            entity_columns: fv.entity_columns.iter().map(feature_to_proto).collect(),
            offline: fv.offline,
            source_views: Vec::new(),
            feature_transformation: None,
            mode: fv.mode.clone().unwrap_or_default(),
            enable_validation: fv.enable_validation,
            version: fv.version.clone(),
            org: fv.org.clone(),
            disabled: fv.disabled,
        }),
        meta: Some(proto::FeatureViewMeta {
            created_timestamp: dt_to_ts(&fv.created_timestamp),
            last_updated_timestamp: dt_to_ts(&fv.last_updated_timestamp),
            materialization_intervals: intervals_to_proto(&fv.materialization_intervals),
            current_version_number: 1,
            version_id: String::new(),
            state: state_to_proto(fv.state.clone()),
        }),
    }
}

// ---------------------------------------------------------------------------
//  OnDemandSource conversion
// ---------------------------------------------------------------------------

fn on_demand_source_from_proto(s: &proto::OnDemandSource) -> OnDemandSource {
    use proto::on_demand_source::Source;
    match &s.source {
        Some(Source::FeatureView(fv)) => OnDemandSource::FeatureView(feature_view_from_proto(fv)),
        Some(Source::FeatureViewProjection(p)) => {
            OnDemandSource::FeatureViewProjection(projection_from_proto(p))
        }
        Some(Source::RequestDataSource(ds)) => {
            OnDemandSource::RequestDataSource(data_source_from_proto(ds))
        }
        None => OnDemandSource::RequestDataSource(DataSource::new(
            "unknown",
            DataSourceOptions::Request { schema: Vec::new() },
        )),
    }
}

fn on_demand_source_to_proto(s: &OnDemandSource) -> proto::OnDemandSource {
    use proto::on_demand_source::Source;
    match s {
        OnDemandSource::FeatureView(fv) => proto::OnDemandSource {
            source: Some(Source::FeatureView(feature_view_to_proto(fv))),
        },
        OnDemandSource::FeatureViewProjection(p) => proto::OnDemandSource {
            source: Some(Source::FeatureViewProjection(projection_to_proto(p))),
        },
        OnDemandSource::RequestDataSource(ds) => proto::OnDemandSource {
            source: Some(Source::RequestDataSource(data_source_to_proto(ds))),
        },
    }
}

// ---------------------------------------------------------------------------
//  OnDemandFeatureView <-> proto OnDemandFeatureView
// ---------------------------------------------------------------------------

pub fn odfv_from_proto(podfv: &proto::OnDemandFeatureView) -> OnDemandFeatureView {
    let spec = match podfv.spec.as_ref() {
        Some(s) => s,
        None => return OnDemandFeatureView::default(),
    };
    OnDemandFeatureView {
        name: spec.name.clone(),
        project: spec.project.clone(),
        features: spec.features.iter().map(feature_from_proto).collect(),
        sources: spec
            .sources
            .iter()
            .map(|(k, v)| (k.clone(), on_demand_source_from_proto(v)))
            .collect(),
        feature_transformation: spec.feature_transformation.as_ref().and_then(|ft| {
            use ofs_proto::feast::core::feature_transformation_v2::Transformation;
            ft.transformation.as_ref().map(|t| match t {
                Transformation::UserDefinedFunction(u) => FeatureTransformation {
                    udf: Some(UserDefinedFunction {
                        name: u.name.clone(),
                        body: u.body.to_vec(),
                        body_text: u.body_text.clone(),
                        mode: u.mode.clone(),
                    }),
                    substrait_plan: None,
                },
                Transformation::SubstraitTransformation(s) => FeatureTransformation {
                    udf: None,
                    substrait_plan: Some(s.substrait_plan.to_vec()),
                },
            })
        }),
        description: spec.description.clone(),
        tags: spec.tags.clone(),
        owner: spec.owner.clone(),
        mode: spec.mode.clone(),
        write_to_online_store: spec.write_to_online_store,
        entities: spec.entities.clone(),
        entity_columns: spec.entity_columns.iter().map(feature_from_proto).collect(),
        singleton: spec.singleton,
        version: if spec.version.is_empty() {
            "latest".to_string()
        } else {
            spec.version.clone()
        },
        disabled: spec.disabled,
    }
}

pub fn odfv_to_proto(odfv: &OnDemandFeatureView) -> proto::OnDemandFeatureView {
    proto::OnDemandFeatureView {
        spec: Some(proto::OnDemandFeatureViewSpec {
            name: odfv.name.clone(),
            project: odfv.project.clone(),
            features: odfv.features.iter().map(feature_to_proto).collect(),
            sources: odfv
                .sources
                .iter()
                .map(|(k, v)| (k.clone(), on_demand_source_to_proto(v)))
                .collect(),
            feature_transformation: odfv.feature_transformation.as_ref().and_then(|ft| {
                use ofs_proto::feast::core::feature_transformation_v2::Transformation;
                ft.udf
                    .as_ref()
                    .map(|u| proto::FeatureTransformationV2 {
                        transformation: Some(Transformation::UserDefinedFunction(
                            proto::UserDefinedFunctionV2 {
                                name: u.name.clone(),
                                body: u.body.clone(),
                                body_text: u.body_text.clone(),
                                mode: u.mode.clone(),
                            },
                        )),
                    })
                    .or_else(|| {
                        ft.substrait_plan
                            .as_ref()
                            .map(|plan| proto::FeatureTransformationV2 {
                                transformation: Some(Transformation::SubstraitTransformation(
                                    proto::SubstraitTransformationV2 {
                                        substrait_plan: plan.clone(),
                                        ibis_function: Vec::new(),
                                    },
                                )),
                            })
                    })
            }),
            description: odfv.description.clone(),
            tags: odfv.tags.clone(),
            owner: odfv.owner.clone(),
            mode: odfv.mode.clone(),
            write_to_online_store: odfv.write_to_online_store,
            entities: odfv.entities.clone(),
            entity_columns: odfv.entity_columns.iter().map(feature_to_proto).collect(),
            singleton: odfv.singleton,
            aggregations: Vec::new(),
            version: odfv.version.clone(),
            org: String::new(),
            disabled: odfv.disabled,
        }),
        meta: Some(proto::OnDemandFeatureViewMeta {
            created_timestamp: None,
            last_updated_timestamp: None,
            current_version_number: 1,
            version_id: String::new(),
            state: 0,
        }),
    }
}

// ---------------------------------------------------------------------------
//  FeatureService <-> proto FeatureService
// ---------------------------------------------------------------------------

pub fn feature_service_from_proto(pfs: &proto::FeatureService) -> FeatureService {
    let spec = match pfs.spec.as_ref() {
        Some(s) => s,
        None => return FeatureService::default(),
    };
    FeatureService {
        name: spec.name.clone(),
        project: spec.project.clone(),
        features: spec.features.iter().map(projection_from_proto).collect(),
        tags: spec.tags.clone(),
        description: spec.description.clone(),
        owner: spec.owner.clone(),
        precompute_online: spec.precompute_online,
        logging_config: spec.logging_config.as_ref().map(logging_from_proto),
        created_timestamp: pfs
            .meta
            .as_ref()
            .and_then(|m| ts_to_dt(&m.created_timestamp)),
        last_updated_timestamp: pfs
            .meta
            .as_ref()
            .and_then(|m| ts_to_dt(&m.last_updated_timestamp)),
    }
}

pub fn feature_service_to_proto(fs: &FeatureService) -> proto::FeatureService {
    proto::FeatureService {
        spec: Some(proto::FeatureServiceSpec {
            name: fs.name.clone(),
            project: fs.project.clone(),
            features: fs.features.iter().map(projection_to_proto).collect(),
            tags: fs.tags.clone(),
            description: fs.description.clone(),
            owner: fs.owner.clone(),
            logging_config: fs.logging_config.as_ref().map(logging_to_proto),
            precompute_online: fs.precompute_online,
        }),
        meta: Some(proto::FeatureServiceMeta {
            created_timestamp: dt_to_ts(&fs.created_timestamp),
            last_updated_timestamp: dt_to_ts(&fs.last_updated_timestamp),
        }),
    }
}
