from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    app_name: str = "NeHex Core API"
    app_env: str = "dev"
    app_port: int = 7878
    admin_manager_web: str = "/nehex-admin"
    admin_api_secret: str = "please-change-me"
    admin_api_client_id: str = "nehex-vuetify-admin"
    admin_api_token_ttl_seconds: int = 43200

    db_host: str = "127.0.0.1"
    db_port: int = 3306
    db_name: str = "nehex_dtbs"
    db_user: str = "nehex_dtbs"
    db_password: str = ""
    db_charset: str = "utf8mb4"

    db_pool_size: int = 10
    db_max_overflow: int = 20
    db_pool_recycle: int = 1800
    db_pool_timeout: int = 30
    db_connect_timeout: int = 5
    db_read_timeout: int = 15
    db_write_timeout: int = 15

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
    )

    @property
    def database_url(self) -> str:
        return (
            f"mysql+pymysql://{self.db_user}:{self.db_password}"
            f"@{self.db_host}:{self.db_port}/{self.db_name}"
            f"?charset={self.db_charset}"
        )

    @property
    def admin_manager_web_path(self) -> str:
        value = (self.admin_manager_web or "/nehex-admin").strip()
        if not value:
            value = "/nehex-admin"
        if not value.startswith("/"):
            value = f"/{value}"
        if value != "/":
            value = value.rstrip("/")
        if value == "/":
            value = "/nehex-admin"
        return value


settings = Settings()
