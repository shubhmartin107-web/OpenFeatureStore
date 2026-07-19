use std::sync::Arc;

use futures_util::StreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::azure::MicrosoftAzureBuilder;
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::{ObjectStore, path::Path};
use tracing::info;

use crate::error::{RemoteResult, RemoteStoreError};
use crate::types::{CloudProvider, RemoteFileInfo, RemoteLocation};

#[derive(Clone)]
pub struct RemoteBackend {
    pub provider: CloudProvider,
    pub store: Arc<dyn ObjectStore>,
}

impl RemoteBackend {
    pub fn from_location(
        location: &RemoteLocation,
        config: &crate::RemoteStoreConfig,
    ) -> RemoteResult<Self> {
        match location.provider {
            CloudProvider::Aws => Self::create_s3(location, config),
            CloudProvider::Gcp => Self::create_gcs(location, config),
            CloudProvider::Azure => Self::create_azure(location, config),
        }
    }

    fn create_s3(
        location: &RemoteLocation,
        config: &crate::RemoteStoreConfig,
    ) -> RemoteResult<Self> {
        let region = config
            .s3
            .as_ref()
            .and_then(|s| s.region.clone())
            .unwrap_or_else(|| "us-east-1".into());

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&location.bucket)
            .with_region(&region);

        if let Some(ref s3) = config.s3 {
            if let Some(ref key_id) = s3.access_key_id {
                builder = builder.with_access_key_id(key_id);
            }
            if let Some(ref secret) = s3.secret_access_key {
                builder = builder.with_secret_access_key(secret);
            }
            if let Some(ref endpoint) = s3.endpoint {
                builder = builder.with_endpoint(endpoint);
            }
        }

        let store = Arc::new(builder.build().map_err(|e| {
            RemoteStoreError::MissingCredential(format!("failed to build S3 client: {e}"))
        })?);

        info!(bucket = %location.bucket, "initialized S3 remote store backend");
        Ok(Self {
            provider: CloudProvider::Aws,
            store,
        })
    }

    fn create_gcs(
        location: &RemoteLocation,
        config: &crate::RemoteStoreConfig,
    ) -> RemoteResult<Self> {
        let mut builder = GoogleCloudStorageBuilder::new().with_bucket_name(&location.bucket);

        if let Some(ref gcs) = config.gcs
            && let Some(ref sa_path) = gcs.service_account_path
        {
            builder = builder.with_service_account_path(sa_path);
        }

        let store = Arc::new(builder.build().map_err(|e| {
            RemoteStoreError::MissingCredential(format!("failed to build GCS client: {e}"))
        })?);

        info!(bucket = %location.bucket, "initialized GCS remote store backend");
        Ok(Self {
            provider: CloudProvider::Gcp,
            store,
        })
    }

    fn create_azure(
        location: &RemoteLocation,
        config: &crate::RemoteStoreConfig,
    ) -> RemoteResult<Self> {
        let builder = MicrosoftAzureBuilder::new().with_container_name(&location.bucket);

        if let Some(ref azure) = config.azure {
            // Azure connection string is not directly supported by object_store.
            // Use individual config keys. In practice, this reads from env vars
            // like AZURE_STORAGE_ACCOUNT_NAME and AZURE_STORAGE_ACCESS_KEY.
            if let Some(ref _conn_str) = azure.connection_string {
                tracing::warn!(
                    "Azure connection_string config field is set but not directly used; \
                    set AZURE_STORAGE_ACCOUNT_NAME and AZURE_STORAGE_ACCESS_KEY env vars instead"
                );
            }
        }

        let store = Arc::new(builder.build().map_err(|e| {
            RemoteStoreError::MissingCredential(format!("failed to build Azure client: {e}"))
        })?);

        info!(container = %location.bucket, "initialized Azure remote store backend");
        Ok(Self {
            provider: CloudProvider::Azure,
            store,
        })
    }

    pub async fn list_files(&self, prefix: &str) -> RemoteResult<Vec<RemoteFileInfo>> {
        let path = Path::from(prefix);
        let mut files = Vec::new();

        let mut result = self.store.list(Some(&path));
        while let Some(meta) = result.next().await {
            let meta = meta?;
            files.push(RemoteFileInfo {
                location: RemoteLocation {
                    provider: self.provider.clone(),
                    bucket: String::new(),
                    key: meta.location.to_string(),
                },
                size: meta.size as u64,
                last_modified: meta.last_modified,
                path: meta.location.to_string(),
            });
        }

        Ok(files)
    }

    pub async fn head(&self, path: &str) -> RemoteResult<object_store::ObjectMeta> {
        let meta = self.store.head(&Path::from(path)).await?;
        Ok(meta)
    }

    pub async fn file_exists(&self, path: &str) -> RemoteResult<bool> {
        match self.store.head(&Path::from(path)).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn download_to_temp(&self, remote_path: &str) -> RemoteResult<std::path::PathBuf> {
        use tokio::io::AsyncWriteExt;

        let path = Path::from(remote_path);
        let data = self.store.get(&path).await?.bytes().await?;

        let file_name = remote_path.split('/').next_back().unwrap_or("remote_file");
        let temp_dir = std::env::temp_dir().join("ofs-remote-cache");
        tokio::fs::create_dir_all(&temp_dir).await?;

        let local_path = temp_dir.join(format!(
            "{}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            file_name
        ));
        let mut file = tokio::fs::File::create(&local_path).await?;
        file.write_all(&data).await?;
        file.flush().await?;

        Ok(local_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RemoteStoreConfig;

    #[tokio::test]
    async fn test_backend_location_creation() {
        let config = RemoteStoreConfig::default();
        let loc = RemoteLocation {
            provider: CloudProvider::Aws,
            bucket: "test-bucket".into(),
            key: String::new(),
        };
        // In a non-AWS environment with no credentials configured,
        // S3 building may succeed or fail depending on the environment.
        // We just verify the method doesn't panic.
        let _ = RemoteBackend::from_location(&loc, &config);
    }
}
