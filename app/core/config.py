from typing import Optional
from urllib.parse import quote_plus

from pydantic_settings import BaseSettings, SettingsConfigDict


def normalize_admin_manager_web_path(value: Optional[str], fallback: str = "/nehex-admin") -> str:
    normalized_fallback = (fallback or "/nehex-admin").strip() or "/nehex-admin"
    if not normalized_fallback.startswith("/"):
        normalized_fallback = f"/{normalized_fallback}"
    if normalized_fallback != "/":
        normalized_fallback = normalized_fallback.rstrip("/")
    if normalized_fallback == "/":
        normalized_fallback = "/nehex-admin"

    text = (value or "").strip()
    if not text:
        return normalized_fallback
    if not text.startswith("/"):
        text = f"/{text}"
    if text != "/":
        text = text.rstrip("/")
    if text == "/":
        return normalized_fallback
    return text


class Settings(BaseSettings):
    app_name: str = "NeHex Core API"
    app_version: str = "v1.0.2"
    app_env: str = "dev"
    app_port: int = 7878
    cors_allow_origins: str = "http://127.0.0.1:3000,http://localhost:3000"
    cors_allow_credentials: bool = True
    admin_manager_web: str = "/nehex-admin"
    admin_api_secret: str = "please-change-me"
    admin_api_client_id: str = "nehex-vuetify-admin"
    admin_api_token_ttl_seconds: int = 43200
    admin_cookie_domain: str = ""
    admin_public_cookie_domain: str = ""
    admin_manager_build_on_startup: bool = True
    simple_cache_max_entries: int = 1024
    redis_enabled: bool = True
    redis_url: str = "redis://127.0.0.1:6379/0"
    redis_cache_prefix: str = "nehex:cache:"
    redis_connect_retry_seconds: int = 30
    redis_socket_connect_timeout: float = 1.0
    redis_socket_timeout: float = 1.5

    db_driver: str = "postgresql"
    db_url: str = ""
    db_host: str = "127.0.0.1"
    db_port: int = 5432
    db_name: str = "nehex_dtbs"
    db_user: str = "nehex_dtbs"
    db_password: str = ""

    db_pool_size: int = 10
    db_max_overflow: int = 20
    db_pool_recycle: int = 1800
    db_pool_timeout: int = 30
    db_connect_timeout: int = 5
    db_read_timeout: int = 15
    db_write_timeout: int = 15
    db_auto_create_tables: bool = False
    db_startup_max_retries: int = 30
    db_startup_retry_interval_seconds: int = 2

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
    )

    @property
    def database_url(self) -> str:
        explicit_url = self.db_url.strip()
        if explicit_url:
            return explicit_url

        driver = self.db_driver.strip().lower()
        encoded_user = quote_plus(self.db_user)
        encoded_password = quote_plus(self.db_password)

        if driver in {"postgresql", "postgres", "psycopg", "postgresql+psycopg"}:
            return (
                f"postgresql+psycopg://{encoded_user}:{encoded_password}"
                f"@{self.db_host}:{self.db_port}/{self.db_name}"
            )

        if driver in {"mysql", "pymysql", "mysql+pymysql"}:
            return (
                f"mysql+pymysql://{encoded_user}:{encoded_password}"
                f"@{self.db_host}:{self.db_port}/{self.db_name}"
                "?charset=utf8mb4"
            )

        raise ValueError(f"Unsupported DB_DRIVER: {self.db_driver}")

    @property
    def admin_manager_web_path(self) -> str:
        return normalize_admin_manager_web_path(self.admin_manager_web)

    @property
    def cors_allow_origins_list(self) -> list[str]:
        raw_items = [item.strip() for item in self.cors_allow_origins.split(",")]
        return [item for item in raw_items if item]


settings = Settings()
