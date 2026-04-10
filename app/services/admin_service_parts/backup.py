from __future__ import annotations

import base64
import io
import json
import re
import secrets
import shutil
import tarfile
from datetime import date, datetime, time, timezone
from decimal import Decimal
from pathlib import Path
from typing import Any
from typing import BinaryIO

from sqlalchemy import inspect, select
from sqlalchemy import text
from sqlalchemy.sql.sqltypes import BigInteger, Integer, SmallInteger
from sqlalchemy.orm import Session

import app.models  # noqa: F401
from app.core.simple_cache import cache
from app.core.database import engine
from app.models.base import Base
from app.schemas.admin import AdminBackupItem
from app.services.object_storage_service import get_object_storage_config

PROJECT_ROOT = Path(__file__).resolve().parents[3]
BACKUP_ROOT = PROJECT_ROOT / "backups"
BACKUP_TMP_ROOT = BACKUP_ROOT / ".tmp"
BACKUP_FILENAME_RE = re.compile(r"^nehex-backup-\d{8}-\d{6}(?:-[a-z0-9]{6})?\.tar\.gz$")
SERIALIZED_TYPE_KEY = "$nehex_type"
MAX_BACKUP_UPLOAD_BYTES = 512 * 1024 * 1024
BACKUP_UPLOAD_CHUNK_BYTES = 1024 * 1024
MAX_BACKUP_EXTRACT_BYTES = 2 * 1024 * 1024 * 1024
MYSQL_SYSTEM_LOG_TABLES = {"slow_log", "general_log"}


def _list_existing_app_tables():
    inspector = inspect(engine)
    return [
        table
        for table in Base.metadata.sorted_tables
        if inspector.has_table(table.name, schema=table.schema)
    ]


def _serialize_db_value(value: Any) -> Any:
    if value is None:
        return None
    if isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, datetime):
        return {SERIALIZED_TYPE_KEY: "datetime", "value": value.isoformat()}
    if isinstance(value, date):
        return {SERIALIZED_TYPE_KEY: "date", "value": value.isoformat()}
    if isinstance(value, time):
        return {SERIALIZED_TYPE_KEY: "time", "value": value.isoformat()}
    if isinstance(value, Decimal):
        return {SERIALIZED_TYPE_KEY: "decimal", "value": str(value)}
    if isinstance(value, bytes):
        return {
            SERIALIZED_TYPE_KEY: "bytes",
            "value": base64.b64encode(value).decode("ascii"),
        }
    if isinstance(value, dict):
        return {str(k): _serialize_db_value(v) for k, v in value.items()}
    if isinstance(value, (list, tuple, set)):
        return [_serialize_db_value(item) for item in value]
    return str(value)


def _deserialize_db_value(value: Any) -> Any:
    if isinstance(value, dict):
        marker = value.get(SERIALIZED_TYPE_KEY)
        if isinstance(marker, str) and "value" in value:
            raw = value.get("value")
            if marker == "datetime" and isinstance(raw, str):
                return datetime.fromisoformat(raw)
            if marker == "date" and isinstance(raw, str):
                return date.fromisoformat(raw)
            if marker == "time" and isinstance(raw, str):
                return time.fromisoformat(raw)
            if marker == "decimal" and isinstance(raw, str):
                return Decimal(raw)
            if marker == "bytes" and isinstance(raw, str):
                return base64.b64decode(raw.encode("ascii"))
        return {str(k): _deserialize_db_value(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_deserialize_db_value(item) for item in value]
    return value


def _ensure_backup_root() -> None:
    BACKUP_ROOT.mkdir(parents=True, exist_ok=True)
    BACKUP_TMP_ROOT.mkdir(parents=True, exist_ok=True)


def _build_backup_filename() -> str:
    now = datetime.now().strftime("%Y%m%d-%H%M%S")
    return f"nehex-backup-{now}-{secrets.token_hex(3)}.tar.gz"


def _is_valid_backup_filename(filename: str) -> bool:
    return bool(BACKUP_FILENAME_RE.match(filename))


def _resolve_backup_file(filename: str) -> Path:
    normalized = filename.strip()
    if not _is_valid_backup_filename(normalized):
        raise ValueError("无效的备份文件名")

    path = (BACKUP_ROOT / normalized).resolve()
    try:
        path.relative_to(BACKUP_ROOT.resolve())
    except ValueError as error:
        raise ValueError("非法备份路径") from error

    return path


def _validate_uploaded_backup_file_name(file_name: str) -> None:
    normalized_name = file_name.strip().lower()
    if not normalized_name.endswith(".tar.gz"):
        raise ValueError("仅支持 .tar.gz 备份文件")


def _to_backup_item(path: Path) -> AdminBackupItem:
    stat = path.stat()
    return AdminBackupItem(
        filename=path.name,
        size_bytes=max(0, int(stat.st_size)),
        created_at=datetime.fromtimestamp(stat.st_ctime, tz=timezone.utc),
        updated_at=datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc),
    )


def _snapshot_database(snapshot_root: Path) -> None:
    app_tables = _list_existing_app_tables()

    db_payload: dict[str, Any] = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "dialect": engine.dialect.name,
        "tables": [],
    }

    with engine.connect() as connection:
        for table in app_tables:
            rows = connection.execute(select(table)).mappings().all()
            serialized_rows = [
                {str(key): _serialize_db_value(value) for key, value in row.items()}
                for row in rows
            ]
            db_payload["tables"].append(
                {
                    "name": table.name,
                    "columns": [column.name for column in table.columns],
                    "rows": serialized_rows,
                },
            )

    db_dir = snapshot_root / "database"
    db_dir.mkdir(parents=True, exist_ok=True)
    db_file = db_dir / "data.json"
    db_file.write_text(
        json.dumps(db_payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def _resolve_local_storage_root(session: Session) -> Path:
    config = get_object_storage_config(session)
    root = Path(config.local_root)
    if not root.is_absolute():
        root = PROJECT_ROOT / root
    return root.resolve()


def _snapshot_storage(session: Session, snapshot_root: Path) -> dict[str, Any]:
    storage_root = _resolve_local_storage_root(session)
    storage_dest = snapshot_root / "storage"

    copied = False
    file_count = 0
    if storage_root.exists() and storage_root.is_dir():
        shutil.copytree(storage_root, storage_dest, dirs_exist_ok=True)
        copied = True
        file_count = sum(1 for item in storage_dest.rglob("*") if item.is_file())

    return {
        "local_storage_root": str(storage_root),
        "copied": copied,
        "file_count": file_count,
    }


def _snapshot_meta(snapshot_root: Path, *, storage_meta: dict[str, Any]) -> None:
    meta_payload = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "project_root": str(PROJECT_ROOT),
        "storage": storage_meta,
    }
    (snapshot_root / "meta.json").write_text(
        json.dumps(meta_payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def list_admin_backups() -> list[AdminBackupItem]:
    _ensure_backup_root()
    backups = [
        _to_backup_item(path)
        for path in BACKUP_ROOT.glob("*.tar.gz")
        if path.is_file() and _is_valid_backup_filename(path.name)
    ]
    backups.sort(key=lambda item: item.updated_at, reverse=True)
    return backups


def create_admin_backup(session: Session) -> AdminBackupItem:
    _ensure_backup_root()
    filename = _build_backup_filename()
    backup_path = BACKUP_ROOT / filename
    tmp_root = (BACKUP_TMP_ROOT / f"backup-{secrets.token_hex(8)}").resolve()
    snapshot_root = tmp_root / "snapshot"

    tmp_root.mkdir(parents=True, exist_ok=True)
    snapshot_root.mkdir(parents=True, exist_ok=True)

    try:
        _snapshot_database(snapshot_root)
        storage_meta = _snapshot_storage(session, snapshot_root)
        _snapshot_meta(snapshot_root, storage_meta=storage_meta)

        with tarfile.open(backup_path, mode="w:gz") as tar:
            tar.add(snapshot_root, arcname="snapshot")
    finally:
        shutil.rmtree(tmp_root, ignore_errors=True)

    return _to_backup_item(backup_path)


def get_admin_backup_file_path(filename: str) -> Path:
    path = _resolve_backup_file(filename)
    if not path.exists() or not path.is_file():
        raise FileNotFoundError("备份文件不存在")
    return path


def delete_admin_backup(filename: str) -> AdminBackupItem:
    path = get_admin_backup_file_path(filename)
    item = _to_backup_item(path)
    path.unlink()
    return item


def save_uploaded_backup_file(file_name: str, content: bytes) -> AdminBackupItem:
    _validate_uploaded_backup_file_name(file_name)
    _ensure_backup_root()
    if not content:
        raise ValueError("上传备份文件为空")
    if len(content) > MAX_BACKUP_UPLOAD_BYTES:
        raise ValueError("备份文件不能超过 512MB")

    filename = _build_backup_filename()
    backup_path = BACKUP_ROOT / filename
    backup_path.write_bytes(content)
    return _to_backup_item(backup_path)


def save_uploaded_backup_file_stream(file_name: str, stream: BinaryIO) -> AdminBackupItem:
    _validate_uploaded_backup_file_name(file_name)
    _ensure_backup_root()

    filename = _build_backup_filename()
    backup_path = BACKUP_ROOT / filename
    total_bytes = 0

    with backup_path.open("wb") as target:
        while True:
            chunk = stream.read(BACKUP_UPLOAD_CHUNK_BYTES)
            if not chunk:
                break
            total_bytes += len(chunk)
            if total_bytes > MAX_BACKUP_UPLOAD_BYTES:
                backup_path.unlink(missing_ok=True)
                raise ValueError("备份文件不能超过 512MB")
            target.write(chunk)

    if total_bytes <= 0:
        backup_path.unlink(missing_ok=True)
        raise ValueError("上传备份文件为空")

    return _to_backup_item(backup_path)


def _safe_extract_tar(archive_path: Path, dest_root: Path) -> None:
    try:
        with tarfile.open(archive_path, mode="r:gz") as tar:
            members = tar.getmembers()
            total_extract_bytes = 0
            for member in members:
                if not member.name or member.name.startswith("/"):
                    raise ValueError("备份压缩包包含非法路径")
                if member.issym() or member.islnk() or member.isdev() or member.isfifo():
                    raise ValueError("备份压缩包包含不安全条目")
                member_path = (dest_root / member.name).resolve()
                try:
                    member_path.relative_to(dest_root.resolve())
                except ValueError as error:
                    raise ValueError("备份压缩包包含非法路径") from error
                if member.isfile():
                    total_extract_bytes += max(0, int(member.size))
                    if total_extract_bytes > MAX_BACKUP_EXTRACT_BYTES:
                        raise ValueError("备份文件解压后体积超过限制")

            for member in members:
                target = (dest_root / member.name).resolve()
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                if member.isfile():
                    target.parent.mkdir(parents=True, exist_ok=True)
                    source = tar.extractfile(member)
                    if source is None:
                        raise ValueError("备份压缩包包含无法读取的文件")
                    with source, target.open("wb") as dest_file:
                        shutil.copyfileobj(source, dest_file, length=io.DEFAULT_BUFFER_SIZE)
                    continue
                raise ValueError("备份压缩包包含不支持的条目")
    except tarfile.TarError as error:
        raise ValueError("备份文件格式错误，必须是 tar.gz 压缩包") from error


def _restore_database(snapshot_root: Path) -> None:
    db_file = snapshot_root / "database" / "data.json"
    if not db_file.exists():
        raise ValueError("备份包中缺少数据库快照")

    payload = json.loads(db_file.read_text(encoding="utf-8"))
    raw_tables = payload.get("tables")
    if not isinstance(raw_tables, list):
        raise ValueError("数据库快照格式不正确")

    app_tables = _list_existing_app_tables()
    table_map = {table.name: table for table in app_tables}

    with engine.begin() as connection:
        dialect_name = engine.dialect.name
        is_mysql = dialect_name == "mysql"
        is_postgresql = dialect_name == "postgresql"
        identifier_preparer = connection.dialect.identifier_preparer

        if is_mysql:
            connection.exec_driver_sql("SET FOREIGN_KEY_CHECKS=0")
        elif is_postgresql:
            table_names = [
                identifier_preparer.quote(table.name)
                for table in app_tables
                if table.name not in MYSQL_SYSTEM_LOG_TABLES
            ]
            if table_names:
                connection.exec_driver_sql(
                    f"TRUNCATE TABLE {', '.join(table_names)} RESTART IDENTITY CASCADE",
                )

        try:
            if not is_postgresql:
                for table in reversed(app_tables):
                    if table.name in MYSQL_SYSTEM_LOG_TABLES:
                        continue
                    connection.execute(table.delete())

            for table_payload in raw_tables:
                if not isinstance(table_payload, dict):
                    continue
                table_name = str(table_payload.get("name") or "").strip()
                if not table_name:
                    continue
                table = table_map.get(table_name)
                if table is None:
                    continue

                raw_rows = table_payload.get("rows")
                if not isinstance(raw_rows, list) or not raw_rows:
                    continue

                allowed_columns = {column.name for column in table.columns}
                rows: list[dict[str, Any]] = []
                for raw_row in raw_rows:
                    if not isinstance(raw_row, dict):
                        continue
                    row: dict[str, Any] = {}
                    for key, value in raw_row.items():
                        if key in allowed_columns:
                            row[key] = _deserialize_db_value(value)
                    if row:
                        rows.append(row)

                if rows:
                    connection.execute(table.insert(), rows)

            if is_postgresql:
                for table in app_tables:
                    quoted_table = identifier_preparer.quote(table.name)
                    if table.schema:
                        quoted_table = f"{identifier_preparer.quote(table.schema)}.{quoted_table}"
                    for column in table.columns:
                        if not column.primary_key:
                            continue
                        if not isinstance(column.type, (SmallInteger, Integer, BigInteger)):
                            continue

                        sequence_name = connection.execute(
                            text("SELECT pg_get_serial_sequence(:table_name, :column_name)"),
                            {
                                "table_name": quoted_table,
                                "column_name": column.name,
                            },
                        ).scalar()
                        if not sequence_name:
                            continue

                        quoted_column = identifier_preparer.quote(column.name)
                        connection.execute(
                            text(
                                "SELECT setval(CAST(:seq_name AS regclass), "
                                f"COALESCE((SELECT MAX({quoted_column}) FROM {quoted_table}), 0) + 1, false)",
                            ),
                            {"seq_name": sequence_name},
                        )
        finally:
            if is_mysql:
                connection.exec_driver_sql("SET FOREIGN_KEY_CHECKS=1")


def _clear_directory(path: Path) -> None:
    if not path.exists():
        return
    for child in path.iterdir():
        if child.is_dir():
            shutil.rmtree(child, ignore_errors=True)
        else:
            child.unlink(missing_ok=True)


def _restore_storage(session: Session, snapshot_root: Path) -> None:
    storage_src = snapshot_root / "storage"
    if not storage_src.exists() or not storage_src.is_dir():
        return

    target_root = _resolve_local_storage_root(session)
    if target_root in {Path("/"), PROJECT_ROOT.resolve(), Path.home()}:
        raise ValueError("存储目录配置过于宽泛，拒绝覆盖")

    target_root.mkdir(parents=True, exist_ok=True)
    _clear_directory(target_root)

    for child in storage_src.iterdir():
        target = target_root / child.name
        if child.is_dir():
            shutil.copytree(child, target, dirs_exist_ok=True)
        else:
            shutil.copy2(child, target)


def restore_admin_backup(session: Session, filename: str, *, confirm_overwrite: bool) -> None:
    if not confirm_overwrite:
        raise ValueError("恢复操作需要确认覆盖现有数据")

    backup_path = get_admin_backup_file_path(filename)
    tmp_root = (BACKUP_TMP_ROOT / f"restore-{secrets.token_hex(8)}").resolve()
    tmp_root.mkdir(parents=True, exist_ok=True)

    try:
        _safe_extract_tar(backup_path, tmp_root)
        snapshot_root = tmp_root / "snapshot"
        if not snapshot_root.exists() or not snapshot_root.is_dir():
            raise ValueError("备份包缺少 snapshot 目录")

        _restore_database(snapshot_root)
        _restore_storage(session, snapshot_root)
        cache.clear()
    finally:
        shutil.rmtree(tmp_root, ignore_errors=True)


def upload_and_restore_admin_backup(
    session: Session,
    *,
    file_name: str,
    content: bytes,
    confirm_overwrite: bool,
) -> AdminBackupItem:
    item = save_uploaded_backup_file(file_name=file_name, content=content)
    restore_admin_backup(
        session=session,
        filename=item.filename,
        confirm_overwrite=confirm_overwrite,
    )
    return item


def upload_and_restore_admin_backup_stream(
    session: Session,
    *,
    file_name: str,
    stream: BinaryIO,
    confirm_overwrite: bool,
) -> AdminBackupItem:
    item = save_uploaded_backup_file_stream(file_name=file_name, stream=stream)
    restore_admin_backup(
        session=session,
        filename=item.filename,
        confirm_overwrite=confirm_overwrite,
    )
    return item
