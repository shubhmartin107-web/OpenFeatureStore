use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
}

impl CloudProvider {
    pub fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme {
            "s3" => Some(Self::Aws),
            "gs" => Some(Self::Gcp),
            "az" | "azure" | "abfs" => Some(Self::Azure),
            _ => None,
        }
    }

    pub fn default_scheme(&self) -> &'static str {
        match self {
            Self::Aws => "s3",
            Self::Gcp => "gs",
            Self::Azure => "az",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileInfo {
    pub location: RemoteLocation,
    pub size: u64,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemoteLocation {
    pub provider: CloudProvider,
    pub bucket: String,
    pub key: String,
}

impl RemoteLocation {
    pub fn from_uri(uri: &str) -> Result<Self, crate::error::RemoteStoreError> {
        let parsed = url::Url::parse(uri)?;
        let scheme = parsed.scheme();
        let provider = CloudProvider::from_scheme(scheme)
            .ok_or_else(|| crate::error::RemoteStoreError::UnsupportedScheme(scheme.to_string()))?;
        let bucket = parsed
            .host_str()
            .ok_or_else(|| {
                crate::error::RemoteStoreError::InvalidField(
                    "missing bucket/host in remote URI".into(),
                )
            })?
            .to_string();
        let key = parsed.path().trim_start_matches('/').to_string();
        Ok(Self {
            provider,
            bucket,
            key,
        })
    }

    pub fn to_uri(&self) -> String {
        format!(
            "{}://{}/{}",
            self.provider.default_scheme(),
            self.bucket,
            self.key
        )
    }

    pub fn join(&self, child: &str) -> Self {
        let key = if self.key.is_empty() || self.key.ends_with('/') {
            format!("{}{}", self.key, child)
        } else {
            format!("{}/{}", self.key, child)
        };
        Self {
            key,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDataSet {
    pub location: RemoteLocation,
    pub file_format: FileFormat,
    pub partition_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FileFormat {
    #[default]
    Parquet,
    Csv,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_uri() {
        let loc = RemoteLocation::from_uri("s3://my-bucket/path/to/file.parquet").unwrap();
        assert!(matches!(loc.provider, CloudProvider::Aws));
        assert_eq!(loc.bucket, "my-bucket");
        assert_eq!(loc.key, "path/to/file.parquet");
    }

    #[test]
    fn test_parse_gcs_uri() {
        let loc = RemoteLocation::from_uri("gs://my-bucket/data/file.parquet").unwrap();
        assert!(matches!(loc.provider, CloudProvider::Gcp));
        assert_eq!(loc.bucket, "my-bucket");
        assert_eq!(loc.key, "data/file.parquet");
    }

    #[test]
    fn test_parse_azure_uri() {
        let loc = RemoteLocation::from_uri("az://container/path/file.parquet").unwrap();
        assert!(matches!(loc.provider, CloudProvider::Azure));
        assert_eq!(loc.bucket, "container");
        assert_eq!(loc.key, "path/file.parquet");
    }

    #[test]
    fn test_unsupported_scheme() {
        let result = RemoteLocation::from_uri("hdfs://bucket/file.parquet");
        assert!(result.is_err());
    }

    #[test]
    fn test_join_paths() {
        let loc = RemoteLocation::from_uri("s3://bucket/base/").unwrap();
        let child = loc.join("nested/file.parquet");
        assert_eq!(child.key, "base/nested/file.parquet");

        let loc2 = RemoteLocation::from_uri("s3://bucket").unwrap();
        let child2 = loc2.join("file.parquet");
        assert_eq!(child2.key, "file.parquet");
    }

    #[test]
    fn test_to_uri() {
        let loc = RemoteLocation::from_uri("s3://bucket/path/file.parquet").unwrap();
        assert_eq!(loc.to_uri(), "s3://bucket/path/file.parquet");
    }

    #[test]
    fn test_cloud_provider_from_scheme() {
        assert!(matches!(
            CloudProvider::from_scheme("s3"),
            Some(CloudProvider::Aws)
        ));
        assert!(matches!(
            CloudProvider::from_scheme("gs"),
            Some(CloudProvider::Gcp)
        ));
        assert!(matches!(
            CloudProvider::from_scheme("az"),
            Some(CloudProvider::Azure)
        ));
        assert!(matches!(
            CloudProvider::from_scheme("abfs"),
            Some(CloudProvider::Azure)
        ));
        assert!(CloudProvider::from_scheme("http").is_none());
    }
}
