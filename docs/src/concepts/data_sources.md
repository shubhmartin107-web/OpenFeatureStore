# Data Sources

A **DataSource** describes where feature data originates. It specifies the physical
location and format of the underlying data.

## Source Types

The following source types are supported:

| Source Type | Description |
|---|---|
| `BatchFile` | Files in Parquet, CSV, or Arrow format |
| `BatchBigQuery` | Google BigQuery table or query |
| `StreamKafka` | Apache Kafka topic |
| `StreamKinesis` | Amazon Kinesis stream |
| `BatchRedshift` | Amazon Redshift |
| `BatchSnowflake` | Snowflake |
| `RequestSource` | Request-time data (for on-demand FVs) |
| `PushSource` | Pushed streaming data |
| `CustomSource` | User-defined source |

## Options

Each source type has specific options:

```python
from ofs import DataSourceOptions

# File source
file_opts = DataSourceOptions.file(
    path="/data/features.parquet",
    file_format="parquet"
)

# A data source with file options
from ofs import DataSource
# Typically created via the registry or API
```

## Field Mapping

Data sources support field mapping to rename columns:

```rust
pub struct DataSource {
    pub name: String,
    pub source_type: SourceType,
    pub timestamp_field: Option<String>,
    pub created_timestamp_column: Option<String>,
    pub field_mapping: HashMap<String, String>,
    pub date_partition_column: Option<String>,
    pub options: DataSourceOptions,
}
```
