use sqlx::PgPool;
use sqlx::SqlitePool;

/// Run all pending migrations to initialize the registry SQLite schema.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            dirty INTEGER NOT NULL DEFAULT 0,
            timestamp TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    let current_version: Option<i64> =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM migrations")
            .fetch_one(pool)
            .await?;

    let current = current_version.unwrap_or(0);

    if current < 1 {
        apply_v1(pool).await?;
    }

    if current < 2 {
        apply_v2(pool).await?;
    }

    Ok(())
}

async fn apply_v1(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_registries (
            project_name TEXT PRIMARY KEY NOT NULL,
            serialized_registry BLOB NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            last_updated TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO migrations (version, dirty) VALUES (1, 0)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn apply_v2(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS backfill_jobs (
            id TEXT PRIMARY KEY NOT NULL,
            feature_view_name TEXT NOT NULL,
            project TEXT NOT NULL,
            start_ts TEXT NOT NULL,
            end_ts TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Pending',
            progress REAL NOT NULL DEFAULT 0.0,
            chunk_size_seconds INTEGER NOT NULL DEFAULT 86400,
            error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO migrations (version, dirty) VALUES (2, 0)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Run all pending PostgreSQL migrations.
pub async fn run_migrations_pg(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            dirty INTEGER NOT NULL DEFAULT 0,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    let current_version: Option<i64> =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM migrations")
            .fetch_one(pool)
            .await?;

    let current = current_version.unwrap_or(0);

    if current < 1 {
        apply_v1_pg(pool).await?;
    }

    if current < 2 {
        apply_v2_pg(pool).await?;
    }

    Ok(())
}

async fn apply_v1_pg(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_registries (
            project_name VARCHAR(255) PRIMARY KEY NOT NULL,
            serialized_registry BYTEA NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO migrations (version, dirty) VALUES (1, 0)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn apply_v2_pg(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS backfill_jobs (
            id TEXT PRIMARY KEY NOT NULL,
            feature_view_name TEXT NOT NULL,
            project TEXT NOT NULL,
            start_ts TIMESTAMPTZ NOT NULL,
            end_ts TIMESTAMPTZ NOT NULL,
            status TEXT NOT NULL DEFAULT 'Pending',
            progress DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            chunk_size_seconds BIGINT NOT NULL DEFAULT 86400,
            error TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO migrations (version, dirty) VALUES (2, 0)")
        .execute(pool)
        .await?;

    Ok(())
}
