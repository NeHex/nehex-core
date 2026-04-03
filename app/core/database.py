from collections.abc import Generator

from sqlalchemy import inspect, text
from sqlalchemy.orm import Session, sessionmaker
from sqlalchemy import create_engine

from app.core.config import settings
from app.models.base import Base
from app.models.friend_apply import FriendApply
import app.models  # noqa: F401


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
    Base.metadata.create_all(bind=engine, tables=[FriendApply.__table__], checkfirst=True)


def ensure_all_tables() -> None:
    Base.metadata.create_all(bind=engine, checkfirst=True)


def list_database_tables() -> set[str]:
    inspector = inspect(engine)
    return set(inspector.get_table_names())


def database_table_exists(table_name: str) -> bool:
    inspector = inspect(engine)
    return inspector.has_table(table_name)


def ensure_performance_indexes() -> None:
    index_specs = [
        ("comment", "idx_comment_target_status_time", "target_type,target_id,status,create_time,id"),
        ("comment", "idx_comment_ip_time", "ip,create_time"),
        ("album", "idx_album_update_time", "update_time,id"),
        ("daily", "idx_daily_create_time", "create_time,id"),
        ("singlepage", "idx_singlepage_status_sort", "status,sort,id"),
        ("project", "idx_project_status_sort", "status,sort,id"),
        ("friends", "idx_friends_status_time", "status,create_time,id"),
        ("friend_apply", "idx_friend_apply_status_time", "status,create_time,id"),
        ("friend_apply", "idx_friend_apply_url_time", "site_url,create_time,id"),
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
        for table_name, index_name, columns_sql in index_specs:
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

            conn.execute(
                text(f"CREATE INDEX {index_name} ON {table_name} ({columns_sql})"),
            )


def close_database() -> None:
    engine.dispose()
