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

StorageProvider = Literal["local", "r2", "s3", "aliyun_oss", "hi168_s3"]

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_LOCAL_ROOT = "storage"
DEFAULT_LOCAL_PATH_RULE = "/{year}-{month}/{day}/{random_name}.{file_type}"
DEFAULT_LOCAL_URL_PREFIX = "/storage"
MAX_IMAGE_SIZE_BYTES = 20 * 1024 * 1024
MAX_MEDIA_SIZE_BYTES = 200 * 1024 * 1024

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
STORAGE_S3_ENDPOINT_KEY = "object_storage_s3_endpoint"
STORAGE_S3_BUCKET_KEY = "object_storage_s3_bucket"
STORAGE_S3_ACCESS_KEY_ID_KEY = "object_storage_s3_access_key_id"
STORAGE_S3_SECRET_ACCESS_KEY_KEY = "object_storage_s3_secret_access_key"
STORAGE_S3_REGION_KEY = "object_storage_s3_region"
STORAGE_HI168_S3_ENDPOINT_KEY = "object_storage_hi168_s3_endpoint"
STORAGE_HI168_S3_BUCKET_KEY = "object_storage_hi168_s3_bucket"
STORAGE_HI168_S3_ACCESS_KEY_ID_KEY = "object_storage_hi168_s3_access_key_id"
STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY = "object_storage_hi168_s3_secret_access_key"
STORAGE_HI168_S3_REGION_KEY = "object_storage_hi168_s3_region"
STORAGE_ALIYUN_OSS_ENDPOINT_KEY = "object_storage_aliyun_oss_endpoint"
STORAGE_ALIYUN_OSS_BUCKET_KEY = "object_storage_aliyun_oss_bucket"
STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY = "object_storage_aliyun_oss_access_key_id"
STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY = "object_storage_aliyun_oss_secret_access_key"
STORAGE_ALIYUN_OSS_REGION_KEY = "object_storage_aliyun_oss_region"

# Backward compatibility for old "oss" provider settings.
LEGACY_STORAGE_OSS_ENDPOINT_KEY = "object_storage_oss_endpoint"
LEGACY_STORAGE_OSS_BUCKET_KEY = "object_storage_oss_bucket"
LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY = "object_storage_oss_access_key_id"
LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY = "object_storage_oss_secret_access_key"

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
    STORAGE_S3_ENDPOINT_KEY,
    STORAGE_S3_BUCKET_KEY,
    STORAGE_S3_ACCESS_KEY_ID_KEY,
    STORAGE_S3_SECRET_ACCESS_KEY_KEY,
    STORAGE_S3_REGION_KEY,
    STORAGE_HI168_S3_ENDPOINT_KEY,
    STORAGE_HI168_S3_BUCKET_KEY,
    STORAGE_HI168_S3_ACCESS_KEY_ID_KEY,
    STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY,
    STORAGE_HI168_S3_REGION_KEY,
    STORAGE_ALIYUN_OSS_ENDPOINT_KEY,
    STORAGE_ALIYUN_OSS_BUCKET_KEY,
    STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY,
    STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY,
    STORAGE_ALIYUN_OSS_REGION_KEY,
    LEGACY_STORAGE_OSS_ENDPOINT_KEY,
    LEGACY_STORAGE_OSS_BUCKET_KEY,
    LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY,
    LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY,
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
MIME_TO_EXT = {
    **IMAGE_MIME_TO_EXT,
    "video/mp4": "mp4",
    "video/webm": "webm",
    "video/ogg": "ogv",
    "video/quicktime": "mov",
    "video/x-matroska": "mkv",
    "audio/mpeg": "mp3",
    "audio/wav": "wav",
    "audio/ogg": "ogg",
    "audio/flac": "flac",
    "audio/aac": "aac",
    "application/pdf": "pdf",
    "text/plain": "txt",
    "text/markdown": "md",
    "text/csv": "csv",
    "application/json": "json",
    "application/zip": "zip",
    "application/x-zip-compressed": "zip",
    "application/x-rar-compressed": "rar",
    "application/vnd.rar": "rar",
    "application/x-7z-compressed": "7z",
    "application/msword": "doc",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
    "application/vnd.ms-excel": "xls",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": "xlsx",
    "application/vnd.ms-powerpoint": "ppt",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation": "pptx",
}
BLOCKED_FILE_EXTENSIONS = {
    "html",
    "htm",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "php",
    "py",
    "sh",
    "bash",
    "zsh",
    "bat",
    "cmd",
    "ps1",
    "exe",
    "dll",
    "msi",
    "apk",
    "jar",
    "war",
    "com",
    "scr",
}
BLOCKED_CONTENT_TYPES = {
    "text/html",
    "application/xhtml+xml",
    "application/javascript",
    "text/javascript",
    "application/x-javascript",
    "application/x-msdownload",
    "application/x-msdos-program",
    "application/x-sh",
    "application/x-php",
}
SAFE_EXTENSION_RE = re.compile(r"^[a-z0-9]{1,16}$")


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
    s3_endpoint: str
    s3_bucket: str
    s3_access_key_id: str
    s3_secret_access_key: str
    s3_region: str
    hi168_s3_endpoint: str
    hi168_s3_bucket: str
    hi168_s3_access_key_id: str
    hi168_s3_secret_access_key: str
    hi168_s3_region: str
    aliyun_oss_endpoint: str
    aliyun_oss_bucket: str
    aliyun_oss_access_key_id: str
    aliyun_oss_secret_access_key: str
    aliyun_oss_region: str


def _normalize_text(value: Optional[str]) -> str:
    return str(value or "").strip()


def _parse_boolean(value: Optional[str], default: bool) -> bool:
    text = _normalize_text(value).lower()
    if not text:
        return default
    return text in {"1", "true", "yes", "on"}


def _normalize_provider(value: Optional[str]) -> StorageProvider:
    text = _normalize_text(value).lower()
    if text in {"local", "r2", "s3", "aliyun_oss", "hi168_s3"}:
        return text  # type: ignore[return-value]
    if text == "oss":
        return "s3"
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


def _read_setting_with_fallback(setting_map: dict[str, str], primary: str, *fallback_keys: str) -> str:
    text = _normalize_text(setting_map.get(primary))
    if text:
        return text

    for key in fallback_keys:
        text = _normalize_text(setting_map.get(key))
        if text:
            return text
    return ""


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
        s3_endpoint=_normalize_endpoint(
            _read_setting_with_fallback(
                setting_map,
                STORAGE_S3_ENDPOINT_KEY,
                LEGACY_STORAGE_OSS_ENDPOINT_KEY,
            ),
        ),
        s3_bucket=_normalize_bucket(
            _read_setting_with_fallback(
                setting_map,
                STORAGE_S3_BUCKET_KEY,
                LEGACY_STORAGE_OSS_BUCKET_KEY,
            ),
        ),
        s3_access_key_id=_normalize_text(
            _read_setting_with_fallback(
                setting_map,
                STORAGE_S3_ACCESS_KEY_ID_KEY,
                LEGACY_STORAGE_OSS_ACCESS_KEY_ID_KEY,
            ),
        ),
        s3_secret_access_key=_normalize_text(
            _read_setting_with_fallback(
                setting_map,
                STORAGE_S3_SECRET_ACCESS_KEY_KEY,
                LEGACY_STORAGE_OSS_SECRET_ACCESS_KEY_KEY,
            ),
        ),
        s3_region=_normalize_text(setting_map.get(STORAGE_S3_REGION_KEY, "")),
        hi168_s3_endpoint=_normalize_endpoint(setting_map.get(STORAGE_HI168_S3_ENDPOINT_KEY, "")),
        hi168_s3_bucket=_normalize_bucket(setting_map.get(STORAGE_HI168_S3_BUCKET_KEY, "")),
        hi168_s3_access_key_id=_normalize_text(setting_map.get(STORAGE_HI168_S3_ACCESS_KEY_ID_KEY, "")),
        hi168_s3_secret_access_key=_normalize_text(setting_map.get(STORAGE_HI168_S3_SECRET_ACCESS_KEY_KEY, "")),
        hi168_s3_region=_normalize_text(setting_map.get(STORAGE_HI168_S3_REGION_KEY, "")),
        aliyun_oss_endpoint=_normalize_endpoint(setting_map.get(STORAGE_ALIYUN_OSS_ENDPOINT_KEY, "")),
        aliyun_oss_bucket=_normalize_bucket(setting_map.get(STORAGE_ALIYUN_OSS_BUCKET_KEY, "")),
        aliyun_oss_access_key_id=_normalize_text(setting_map.get(STORAGE_ALIYUN_OSS_ACCESS_KEY_ID_KEY, "")),
        aliyun_oss_secret_access_key=_normalize_text(setting_map.get(STORAGE_ALIYUN_OSS_SECRET_ACCESS_KEY_KEY, "")),
        aliyun_oss_region=_normalize_text(setting_map.get(STORAGE_ALIYUN_OSS_REGION_KEY, "")),
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


def _normalize_extension(ext: str) -> str:
    normalized = ext.lower().lstrip(".").strip()
    if not normalized:
        return ""
    if not SAFE_EXTENSION_RE.match(normalized):
        return ""
    if normalized == "jpeg":
        return "jpg"
    return normalized


def _guess_file_extension(file_name: str, content_type: str) -> str:
    ext = _normalize_extension(Path(file_name).suffix)
    if ext:
        return ext

    mapped = MIME_TO_EXT.get(content_type.lower())
    if mapped:
        return mapped

    return "bin"


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


def _validate_media_file(file_name: str, content_type: str, content: bytes) -> None:
    if not content:
        raise ValueError("上传文件为空")
    if len(content) > MAX_MEDIA_SIZE_BYTES:
        raise ValueError("文件大小不能超过 200MB")

    normalized_content_type = _normalize_text(content_type).lower()
    if normalized_content_type in BLOCKED_CONTENT_TYPES:
        raise ValueError("不支持该文件类型上传")

    extension = _normalize_extension(Path(file_name).suffix)
    if extension in BLOCKED_FILE_EXTENSIONS:
        raise ValueError("不支持该文件类型上传")


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
        raise ValueError("Cloudflare R2 配置不完整")

    try:
        import boto3
    except Exception as error:  # pragma: no cover - runtime dependency
        raise RuntimeError("缺少 boto3 依赖，无法上传到 Cloudflare R2") from error

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


def _infer_region_from_endpoint(endpoint: str, *, fallback: str = "us-east-1") -> str:
    parsed = urlparse(endpoint)
    host = (parsed.netloc or parsed.path or "").lower()
    # Common endpoint formats:
    # - oss-cn-hangzhou.aliyuncs.com
    # - s3.cn-hangzhou.aliyuncs.com
    # - s3.ap-shanghai.myqcloud.com
    # - s3.us-west-004.backblazeb2.com
    match = re.search(r"oss-([a-z0-9-]+)\.", host)
    if match:
        candidate = match.group(1)
        # Avoid treating plain host segments (e.g. "hi168") as region.
        if "-" in candidate:
            return candidate

    match = re.search(r"s3[.-]([a-z0-9-]+)\.", host)
    if match:
        candidate = match.group(1)
        # Common S3-compatible regions usually contain dashes, e.g. ap-guangzhou/us-east-1.
        if "-" in candidate:
            return candidate
    return fallback


def _build_s3_default_public_url(endpoint: str, bucket: str, object_key: str, *, virtual_hosted: bool) -> str:
    parsed = urlparse(endpoint)
    scheme = parsed.scheme or "https"
    host = parsed.netloc or parsed.path
    if not host:
        return _join_public_url(f"{endpoint.rstrip('/')}/{bucket}", object_key)
    if virtual_hosted:
        return _join_public_url(f"{scheme}://{bucket}.{host}", object_key)
    return _join_public_url(f"{scheme}://{host}/{bucket}", object_key)


def _upload_s3_compatible_file(
    *,
    provider_label: str,
    endpoint: str,
    bucket: str,
    access_key_id: str,
    secret_access_key: str,
    region: str,
    object_key: str,
    content_type: str,
    content: bytes,
    public_base_url: str,
    default_virtual_hosted_url: bool,
    allow_virtual_hosted_retry: bool = True,
    allow_signature_v2_retry: bool = False,
) -> str:
    try:
        import boto3
        from botocore.config import Config as BotoCoreConfig
    except Exception as error:  # pragma: no cover - runtime dependency
        raise RuntimeError(f"缺少 boto3 依赖，无法上传到 {provider_label}") from error

    resolved_region = _normalize_text(region) or _infer_region_from_endpoint(endpoint)
    put_kwargs = {
        "Bucket": bucket,
        "Key": object_key,
        "Body": content,
    }
    if content_type:
        put_kwargs["ContentType"] = content_type

    put_kwargs["ContentLength"] = len(content)

    attempts = [
        ("s3v4", "path", False, True),
        ("s3v4", "path", True, True),
        ("s3v4", "path", False, False),
        ("s3v4", "path", True, False),
    ]
    if allow_virtual_hosted_retry:
        attempts.extend([
            ("s3v4", "virtual", False, True),
            ("s3v4", "virtual", True, True),
            ("s3v4", "virtual", False, False),
            ("s3v4", "virtual", True, False),
        ])
    if allow_signature_v2_retry:
        attempts.extend([
            ("s3", "path", False, True),
            ("s3", "path", False, False),
        ])
        if allow_virtual_hosted_retry:
            attempts.extend([
                ("s3", "virtual", False, True),
                ("s3", "virtual", False, False),
            ])

    last_error: Exception | None = None
    for signature_version, addressing_style, payload_signing_enabled, checksum_compat in attempts:
        try:
            config_kwargs: dict[str, object] = {
                "signature_version": signature_version,
                "s3": {
                    "addressing_style": addressing_style,
                    "payload_signing_enabled": payload_signing_enabled,
                },
            }
            if checksum_compat:
                # Some S3-compatible providers fail when SDK sends optional checksum-related headers.
                config_kwargs["request_checksum_calculation"] = "when_required"
                config_kwargs["response_checksum_validation"] = "when_required"
            try:
                client_config = BotoCoreConfig(**config_kwargs)
            except TypeError:
                # Backward compatibility for older botocore versions lacking checksum config options.
                config_kwargs.pop("request_checksum_calculation", None)
                config_kwargs.pop("response_checksum_validation", None)
                client_config = BotoCoreConfig(**config_kwargs)

            client = boto3.client(
                "s3",
                endpoint_url=endpoint,
                aws_access_key_id=access_key_id,
                aws_secret_access_key=secret_access_key,
                region_name=resolved_region,
                config=client_config,
            )
            client.put_object(**put_kwargs)
            break
        except Exception as error:  # pragma: no cover - runtime behavior depends on provider
            last_error = error
    else:
        message = str(last_error) if last_error else "未知错误"
        raise RuntimeError(f"{provider_label} 上传失败: {message}") from last_error

    if public_base_url:
        return _join_public_url(public_base_url, object_key)
    return _build_s3_default_public_url(
        endpoint=endpoint,
        bucket=bucket,
        object_key=object_key,
        virtual_hosted=default_virtual_hosted_url,
    )


def _upload_s3_file(
    config: ObjectStorageConfig,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if not config.s3_endpoint or not config.s3_bucket or not config.s3_access_key_id or not config.s3_secret_access_key:
        raise ValueError("S3对象存储配置不完整")

    return _upload_s3_compatible_file(
        provider_label="S3对象存储",
        endpoint=config.s3_endpoint,
        bucket=config.s3_bucket,
        access_key_id=config.s3_access_key_id,
        secret_access_key=config.s3_secret_access_key,
        region=config.s3_region,
        object_key=object_key,
        content_type=content_type,
        content=content,
        public_base_url=config.public_base_url,
        default_virtual_hosted_url=False,
        allow_virtual_hosted_retry=True,
    )


def _upload_hi168_s3_file(
    config: ObjectStorageConfig,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if (
        not config.hi168_s3_endpoint
        or not config.hi168_s3_bucket
        or not config.hi168_s3_access_key_id
        or not config.hi168_s3_secret_access_key
    ):
        raise ValueError("HI168 S3 配置不完整")

    region_candidates: list[str] = []
    normalized_region = _normalize_text(config.hi168_s3_region)
    if normalized_region and normalized_region.lower() != "auto":
        region_candidates.append(normalized_region)
    if "us-east-1" not in {item.lower() for item in region_candidates}:
        region_candidates.append("us-east-1")

    last_error: Exception | None = None
    for region in region_candidates:
        try:
            return _upload_s3_compatible_file(
                provider_label="HI168 S3",
                endpoint=config.hi168_s3_endpoint,
                bucket=config.hi168_s3_bucket,
                access_key_id=config.hi168_s3_access_key_id,
                secret_access_key=config.hi168_s3_secret_access_key,
                region=region,
                object_key=object_key,
                content_type=content_type,
                content=content,
                public_base_url=config.public_base_url,
                default_virtual_hosted_url=False,
                allow_virtual_hosted_retry=False,
                allow_signature_v2_retry=True,
            )
        except Exception as error:  # pragma: no cover - runtime behavior depends on provider
            last_error = error

    if last_error is None:
        raise RuntimeError("HI168 S3 上传失败: 未知错误")
    raise last_error


def _upload_aliyun_oss_file(
    config: ObjectStorageConfig,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if (
        not config.aliyun_oss_endpoint
        or not config.aliyun_oss_bucket
        or not config.aliyun_oss_access_key_id
        or not config.aliyun_oss_secret_access_key
    ):
        raise ValueError("阿里云 OSS 配置不完整")

    return _upload_s3_compatible_file(
        provider_label="阿里云 OSS",
        endpoint=config.aliyun_oss_endpoint,
        bucket=config.aliyun_oss_bucket,
        access_key_id=config.aliyun_oss_access_key_id,
        secret_access_key=config.aliyun_oss_secret_access_key,
        region=config.aliyun_oss_region,
        object_key=object_key,
        content_type=content_type,
        content=content,
        public_base_url=config.public_base_url,
        default_virtual_hosted_url=True,
        allow_virtual_hosted_retry=True,
    )


def _upload_object_to_provider(
    config: ObjectStorageConfig,
    *,
    object_key: str,
    content_type: str,
    content: bytes,
) -> str:
    if config.provider == "local":
        url = _upload_local_file(config, object_key=object_key, content=content)
    elif config.provider == "r2":
        url = _upload_r2_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )
    elif config.provider == "s3":
        url = _upload_s3_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )
    elif config.provider == "hi168_s3":
        url = _upload_hi168_s3_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )
    else:
        url = _upload_aliyun_oss_file(
            config,
            object_key=object_key,
            content_type=content_type,
            content=content,
        )
    return url


def upload_image_to_object_storage(
    session: Session,
    *,
    file_name: str,
    content_type: str,
    content: bytes,
) -> dict[str, str]:
    config = get_object_storage_config(session)
    if not config.enabled:
        raise ValueError("存储设置未启用")

    _validate_image_file(file_name, content_type, content)
    object_key = _build_object_key(config.local_path_rule, file_name=file_name, content_type=content_type)
    url = _upload_object_to_provider(
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


def upload_file_to_object_storage(
    session: Session,
    *,
    file_name: str,
    content_type: str,
    content: bytes,
) -> dict[str, str]:
    config = get_object_storage_config(session)
    if not config.enabled:
        raise ValueError("存储设置未启用")

    _validate_media_file(file_name, content_type, content)
    object_key = _build_object_key(config.local_path_rule, file_name=file_name, content_type=content_type)
    url = _upload_object_to_provider(
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
