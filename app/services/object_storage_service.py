from __future__ import annotations

import posixpath
import re
import secrets
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal, Optional
from urllib.parse import quote, urlparse

from sqlalchemy import select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.models.setting import Setting

StorageProvider = Literal["local", "r2", "oss"]

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_LOCAL_ROOT = "storage"
DEFAULT_LOCAL_PATH_RULE = "/{year}-{month}/{day}/{random_name}.{file_type}"
DEFAULT_LOCAL_URL_PREFIX = "/storage"
MAX_IMAGE_SIZE_BYTES = 20 * 1024 * 1024

STORAGE_PROVIDER_KEY = "object_storage_provider"
STORAGE_ENABLED_KEY = "object_storage_enabled"
STORAGE_PUBLIC_BASE_URL_KEY = "object_storage_public_base_url"
STORAGE_LOCAL_ROOT_KEY = "object_storage_local_root"
STORAGE_LOCAL_PATH_RULE_KEY = "object_storage_local_path_rule"
STORAGE_R2_ENDPOINT_KEY = "object_storage_r2_endpoint"
STORAGE_R2_BUCKET_KEY = "object_storage_r2_bucket"
STORAGE_R2_ACCESS_KEY_ID_KEY = "object_storage_r2_access_key_id"
STORAGE_R2_SECRET_ACCESS_KEY_KEY = "object_storage_r2_secret_access_key"
STORAGE_R2_REGION_KEY = "object_storage_r2_region"
STORAGE_OSS_ENDPOINT_KEY = "object_storage_oss_endpoint"
STORAGE_OSS_BUCKET_KEY = "object_storage_oss_bucket"
STORAGE_OSS_ACCESS_KEY_ID_KEY = "object_storage_oss_access_key_id"
STORAGE_OSS_SECRET_ACCESS_KEY_KEY = "object_storage_oss_secret_access_key"

SETTINGS_KEYS = {
    STORAGE_PROVIDER_KEY,
    STORAGE_ENABLED_KEY,
    STORAGE_PUBLIC_BASE_URL_KEY,
    STORAGE_LOCAL_ROOT_KEY,
    STORAGE_LOCAL_PATH_RULE_KEY,
    STORAGE_R2_ENDPOINT_KEY,
    STORAGE_R2_BUCKET_KEY,
    STORAGE_R2_ACCESS_KEY_ID_KEY,
    STORAGE_R2_SECRET_ACCESS_KEY_KEY,
    STORAGE_R2_REGION_KEY,
    STORAGE_OSS_ENDPOINT_KEY,
    STORAGE_OSS_BUCKET_KEY,
    STORAGE_OSS_ACCESS_KEY_ID_KEY,
    STORAGE_OSS_SECRET_ACCESS_KEY_KEY,
}

IMAGE_MIME_TO_EXT = {
    "image/jpeg": "jpg",
    "image/png": "png",
    "image/webp": "webp",
    "image/gif": "gif",
    "image/svg+xml": "svg",
    "image/bmp": "bmp",
    "image/avif": "avif",
}
ALLOWED_IMAGE_EXTENSIONS = set(IMAGE_MIME_TO_EXT.values()) | {"jpeg", "jpg", "png", "webp", "gif", "svg", "bmp", "avif"}


@dataclass(frozen=True)
class ObjectStorageConfig:
    provider: StorageProvider
    enabled: bool
    public_base_url: str
    local_root: str
    local_path_rule: str
    r2_endpoint: str
    r2_bucket: str
    r2_access_key_id: str
    r2_secret_access_key: str
    r2_region: str
    oss_endpoint: str
    oss_bucket: str
    oss_access_key_id: str
    oss_secret_access_key: str


def _normalize_text(value: Optional[str]) -> str:
    return str(value or "").strip()


def _parse_boolean(value: Optional[str], default: bool) -> bool:
    text = _normalize_text(value).lower()
    if not text:
        return default
    return text in {"1", "true", "yes", "on"}


def _normalize_provider(value: Optional[str]) -> StorageProvider:
    text = _normalize_text(value).lower()
    if text in {"local", "r2", "oss"}:
        return text  # type: ignore[return-value]
    return "local"


def _normalize_endpoint(value: str) -> str:
    text = _normalize_text(value)
    if not text:
        return ""
    if "://" not in text:
        text = f"https://{text}"
    return text.rstrip("/")


def _normalize_bucket(value: str) -> str:
    return _normalize_text(value)


def _normalize_base_url(value: str) -> str:
    text = _normalize_text(value)
    if not text:
        return ""
    if not re.match(r"^https?://", text, flags=re.IGNORECASE):
        return ""
    return text.rstrip("/")


def _settings_map(session: Session) -> dict[str, str]:
    try:
        rows = session.execute(
            select(Setting.setting_key, Setting.setting_content).where(Setting.setting_key.in_(SETTINGS_KEYS)),
        ).all()
    except SQLAlchemyError:
        return {}
    return {
        str(setting_key): _normalize_text(setting_content)
        for setting_key, setting_content in rows
    }


def get_object_storage_config(session: Session) -> ObjectStorageConfig:
    setting_map = _settings_map(session)
    return ObjectStorageConfig(
        provider=_normalize_provider(setting_map.get(STORAGE_PROVIDER_KEY)),
        enabled=_parse_boolean(setting_map.get(STORAGE_ENABLED_KEY), default=True),
        public_base_url=_normalize_base_url(setting_map.get(STORAGE_PUBLIC_BASE_URL_KEY, "")),
        local_root=_normalize_text(setting_map.get(STORAGE_LOCAL_ROOT_KEY)) or DEFAULT_LOCAL_ROOT,
        local_path_rule=_normalize_text(setting_map.get(STORAGE_LOCAL_PATH_RULE_KEY)) or DEFAULT_LOCAL_PATH_RULE,
        r2_endpoint=_normalize_endpoint(setting_map.get(STORAGE_R2_ENDPOINT_KEY, "")),
        r2_bucket=_normalize_bucket(setting_map.get(STORAGE_R2_BUCKET_KEY, "")),
        r2_access_key_id=_normalize_text(setting_map.get(STORAGE_R2_ACCESS_KEY_ID_KEY, "")),
        r2_secret_access_key=_normalize_text(setting_map.get(STORAGE_R2_SECRET_ACCESS_KEY_KEY, "")),
        r2_region=_normalize_text(setting_map.get(STORAGE_R2_REGION_KEY, "")) or "auto",
        oss_endpoint=_normalize_endpoint(setting_map.get(STORAGE_OSS_ENDPOINT_KEY, "")),
        oss_bucket=_normalize_bucket(setting_map.get(STORAGE_OSS_BUCKET_KEY, "")),
        oss_access_key_id=_normalize_text(setting_map.get(STORAGE_OSS_ACCESS_KEY_ID_KEY, "")),
        oss_secret_access_key=_normalize_text(setting_map.get(STORAGE_OSS_SECRET_ACCESS_KEY_KEY, "")),
    )


def _join_public_url(base_url: str, key: str) -> str:
    encoded_key = quote(key, safe="/-_.~")
    return f"{base_url.rstrip('/')}/{encoded_key.lstrip('/')}"


def _build_local_public_base_url(public_base_url: str) -> str:
    normalized = _normalize_base_url(public_base_url)
    if not normalized:
        return DEFAULT_LOCAL_URL_PREFIX

    parsed = urlparse(normalized)
    path = parsed.path.rstrip("/")
    if path.endswith(DEFAULT_LOCAL_URL_PREFIX):
        return normalized
    return f"{normalized}{DEFAULT_LOCAL_URL_PREFIX}"


def _resolve_local_root_path(local_root: str) -> Path:
    root = Path(local_root)
    if not root.is_absolute():
        root = PROJECT_ROOT / root
    return root.resolve()


def _is_within_root(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _safe_posix_path(value: str) -> str:
    normalized = value.replace("\\", "/").strip()
    if not normalized:
        return ""

    normalized = "/" + normalized.lstrip("/")
    safe = posixpath.normpath(normalized).lstrip("/")
    if not safe or safe in {".", "/"}:
        return ""
    if safe.startswith("..") or "/../" in f"/{safe}":
        return ""
    return safe


class _FormatDict(dict[str, str]):
    def __missing__(self, key: str) -> str:
        return ""


def _guess_file_extension(file_name: str, content_type: str) -> str:
    ext = Path(file_name).suffix.lower().lstrip(".")
    if ext in ALLOWED_IMAGE_EXTENSIONS:
        if ext == "jpeg":
            return "jpg"
        return ext

    mapped = IMAGE_MIME_TO_EXT.get(content_type.lower())
    if mapped:
        return mapped

    return "jpg"


def _build_object_key(path_rule: str, file_name: str, content_type: str) -> str:
    now = datetime.now()
    extension = _guess_file_extension(file_name, content_type)
    random_name = secrets.token_hex(8)
    fallback = f"{now.strftime('%Y%m%d')}/{random_name}.{extension}"

    tokens = _FormatDict(
        year=now.strftime("%Y"),
        month=now.strftime("%m"),
        day=now.strftime("%d"),
        hour=now.strftime("%H"),
        minute=now.strftime("%M"),
        second=now.strftime("%S"),
        timestamp=str(int(now.timestamp())),
        random_name=random_name,
        file_type=extension,
    )

    template = _normalize_text(path_rule) or DEFAULT_LOCAL_PATH_RULE
    if not template.startswith("/"):
        template = f"/{template}"

    try:
        rendered = template.format_map(tokens)
    except Exception:
        rendered = f"/{fallback}"

    object_key = _safe_posix_path(rendered)
    if not object_key:
        return fallback

    if "." not in Path(object_key).name:
        object_key = f"{object_key}.{extension}"

    return object_key


def _validate_image_file(file_name: str, content_type: str, content: bytes) -> None:
    if not content:
        raise ValueError("上传文件为空")
    if len(content) > MAX_IMAGE_SIZE_BYTES:
        raise ValueError("图片大小不能超过 20MB")

    normalized_content_type = _normalize_text(content_type).lower()
    if normalized_content_type and normalized_content_type.startswith("image/"):
        return

    extension = Path(file_name).suffix.lower().lstrip(".")
    if extension in ALLOWED_IMAGE_EXTENSIONS:
        return

    raise ValueError("仅支持图片文件上传")


def _upload_local_file(config: ObjectStorageConfig, object_key: str, content: bytes) -> str:
    root = _resolve_local_root_path(config.local_root)
    root.mkdir(parents=True, exist_ok=True)

    target = (root / object_key).resolve()
    if not _is_within_root(target, root):
        raise ValueError("非法文件路径")

    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(content)
    base_url = _build_local_public_base_url(config.public_base_url)
    return _join_public_url(base_url, object_key)


def _upload_r2_file(
    config: ObjectStorageConfig,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if not config.r2_endpoint or not config.r2_bucket or not config.r2_access_key_id or not config.r2_secret_access_key:
        raise ValueError("CloudFlare R2 配置不完整")

    try:
        import boto3
    except Exception as error:  # pragma: no cover - runtime dependency
        raise RuntimeError("缺少 boto3 依赖，无法上传到 CloudFlare R2") from error

    client = boto3.client(
        "s3",
        endpoint_url=config.r2_endpoint,
        aws_access_key_id=config.r2_access_key_id,
        aws_secret_access_key=config.r2_secret_access_key,
        region_name=config.r2_region or "auto",
    )
    put_kwargs = {
        "Bucket": config.r2_bucket,
        "Key": object_key,
        "Body": content,
    }
    if content_type:
        put_kwargs["ContentType"] = content_type
    client.put_object(**put_kwargs)

    if config.public_base_url:
        return _join_public_url(config.public_base_url, object_key)
    return _join_public_url(f"{config.r2_endpoint}/{config.r2_bucket}", object_key)


def _upload_oss_file(
    config: ObjectStorageConfig,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if not config.oss_endpoint or not config.oss_bucket or not config.oss_access_key_id or not config.oss_secret_access_key:
        raise ValueError("阿里云 OSS 配置不完整")

    try:
        import oss2
    except Exception as error:  # pragma: no cover - runtime dependency
        raise RuntimeError("缺少 oss2 依赖，无法上传到阿里云 OSS") from error

    auth = oss2.Auth(config.oss_access_key_id, config.oss_secret_access_key)
    bucket = oss2.Bucket(auth, config.oss_endpoint, config.oss_bucket)
    headers = {"Content-Type": content_type} if content_type else None
    result = bucket.put_object(object_key, content, headers=headers)
    if int(getattr(result, "status", 200)) >= 300:
        raise RuntimeError(f"阿里云 OSS 上传失败: {getattr(result, 'status', 'unknown')}")

    if config.public_base_url:
        return _join_public_url(config.public_base_url, object_key)

    parsed = urlparse(config.oss_endpoint)
    host = parsed.netloc or parsed.path
    if not host:
        return _join_public_url(config.oss_endpoint, object_key)
    return _join_public_url(f"https://{config.oss_bucket}.{host}", object_key)


def upload_image_to_object_storage(
    session: Session,
    *,
    file_name: str,
    content_type: str,
    content: bytes,
) -> dict[str, str]:
    config = get_object_storage_config(session)
    if not config.enabled:
        raise ValueError("对象存储未启用")

    _validate_image_file(file_name, content_type, content)
    object_key = _build_object_key(config.local_path_rule, file_name=file_name, content_type=content_type)

    if config.provider == "local":
        url = _upload_local_file(config, object_key=object_key, content=content)
    elif config.provider == "r2":
        url = _upload_r2_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )
    else:
        url = _upload_oss_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )

    return {
        "provider": config.provider,
        "key": object_key,
        "url": url,
    }


def resolve_local_storage_file_path(session: Session, file_path: str) -> Path | None:
    safe_path = _safe_posix_path(file_path)
    if not safe_path:
        return None

    config = get_object_storage_config(session)
    root = _resolve_local_root_path(config.local_root)
    target = (root / safe_path).resolve()
    if not _is_within_root(target, root):
        return None
    if not target.is_file():
        return None
    return target
