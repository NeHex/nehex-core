use std::{
    collections::HashSet,
    process::{Command, Stdio},
    time::Duration,
};

use sqlx::{Connection, PgConnection, PgPool, postgres::PgPoolOptions};
use tracing::{info, warn};

use crate::{
    config::Settings,
    error::{AppError, AppResult},
    state::ProjectPaths,
};

pub fn check_admin_secret_safety(settings: &Settings) -> AppResult<()> {
    if settings.admin_api_secret.trim() == "please-change-me" && !settings.is_dev_env() {
        return Err(AppError::internal(
            "Unsafe default ADMIN_API_SECRET is not allowed outside development environments.",
        ));
    }
    Ok(())
}

pub fn ensure_admin_frontend(settings: &Settings, paths: &ProjectPaths) -> AppResult<()> {
    if settings.admin_manager_build_on_startup {
        build_admin_frontend(paths)?;
    }

    if !paths.admin_index_file.exists() {
        return Err(AppError::internal(
            "Admin manager frontend dist not found. Set ADMIN_MANAGER_BUILD_ON_STARTUP=true or build app/nehex-admin first.",
        ));
    }

    Ok(())
}

pub async fn wait_for_database_ready(settings: &Settings) -> AppResult<()> {
    let max_retries = settings.db_startup_max_retries.max(1);
    let retry_interval = settings.db_startup_retry_interval_seconds.max(1);
    let connect_timeout = Duration::from_secs(settings.db_connect_timeout_seconds.max(1));
    let database_url = settings.database_url();

    if !database_url.starts_with("postgresql://") && !database_url.starts_with("postgres://") {
        return Err(AppError::internal(
            "Unsupported database backend. Only PostgreSQL is supported.",
        ));
    }

    let mut last_error = String::new();
    for attempt in 1..=max_retries {
        match tokio::time::timeout(connect_timeout, PgConnection::connect(&database_url)).await {
            Ok(result) => match result {
                Ok(conn) => {
                    let _ = conn.close().await;
                    if attempt > 1 {
                        info!("[startup] database ready after retry {attempt}/{max_retries}");
                    }
                    return Ok(());
                }
                Err(error) => {
                    last_error = error.to_string();
                    if attempt < max_retries {
                        warn!(
                            "[startup] database not ready ({attempt}/{max_retries}) {}:{}, retry in {retry_interval}s: {last_error}",
                            settings.db_host, settings.db_port,
                        );
                        tokio::time::sleep(Duration::from_secs(retry_interval)).await;
                    }
                }
            },
            Err(_) => {
                last_error = format!(
                    "connection timeout after {}s",
                    connect_timeout.as_secs().max(1)
                );
                if attempt < max_retries {
                    warn!(
                        "[startup] database not ready ({attempt}/{max_retries}) {}:{}, retry in {retry_interval}s: {last_error}",
                        settings.db_host, settings.db_port,
                    );
                    tokio::time::sleep(Duration::from_secs(retry_interval)).await;
                }
            }
        }
    }

    Err(AppError::internal(format!(
        "[startup] database unavailable after retries ({max_retries} attempts, host={}, port={}): {last_error}",
        settings.db_host, settings.db_port,
    )))
}

pub async fn create_db_pool(settings: &Settings) -> AppResult<PgPool> {
    let max_connections = settings
        .db_pool_size
        .saturating_add(settings.db_max_overflow)
        .max(1);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(settings.db_pool_timeout_seconds.max(1)))
        .connect(&settings.database_url())
        .await
        .map_err(|error| AppError::internal(format!("Failed to create database pool: {error}")))?;

    Ok(pool)
}

pub async fn apply_startup_schema_maintenance(settings: &Settings, pool: &PgPool) {
    if let Err(error) = ensure_content_sync_tables(pool).await {
        warn!("[startup] skip ensure_content_sync_tables: {error}");
    }

    if let Err(error) = ensure_schema_compatibility_boolean_flags(pool).await {
        warn!("[startup] skip ensure_schema_compatibility_boolean_flags: {error}");
    }
    if let Err(error) = ensure_schema_compatibility_numeric_types(pool).await {
        warn!("[startup] skip ensure_schema_compatibility_numeric_types: {error}");
    }

    if settings.db_auto_create_tables {
        if let Err(error) = ensure_core_content_tables(pool).await {
            warn!("[startup] skip ensure_core_content_tables: {error}");
        }
        if let Err(error) = ensure_system_tables(pool).await {
            warn!("[startup] skip ensure_system_tables: {error}");
        }
        if let Err(error) = ensure_schema_compatibility_columns(pool).await {
            warn!("[startup] skip ensure_schema_compatibility_columns: {error}");
        }
        if let Err(error) = ensure_performance_indexes(pool).await {
            warn!("[startup] skip ensure_performance_indexes: {error}");
        }
    } else {
        info!("[startup] skip schema DDL/index auto-create (DB_AUTO_CREATE_TABLES=false)");
    }
}

pub async fn ensure_installation_schema_bootstrap(pool: &PgPool) -> AppResult<()> {
    ensure_core_content_tables(pool).await?;
    ensure_system_tables(pool).await?;
    ensure_content_sync_tables(pool).await?;
    ensure_schema_compatibility_columns(pool).await?;
    ensure_schema_compatibility_boolean_flags(pool).await?;
    ensure_schema_compatibility_numeric_types(pool).await?;
    ensure_performance_indexes(pool).await?;
    Ok(())
}

async fn ensure_content_sync_tables(pool: &PgPool) -> AppResult<()> {
    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS content_change_log (
            seq BIGSERIAL PRIMARY KEY,
            event_type VARCHAR(40) NOT NULL DEFAULT 'content.updated',
            resource VARCHAR(40) NOT NULL,
            action VARCHAR(20) NOT NULL,
            ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "content_change_log",
    )
    .await?;

    run_ddl(
        pool,
        "CREATE INDEX IF NOT EXISTS idx_content_change_log_resource_seq ON content_change_log (resource, seq)",
        "idx_content_change_log_resource_seq",
    )
    .await?;

    Ok(())
}

async fn ensure_core_content_tables(pool: &PgPool) -> AppResult<()> {
    ensure_enum_type(
        pool,
        "setting_type",
        &["string", "int", "float", "boolean", "json"],
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            setting_key VARCHAR(100) PRIMARY KEY,
            setting_type setting_type NOT NULL DEFAULT 'string',
            setting_content TEXT,
            description VARCHAR(255),
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "settings",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS article (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            "articleTopImage" VARCHAR(500),
            class VARCHAR(100) NOT NULL,
            read BIGINT NOT NULL DEFAULT 0,
            like_count BIGINT NOT NULL DEFAULT 0,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            "lastEditTime" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            tag VARCHAR(255),
            top BIGINT NOT NULL DEFAULT 0,
            status BIGINT NOT NULL DEFAULT 1,
            content TEXT
        )
        "#,
        "article",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS singlepage (
            id BIGSERIAL PRIMARY KEY,
            page_key VARCHAR(120) NOT NULL UNIQUE,
            title VARCHAR(255) NOT NULL,
            cover_image VARCHAR(500),
            content TEXT,
            sort BIGINT NOT NULL DEFAULT 0,
            status BIGINT NOT NULL DEFAULT 1,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "singlepage",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS daily (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            content TEXT,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            weather VARCHAR(50),
            daily_type VARCHAR(20) NOT NULL DEFAULT 'note',
            kuma_movie_id BIGINT
        )
        "#,
        "daily",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS project (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            cover VARCHAR(500),
            category VARCHAR(120),
            description VARCHAR(1000),
            content TEXT,
            tech_stack VARCHAR(500),
            project_url VARCHAR(1000),
            github_url VARCHAR(1000),
            sort BIGINT NOT NULL DEFAULT 0,
            status BIGINT NOT NULL DEFAULT 1,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "project",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS album (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            cover VARCHAR(500),
            class VARCHAR(100) NOT NULL,
            like_count BIGINT NOT NULL DEFAULT 0,
            img_urls TEXT,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "album",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS friends (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            description VARCHAR(500),
            category VARCHAR(100) NOT NULL,
            favicon VARCHAR(500),
            url VARCHAR(500) NOT NULL UNIQUE,
            status VARCHAR(20) NOT NULL DEFAULT 'ok',
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "friends",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS comment (
            id BIGSERIAL PRIMARY KEY,
            parent_id BIGINT NOT NULL DEFAULT 0,
            target_type VARCHAR(20) NOT NULL,
            target_id BIGINT NOT NULL,
            content TEXT NOT NULL,
            nickname VARCHAR(100) NOT NULL,
            email VARCHAR(255),
            website VARCHAR(255),
            like_count BIGINT NOT NULL DEFAULT 0,
            status BIGINT NOT NULL DEFAULT 1,
            is_admin BIGINT NOT NULL DEFAULT 0,
            ip VARCHAR(50),
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "comment",
    )
    .await?;

    Ok(())
}

async fn ensure_system_tables(pool: &PgPool) -> AppResult<()> {
    ensure_enum_type(
        pool,
        "friend_apply_status",
        &["pending", "approved", "rejected", "blocked"],
    )
    .await?;
    ensure_enum_type(pool, "mail_log_status", &["success", "failed"]).await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS friend_apply (
            id BIGSERIAL PRIMARY KEY,
            site_title VARCHAR(255) NOT NULL,
            site_url VARCHAR(500) NOT NULL,
            site_description VARCHAR(1000),
            site_icon VARCHAR(500),
            contact VARCHAR(255),
            status friend_apply_status NOT NULL DEFAULT 'pending',
            ip VARCHAR(50),
            user_agent VARCHAR(255),
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "friend_apply",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS mail_log (
            id BIGSERIAL PRIMARY KEY,
            category VARCHAR(40) NOT NULL DEFAULT 'unknown',
            template_key VARCHAR(40) NOT NULL DEFAULT 'custom',
            to_email VARCHAR(255) NOT NULL,
            subject VARCHAR(255) NOT NULL,
            body TEXT NOT NULL,
            status mail_log_status NOT NULL DEFAULT 'success',
            error_message TEXT,
            trigger_comment_id BIGINT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            sent_at TIMESTAMP
        )
        "#,
        "mail_log",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS media_folder (
            id BIGSERIAL PRIMARY KEY,
            name VARCHAR(120) NOT NULL UNIQUE,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "media_folder",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS media_image (
            id BIGSERIAL PRIMARY KEY,
            folder_id BIGINT REFERENCES media_folder(id) ON DELETE SET NULL,
            provider VARCHAR(40) NOT NULL DEFAULT 'local',
            storage_key VARCHAR(800) NOT NULL,
            url VARCHAR(1200) NOT NULL,
            file_name VARCHAR(255),
            content_type VARCHAR(120),
            size_bytes BIGINT NOT NULL DEFAULT 0,
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "media_image",
    )
    .await?;

    run_ddl(
        pool,
        r#"
        CREATE TABLE IF NOT EXISTS kuma_movie (
            id BIGSERIAL PRIMARY KEY,
            provider VARCHAR(20) NOT NULL,
            movie_id VARCHAR(120) NOT NULL,
            watch_status VARCHAR(20) NOT NULL DEFAULT 'want',
            cover VARCHAR(1200),
            title VARCHAR(500) NOT NULL,
            years VARCHAR(120),
            score VARCHAR(60),
            description TEXT,
            source_url VARCHAR(1200),
            create_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "kuma_movie",
    )
    .await?;

    Ok(())
}

async fn ensure_schema_compatibility_columns(pool: &PgPool) -> AppResult<()> {
    let tables = list_public_tables(pool).await?;

    if tables.contains("article") {
        run_ddl(
            pool,
            "ALTER TABLE article ADD COLUMN IF NOT EXISTS like_count BIGINT NOT NULL DEFAULT 0",
            "article.like_count",
        )
        .await?;
        run_ddl(
            pool,
            "ALTER TABLE article ADD COLUMN IF NOT EXISTS status BIGINT NOT NULL DEFAULT 1",
            "article.status",
        )
        .await?;
        run_ddl(
            pool,
            "ALTER TABLE article ADD COLUMN IF NOT EXISTS create_time TIMESTAMP",
            "article.create_time",
        )
        .await?;
        run_ddl(
            pool,
            r#"UPDATE article SET create_time = "lastEditTime" WHERE create_time IS NULL"#,
            "article.create_time.backfill",
        )
        .await?;
        run_ddl(
            pool,
            "ALTER TABLE article ALTER COLUMN create_time SET DEFAULT CURRENT_TIMESTAMP",
            "article.create_time.default",
        )
        .await?;
        run_ddl(
            pool,
            "ALTER TABLE article ALTER COLUMN create_time SET NOT NULL",
            "article.create_time.not_null",
        )
        .await?;
    }

    if tables.contains("comment") {
        run_ddl(
            pool,
            "ALTER TABLE comment ADD COLUMN IF NOT EXISTS is_admin BIGINT NOT NULL DEFAULT 0",
            "comment.is_admin",
        )
        .await?;
    }

    if tables.contains("daily") {
        run_ddl(
            pool,
            "ALTER TABLE daily ADD COLUMN IF NOT EXISTS daily_type VARCHAR(20) NOT NULL DEFAULT 'note'",
            "daily.daily_type",
        )
        .await?;
        run_ddl(
            pool,
            "ALTER TABLE daily ADD COLUMN IF NOT EXISTS kuma_movie_id BIGINT",
            "daily.kuma_movie_id",
        )
        .await?;
    }

    if tables.contains("kuma_movie") {
        run_ddl(
            pool,
            "ALTER TABLE kuma_movie ADD COLUMN IF NOT EXISTS watch_status VARCHAR(20) NOT NULL DEFAULT 'want'",
            "kuma_movie.watch_status",
        )
        .await?;
    }

    Ok(())
}

async fn ensure_schema_compatibility_numeric_types(pool: &PgPool) -> AppResult<()> {
    let integer_columns = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = 'public' AND data_type = 'integer'
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        AppError::internal(format!(
            "Failed to inspect integer columns for compatibility migration: {error}"
        ))
    })?;

    let integer_column_set = integer_columns
        .into_iter()
        .collect::<HashSet<(String, String)>>();

    let upgrade_targets = [
        ("article", "id"),
        ("article", "read"),
        ("article", "like_count"),
        ("article", "top"),
        ("article", "status"),
        ("daily", "id"),
        ("album", "id"),
        ("album", "like_count"),
        ("singlepage", "id"),
        ("singlepage", "sort"),
        ("singlepage", "status"),
        ("project", "id"),
        ("project", "sort"),
        ("project", "status"),
        ("friends", "id"),
        ("friend_apply", "id"),
        ("comment", "id"),
        ("comment", "parent_id"),
        ("comment", "target_id"),
        ("comment", "like_count"),
        ("comment", "status"),
        ("comment", "is_admin"),
        ("mail_log", "id"),
        ("mail_log", "trigger_comment_id"),
        ("media_image", "folder_id"),
        ("media_image", "id"),
        ("media_image", "size_bytes"),
        ("media_folder", "id"),
        ("kuma_movie", "id"),
    ];

    for (table_name, column_name) in upgrade_targets {
        if !integer_column_set.contains(&(table_name.to_string(), column_name.to_string())) {
            continue;
        }

        let statement = format!(
            "ALTER TABLE {table_name} ALTER COLUMN {column_name} TYPE BIGINT USING {column_name}::BIGINT",
        );
        let label = format!("{table_name}.{column_name}:integer_to_bigint");
        if let Err(error) = run_ddl(pool, &statement, &label).await {
            warn!("[startup] skip numeric type migration for {table_name}.{column_name}: {error}");
        }
    }

    Ok(())
}

async fn ensure_schema_compatibility_boolean_flags(pool: &PgPool) -> AppResult<()> {
    let boolean_columns = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = 'public' AND data_type = 'boolean'
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|error| {
        AppError::internal(format!(
            "Failed to inspect boolean columns for compatibility migration: {error}"
        ))
    })?;

    let boolean_column_set = boolean_columns
        .into_iter()
        .collect::<HashSet<(String, String)>>();

    let upgrade_targets = [
        ("article", "top"),
        ("article", "status"),
        ("singlepage", "status"),
        ("project", "status"),
        ("comment", "status"),
        ("comment", "is_admin"),
    ];

    for (table_name, column_name) in upgrade_targets {
        if !boolean_column_set.contains(&(table_name.to_string(), column_name.to_string())) {
            continue;
        }

        let statement = format!(
            "ALTER TABLE {table_name} ALTER COLUMN {column_name} TYPE BIGINT USING (CASE WHEN {column_name} THEN 1 ELSE 0 END)",
        );
        let label = format!("{table_name}.{column_name}:boolean_to_bigint");
        if let Err(error) = run_ddl(pool, &statement, &label).await {
            warn!("[startup] skip boolean flag migration for {table_name}.{column_name}: {error}");
        }
    }

    Ok(())
}

async fn ensure_performance_indexes(pool: &PgPool) -> AppResult<()> {
    let tables = list_public_tables(pool).await?;
    let specs = [
        (
            "comment",
            "idx_comment_target_status_time",
            "target_type,target_id,status,create_time,id",
            false,
        ),
        ("comment", "idx_comment_ip_time", "ip,create_time", false),
        ("album", "idx_album_update_time", "update_time,id", false),
        ("daily", "idx_daily_create_time", "create_time,id", false),
        (
            "singlepage",
            "idx_singlepage_status_sort",
            "status,sort,id",
            false,
        ),
        (
            "project",
            "idx_project_status_sort",
            "status,sort,id",
            false,
        ),
        (
            "friends",
            "idx_friends_status_time",
            "status,create_time,id",
            false,
        ),
        (
            "friend_apply",
            "idx_friend_apply_status_time",
            "status,create_time,id",
            false,
        ),
        (
            "friend_apply",
            "idx_friend_apply_url_time",
            "site_url,create_time,id",
            false,
        ),
        ("friends", "uq_friends_url", "url", true),
        (
            "mail_log",
            "idx_mail_log_status_time",
            "status,created_at,id",
            false,
        ),
        (
            "mail_log",
            "idx_mail_log_comment",
            "trigger_comment_id,created_at,id",
            false,
        ),
        (
            "media_folder",
            "idx_media_folder_create_time",
            "create_time,id",
            false,
        ),
        (
            "media_image",
            "idx_media_image_folder_time",
            "folder_id,create_time,id",
            false,
        ),
        (
            "kuma_movie",
            "idx_kuma_movie_create_time",
            "create_time,id",
            false,
        ),
        (
            "kuma_movie",
            "uq_kuma_movie_provider_movie_id",
            "provider,movie_id",
            true,
        ),
    ];

    for (table, index_name, columns_sql, is_unique) in specs {
        if !tables.contains(table) {
            continue;
        }
        let prefix = if is_unique {
            "CREATE UNIQUE INDEX"
        } else {
            "CREATE INDEX"
        };
        let statement = format!("{prefix} IF NOT EXISTS {index_name} ON {table} ({columns_sql})");
        if let Err(error) = sqlx::query(&statement).execute(pool).await {
            warn!("[startup] skip index {index_name} on {table}: {error}");
        }
    }

    Ok(())
}

async fn ensure_enum_type(pool: &PgPool, type_name: &str, values: &[&str]) -> AppResult<()> {
    let formatted_values = values
        .iter()
        .map(|value| format!("'{}'", value.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");

    let statement = format!(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = '{type_name}') THEN
                CREATE TYPE {type_name} AS ENUM ({formatted_values});
            END IF;
        END
        $$;
        "#
    );
    run_ddl(pool, &statement, type_name).await
}

async fn run_ddl(pool: &PgPool, statement: &str, label: &str) -> AppResult<()> {
    sqlx::query(statement)
        .execute(pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to apply ddl `{label}`: {error}")))?;
    Ok(())
}

async fn list_public_tables(pool: &PgPool) -> AppResult<HashSet<String>> {
    let table_names = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| AppError::internal(format!("Failed to list database tables: {error}")))?;

    Ok(table_names.into_iter().collect())
}

fn build_admin_frontend(paths: &ProjectPaths) -> AppResult<()> {
    let npm_available = Command::new("npm")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !npm_available {
        if paths.admin_index_file.exists() {
            warn!(
                "[startup] `npm` not found in PATH, skip admin frontend build and use prebuilt dist files."
            );
            return Ok(());
        }

        return Err(AppError::internal(
            "Admin frontend build failed: `npm` not found in PATH and no prebuilt admin dist found.",
        ));
    }

    run_startup_command(&["npm", "install"], &paths.admin_project_dir)?;
    run_startup_command(&["npm", "run", "build"], &paths.admin_project_dir)?;
    Ok(())
}

fn run_startup_command(command: &[&str], cwd: &std::path::Path) -> AppResult<()> {
    let command_text = command.join(" ");
    info!("[startup] running `{}` in {}", command_text, cwd.display());

    let mut cmd = Command::new(command[0]);
    cmd.args(&command[1..]);
    let status = cmd.current_dir(cwd).status().map_err(|error| {
        AppError::internal(format!(
            "Failed to run startup command `{command_text}`: {error}"
        ))
    })?;

    if !status.success() {
        return Err(AppError::internal(format!(
            "Admin frontend build failed: `{command_text}` exited with code {:?}.",
            status.code()
        )));
    }

    Ok(())
}
