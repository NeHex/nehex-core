from collections.abc import Generator
import logging

from sqlalchemy import inspect, text
from sqlalchemy.orm import Session, sessionmaker
from sqlalchemy import create_engine

from app.core.config import settings
from app.models.base import Base
from app.models.friend_apply import FriendApply
from app.models.mail_log import MailLog
import app.models  # noqa: F401

logger = logging.getLogger(__name__)


engine = create_engine(
    settings.database_url,
    pool_pre_ping=True,
    pool_size=settings.db_pool_size,
    max_overflow=settings.db_max_overflow,
    pool_recycle=settings.db_pool_recycle,
    pool_timeout=settings.db_pool_timeout,
    pool_use_lifo=True,
    pool_reset_on_return="rollback",
    connect_args={
        "connect_timeout": settings.db_connect_timeout,
        "read_timeout": settings.db_read_timeout,
        "write_timeout": settings.db_write_timeout,
    },
    echo=False,
    future=True,
)

SessionLocal = sessionmaker(
    bind=engine,
    expire_on_commit=False,
    class_=Session,
)


def get_db_session() -> Generator[Session, None, None]:
    with SessionLocal() as session:
        yield session


def check_database_connection() -> None:
    with engine.connect() as conn:
        conn.execute(text("SELECT 1"))


def ensure_system_tables() -> None:
    Base.metadata.create_all(bind=engine, tables=[FriendApply.__table__, MailLog.__table__], checkfirst=True)


def ensure_all_tables() -> None:
    Base.metadata.create_all(bind=engine, checkfirst=True)


def ensure_schema_compatibility_columns() -> None:
    inspector = inspect(engine)

    if not inspector.has_table("article"):
        return

    try:
        existing_columns = {str(column.get("name", "")) for column in inspector.get_columns("article")}
    except Exception as error:
        logger.warning("[startup] skip schema compatibility check for article columns: %s", error)
        return

    ddl_statements: list[str] = []
    if "like_count" not in existing_columns:
        ddl_statements.append("ALTER TABLE article ADD COLUMN like_count INT NOT NULL DEFAULT 0")

    if not ddl_statements:
        return

    with engine.begin() as conn:
        for ddl in ddl_statements:
            try:
                conn.execute(text(ddl))
            except Exception as error:
                logger.warning("[startup] skip schema compatibility ddl `%s`: %s", ddl, error)


def list_database_tables() -> set[str]:
    inspector = inspect(engine)
    return set(inspector.get_table_names())


def database_table_exists(table_name: str) -> bool:
    inspector = inspect(engine)
    return inspector.has_table(table_name)


def ensure_performance_indexes() -> None:
    index_specs = [
        ("comment", "idx_comment_target_status_time", "target_type,target_id,status,create_time,id", False),
        ("comment", "idx_comment_ip_time", "ip,create_time", False),
        ("album", "idx_album_update_time", "update_time,id", False),
        ("daily", "idx_daily_create_time", "create_time,id", False),
        ("singlepage", "idx_singlepage_status_sort", "status,sort,id", False),
        ("project", "idx_project_status_sort", "status,sort,id", False),
        ("friends", "idx_friends_status_time", "status,create_time,id", False),
        ("friend_apply", "idx_friend_apply_status_time", "status,create_time,id", False),
        ("friend_apply", "idx_friend_apply_url_time", "site_url,create_time,id", False),
        ("friends", "uq_friends_url", "url", True),
        ("mail_log", "idx_mail_log_status_time", "status,created_at,id", False),
        ("mail_log", "idx_mail_log_comment", "trigger_comment_id,created_at,id", False),
    ]

    check_table_sql = text(
        """
        SELECT COUNT(1)
        FROM information_schema.tables
        WHERE table_schema = :schema_name
          AND table_name = :table_name
        """.strip(),
    )

    check_exists_sql = text(
        """
        SELECT COUNT(1)
        FROM information_schema.statistics
        WHERE table_schema = :schema_name
          AND table_name = :table_name
          AND index_name = :index_name
        """.strip(),
    )

    with engine.begin() as conn:
        for table_name, index_name, columns_sql, is_unique in index_specs:
            table_exists = int(
                conn.execute(
                    check_table_sql,
                    {
                        "schema_name": settings.db_name,
                        "table_name": table_name,
                    },
                ).scalar()
                or 0,
            )
            if table_exists <= 0:
                continue

            exists = int(
                conn.execute(
                    check_exists_sql,
                    {
                        "schema_name": settings.db_name,
                        "table_name": table_name,
                        "index_name": index_name,
                    },
                ).scalar()
                or 0,
            )
            if exists > 0:
                continue

            ddl_prefix = "CREATE UNIQUE INDEX" if is_unique else "CREATE INDEX"
            try:
                conn.execute(
                    text(f"{ddl_prefix} {index_name} ON {table_name} ({columns_sql})"),
                )
            except Exception as error:
                # Continue with remaining indexes to avoid blocking startup due to a single bad index.
                logger.warning("[startup] skip index %s on %s: %s", index_name, table_name, error)


def close_database() -> None:
    engine.dispose()
