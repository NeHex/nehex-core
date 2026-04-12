use std::env;

#[derive(Clone, Debug)]
pub struct Settings {
    pub app_name: String,
    pub app_version: String,
    pub app_env: String,
    pub app_port: u16,
    pub cors_allow_origins: Vec<String>,
    pub cors_allow_credentials: bool,
    pub admin_manager_web: String,
    pub admin_api_secret: String,
    pub admin_api_client_id: String,
    pub admin_api_token_ttl_seconds: i64,
    pub admin_cookie_domain: String,
    pub admin_public_cookie_domain: String,
    pub admin_manager_build_on_startup: bool,
    pub simple_cache_max_entries: usize,
    pub redis_enabled: bool,
    pub redis_url: String,
    pub redis_cache_prefix: String,
    pub redis_connect_retry_seconds: u64,
    pub redis_socket_connect_timeout: f64,
    pub redis_socket_timeout: f64,

    pub db_url: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,

    pub db_pool_size: u32,
    pub db_max_overflow: u32,
    pub db_pool_timeout_seconds: u64,
    pub db_connect_timeout_seconds: u64,
    pub db_auto_create_tables: bool,
    pub db_startup_max_retries: u32,
    pub db_startup_retry_interval_seconds: u64,
}

impl Settings {
    pub fn load() -> Self {
        let cors_allow_origins_raw = env_string(
            "CORS_ALLOW_ORIGINS",
            "http://127.0.0.1:3000,http://localhost:3000",
        );

        let cors_allow_origins = cors_allow_origins_raw
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();

        Self {
            app_name: env_string("APP_NAME", "NeHex Core API"),
            app_version: env_string("APP_VERSION", "1.2.0"),
            app_env: env_string("APP_ENV", "dev"),
            app_port: env_u16("APP_PORT", 7878),
            cors_allow_origins,
            cors_allow_credentials: env_bool("CORS_ALLOW_CREDENTIALS", true),
            admin_manager_web: normalize_admin_manager_web_path(
                Some(&env_string("ADMIN_MANAGER_WEB", "/nehex-admin")),
                "/nehex-admin",
            ),
            admin_api_secret: env_string("ADMIN_API_SECRET", "please-change-me"),
            admin_api_client_id: env_string("ADMIN_API_CLIENT_ID", "nehex-vuetify-admin"),
            admin_api_token_ttl_seconds: env_i64("ADMIN_API_TOKEN_TTL_SECONDS", 43200),
            admin_cookie_domain: env_string("ADMIN_COOKIE_DOMAIN", ""),
            admin_public_cookie_domain: env_string("ADMIN_PUBLIC_COOKIE_DOMAIN", ""),
            admin_manager_build_on_startup: env_bool("ADMIN_MANAGER_BUILD_ON_STARTUP", true),
            simple_cache_max_entries: env_usize("SIMPLE_CACHE_MAX_ENTRIES", 1024),
            redis_enabled: env_bool("REDIS_ENABLED", true),
            redis_url: env_string("REDIS_URL", "redis://127.0.0.1:6379/0"),
            redis_cache_prefix: env_string("REDIS_CACHE_PREFIX", "nehex:cache:"),
            redis_connect_retry_seconds: env_u64("REDIS_CONNECT_RETRY_SECONDS", 30),
            redis_socket_connect_timeout: env_f64("REDIS_SOCKET_CONNECT_TIMEOUT", 1.0),
            redis_socket_timeout: env_f64("REDIS_SOCKET_TIMEOUT", 1.5),

            db_url: env_string("DB_URL", ""),
            db_host: env_string("DB_HOST", "127.0.0.1"),
            db_port: env_u16("DB_PORT", 5432),
            db_name: env_string("DB_NAME", "nehex_dtbs"),
            db_user: env_string("DB_USER", "nehex_dtbs"),
            db_password: env_string("DB_PASSWORD", ""),

            db_pool_size: env_u32("DB_POOL_SIZE", 10),
            db_max_overflow: env_u32("DB_MAX_OVERFLOW", 20),
            db_pool_timeout_seconds: env_u64("DB_POOL_TIMEOUT", 30),
            db_connect_timeout_seconds: env_u64("DB_CONNECT_TIMEOUT", 5),
            db_auto_create_tables: env_bool("DB_AUTO_CREATE_TABLES", false),
            db_startup_max_retries: env_u32("DB_STARTUP_MAX_RETRIES", 30),
            db_startup_retry_interval_seconds: env_u64("DB_STARTUP_RETRY_INTERVAL_SECONDS", 2),
        }
    }

    pub fn database_url(&self) -> String {
        let explicit = self.db_url.trim();
        if !explicit.is_empty() {
            if let Some(rest) = explicit.strip_prefix("postgresql+psycopg://") {
                return format!("postgresql://{rest}");
            }
            return explicit.to_string();
        }

        let user = url_encode(&self.db_user);
        let password = url_encode(&self.db_password);
        format!(
            "postgresql://{user}:{password}@{}:{}/{}",
            self.db_host, self.db_port, self.db_name,
        )
    }

    pub fn is_dev_env(&self) -> bool {
        matches!(
            self.app_env.trim().to_lowercase().as_str(),
            "dev" | "development" | "local" | "test"
        )
    }
}

pub fn normalize_admin_manager_web_path(value: Option<&str>, fallback: &str) -> String {
    let mut normalized_fallback = fallback.trim();
    if normalized_fallback.is_empty() {
        normalized_fallback = "/nehex-admin";
    }

    let mut fallback_path = normalized_fallback.to_string();
    if !fallback_path.starts_with('/') {
        fallback_path.insert(0, '/');
    }
    if fallback_path != "/" {
        fallback_path = fallback_path.trim_end_matches('/').to_string();
    }
    if fallback_path == "/" {
        fallback_path = "/nehex-admin".to_string();
    }

    let text = value.unwrap_or("").trim();
    if text.is_empty() {
        return fallback_path;
    }

    let mut normalized = text.to_string();
    if !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    if normalized != "/" {
        normalized = normalized.trim_end_matches('/').to_string();
    }
    if normalized == "/" {
        return fallback_path;
    }

    normalized
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .trim()
        .to_string()
}

fn env_bool(key: &str, default: bool) -> bool {
    let value = env::var(key).unwrap_or_default();
    if value.trim().is_empty() {
        return default;
    }

    matches!(
        value.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn env_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
