use std::sync::Arc;

use ofs_config::OpenFeatureStoreConfig;
use ofs_core::traits::{OfflineStore, OnlineStore, Registry};
use ofs_observability::init_logging;
use ofs_serving::FeatureServer;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("info", "json", false);

    // Load config from default locations
    let config = OpenFeatureStoreConfig::from_source(&ofs_config::source::ConfigSource::default())
        .unwrap_or_else(|e| {
            tracing::warn!("no config file found ({}), using defaults", e);
            OpenFeatureStoreConfig::default()
        });

    config.validate()?;
    tracing::info!("config loaded: version={}", config.version);

    // Build stores
    let registry = build_registry(&config).await?;
    let online_store = build_online_store(&config).await?;
    let offline_store = build_offline_store(&config)?;

    // Build the server and wire stores
    let mut server = FeatureServer::new(config);
    server.with_stores(
        registry.clone(),
        online_store.clone(),
        offline_store.clone(),
    );

    // Start lifecycle background task
    server.start_lifecycle();

    tracing::info!("starting OpenFeatureStore server");
    server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("server error: {}", e))?;

    Ok(())
}

async fn build_registry(config: &OpenFeatureStoreConfig) -> anyhow::Result<Arc<dyn Registry>> {
    let rc = config.registry.clone().unwrap_or_default();
    match rc.backend.as_str() {
        "sqlite" => {
            let pool = SqlitePool::connect(&rc.path)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to SQLite registry: {}", e))?;
            let reg = ofs_registry::SqlRegistry::new(pool)
                .await
                .map_err(|e| anyhow::anyhow!("failed to init registry: {}", e))?;
            Ok(Arc::new(reg))
        }
        "postgres" => {
            let conn_str = rc
                .postgres
                .and_then(|p| p.connection_string)
                .unwrap_or_else(|| "postgres://localhost:5432/ofs".to_string());
            let pool = sqlx::PgPool::connect(&conn_str)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to PostgreSQL registry: {}", e))?;
            let reg = ofs_registry::PgRegistry::new(pool)
                .await
                .map_err(|e| anyhow::anyhow!("failed to init PgRegistry: {}", e))?;
            Ok(Arc::new(reg))
        }
        other => anyhow::bail!("unsupported registry backend: {}", other),
    }
}

async fn build_online_store(
    config: &OpenFeatureStoreConfig,
) -> anyhow::Result<Arc<dyn OnlineStore>> {
    let oc = config.online_store.clone().unwrap_or_default();
    match oc.backend.as_str() {
        "sqlite" => {
            let pool = SqlitePool::connect(&oc.path)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to SQLite online store: {}", e))?;
            let store = ofs_online_store::SqliteOnlineStore::new(pool);
            Ok(Arc::new(store))
        }
        "redis" => {
            let conn_str = oc
                .redis
                .as_ref()
                .and_then(|r| r.connection_string.clone())
                .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
            let store = ofs_online_store::RedisOnlineStore::new(&conn_str)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to Redis: {}", e))?;
            Ok(Arc::new(store))
        }
        other => anyhow::bail!("unsupported online store backend: {}", other),
    }
}

fn build_offline_store(_config: &OpenFeatureStoreConfig) -> anyhow::Result<Arc<dyn OfflineStore>> {
    let store = ofs_offline_store::DuckDbOfflineStore;
    Ok(Arc::new(store))
}
