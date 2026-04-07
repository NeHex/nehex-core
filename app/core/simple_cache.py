from __future__ import annotations

import base64
import dataclasses
from datetime import date, datetime, time
from decimal import Decimal
import importlib
import json
import logging
import threading
import time
from typing import Any

from app.core.config import settings

try:
    from pydantic import BaseModel
except Exception:  # pragma: no cover - fallback for environments without pydantic
    BaseModel = None  # type: ignore[assignment]

try:
    import redis
    from redis.exceptions import RedisError
except Exception:  # pragma: no cover - fallback path for missing optional dependency
    redis = None

    class RedisError(Exception):
        pass

logger = logging.getLogger(__name__)


class TTLCache:
    def __init__(self, max_entries: int = 1024) -> None:
        self._lock = threading.Lock()
        self._store: dict[str, tuple[float, Any]] = {}
        self._max_entries = max(1, int(max_entries))

    def _purge_expired_locked(self, now: float) -> None:
        expired_keys = [
            key
            for key, (expires_at, _) in self._store.items()
            if expires_at <= now
        ]
        for key in expired_keys:
            self._store.pop(key, None)

    def _evict_one_locked(self) -> None:
        if not self._store:
            return
        evict_key = min(self._store.items(), key=lambda item: item[1][0])[0]
        self._store.pop(evict_key, None)

    def get(self, key: str) -> Any | None:
        now = time.monotonic()
        with self._lock:
            value = self._store.get(key)
            if value is None:
                return None

            expires_at, payload = value
            if expires_at <= now:
                self._store.pop(key, None)
                return None

            return payload

    def set(self, key: str, payload: Any, ttl_seconds: int) -> Any:
        now = time.monotonic()
        expires_at = now + max(1, int(ttl_seconds))
        with self._lock:
            self._purge_expired_locked(now)
            if key not in self._store and len(self._store) >= self._max_entries:
                self._evict_one_locked()
            self._store[key] = (expires_at, payload)
        return payload

    def delete(self, key: str) -> None:
        with self._lock:
            self._store.pop(key, None)

    def delete_prefix(self, prefix: str) -> None:
        with self._lock:
            keys = [key for key in self._store if key.startswith(prefix)]
            for key in keys:
                self._store.pop(key, None)

    def clear(self) -> None:
        with self._lock:
            self._store.clear()


class RedisBackedTTLCache:
    def __init__(
        self,
        max_entries: int = 1024,
        prefix: str = "nehex:cache:",
        retry_seconds: int = 30,
    ) -> None:
        self._fallback = TTLCache(max_entries=max_entries)
        self._prefix = prefix
        self._retry_seconds = max(5, int(retry_seconds))
        self._lock = threading.Lock()
        self._client: Any | None = None
        self._next_connect_retry_at = 0.0
        self._last_error_log_at = 0.0
        self._missing_dependency_logged = False
        self._resolved_class_cache: dict[str, type[Any] | None] = {}

    _SERIALIZED_TYPE_KEY = "$nehex_cache_type"
    _SERIALIZED_CLASS_KEY = "$nehex_cache_class"
    _SERIALIZED_DATA_KEY = "$nehex_cache_data"

    def _is_allowed_cache_class(self, class_path: str) -> bool:
        if class_path == "app.services.install_service.InstallStatus":
            return True
        return class_path.startswith("app.schemas.")

    def _resolve_cache_class(self, class_path: str) -> type[Any] | None:
        cached = self._resolved_class_cache.get(class_path)
        if class_path in self._resolved_class_cache:
            return cached

        if not self._is_allowed_cache_class(class_path):
            self._resolved_class_cache[class_path] = None
            return None

        module_name, _, class_name = class_path.rpartition(".")
        if not module_name or not class_name:
            self._resolved_class_cache[class_path] = None
            return None

        try:
            module = importlib.import_module(module_name)
            target: Any = module
            for part in class_name.split("."):
                target = getattr(target, part, None)
                if target is None:
                    break
            if isinstance(target, type):
                self._resolved_class_cache[class_path] = target
                return target
        except Exception:
            pass

        self._resolved_class_cache[class_path] = None
        return None

    def _serialize_value(self, value: Any) -> Any:
        if value is None or isinstance(value, (str, int, float, bool)):
            return value

        if isinstance(value, datetime):
            return {
                self._SERIALIZED_TYPE_KEY: "datetime",
                self._SERIALIZED_DATA_KEY: value.isoformat(),
            }
        if isinstance(value, date):
            return {
                self._SERIALIZED_TYPE_KEY: "date",
                self._SERIALIZED_DATA_KEY: value.isoformat(),
            }
        if isinstance(value, time):
            return {
                self._SERIALIZED_TYPE_KEY: "time",
                self._SERIALIZED_DATA_KEY: value.isoformat(),
            }
        if isinstance(value, Decimal):
            return {
                self._SERIALIZED_TYPE_KEY: "decimal",
                self._SERIALIZED_DATA_KEY: str(value),
            }
        if isinstance(value, bytes):
            return {
                self._SERIALIZED_TYPE_KEY: "bytes",
                self._SERIALIZED_DATA_KEY: base64.b64encode(value).decode("ascii"),
            }

        if BaseModel is not None and isinstance(value, BaseModel):
            return {
                self._SERIALIZED_TYPE_KEY: "pydantic",
                self._SERIALIZED_CLASS_KEY: f"{value.__class__.__module__}.{value.__class__.__qualname__}",
                self._SERIALIZED_DATA_KEY: self._serialize_value(value.model_dump(mode="json")),
            }

        if dataclasses.is_dataclass(value) and not isinstance(value, type):
            return {
                self._SERIALIZED_TYPE_KEY: "dataclass",
                self._SERIALIZED_CLASS_KEY: f"{value.__class__.__module__}.{value.__class__.__qualname__}",
                self._SERIALIZED_DATA_KEY: self._serialize_value(dataclasses.asdict(value)),
            }

        if isinstance(value, dict):
            return {str(key): self._serialize_value(item) for key, item in value.items()}
        if isinstance(value, (list, tuple, set)):
            return [self._serialize_value(item) for item in value]

        return str(value)

    def _deserialize_value(self, value: Any) -> Any:
        if isinstance(value, list):
            return [self._deserialize_value(item) for item in value]

        if not isinstance(value, dict):
            return value

        marker = value.get(self._SERIALIZED_TYPE_KEY)
        raw = value.get(self._SERIALIZED_DATA_KEY)
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

        if marker in {"pydantic", "dataclass"}:
            class_path = str(value.get(self._SERIALIZED_CLASS_KEY) or "").strip()
            decoded_data = self._deserialize_value(raw)
            model_cls = self._resolve_cache_class(class_path)
            if model_cls is None:
                return decoded_data
            try:
                if marker == "pydantic" and hasattr(model_cls, "model_validate"):
                    return model_cls.model_validate(decoded_data)
                if marker == "dataclass" and isinstance(decoded_data, dict):
                    return model_cls(**decoded_data)
            except Exception:
                return decoded_data
            return decoded_data

        return {str(key): self._deserialize_value(item) for key, item in value.items()}

    def _serialize_payload(self, payload: Any) -> bytes:
        serialized = self._serialize_value(payload)
        return json.dumps(serialized, ensure_ascii=False, separators=(",", ":")).encode("utf-8")

    def _deserialize_payload(self, payload: bytes) -> Any:
        decoded = json.loads(payload.decode("utf-8"))
        return self._deserialize_value(decoded)

    def _redis_key(self, key: str) -> str:
        return f"{self._prefix}{key}"

    def _log_error(self, message: str) -> None:
        now = time.monotonic()
        if now - self._last_error_log_at >= 30:
            logger.warning(message)
            self._last_error_log_at = now

    def _mark_client_failed(self, error: Exception) -> None:
        with self._lock:
            self._client = None
            self._next_connect_retry_at = time.monotonic() + self._retry_seconds
        self._log_error(f"[cache] redis unavailable, fallback to in-memory cache: {error}")

    def _get_client(self) -> Any | None:
        now = time.monotonic()
        with self._lock:
            if self._client is not None:
                return self._client
            if now < self._next_connect_retry_at:
                return None

            if redis is None:
                self._next_connect_retry_at = now + self._retry_seconds
                if not self._missing_dependency_logged:
                    logger.warning("[cache] redis dependency missing, fallback to in-memory cache.")
                    self._missing_dependency_logged = True
                return None

            try:
                client = redis.Redis.from_url(
                    settings.redis_url,
                    decode_responses=False,
                    socket_connect_timeout=float(settings.redis_socket_connect_timeout),
                    socket_timeout=float(settings.redis_socket_timeout),
                    health_check_interval=30,
                    retry_on_timeout=True,
                )
                client.ping()
                self._client = client
                self._next_connect_retry_at = 0.0
                logger.info("[cache] redis cache enabled.")
                return client
            except Exception as error:
                self._client = None
                self._next_connect_retry_at = now + self._retry_seconds
                self._log_error(f"[cache] redis unavailable, fallback to in-memory cache: {error}")
                return None

    def _delete_by_pattern(self, client: Any, pattern: str) -> None:
        pipeline = client.pipeline(transaction=False)
        pending = 0
        for redis_key in client.scan_iter(match=pattern, count=200):
            pipeline.delete(redis_key)
            pending += 1
            if pending >= 200:
                pipeline.execute()
                pending = 0
        if pending > 0:
            pipeline.execute()

    def get(self, key: str) -> Any | None:
        client = self._get_client()
        if client is None:
            return self._fallback.get(key)

        try:
            payload = client.get(self._redis_key(key))
            if payload is None:
                return None
            return self._deserialize_payload(payload)
        except Exception as error:
            try:
                client.delete(self._redis_key(key))
            except Exception:
                self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))
            return self._fallback.get(key)

    def set(self, key: str, payload: Any, ttl_seconds: int) -> Any:
        ttl = max(1, int(ttl_seconds))
        client = self._get_client()
        if client is None:
            return self._fallback.set(key, payload, ttl)

        try:
            serialized = self._serialize_payload(payload)
            client.setex(self._redis_key(key), ttl, serialized)
            self._fallback.delete(key)
            return payload
        except Exception as error:
            self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))
            return self._fallback.set(key, payload, ttl)

    def delete(self, key: str) -> None:
        self._fallback.delete(key)
        client = self._get_client()
        if client is None:
            return
        try:
            client.delete(self._redis_key(key))
        except Exception as error:
            self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))

    def delete_prefix(self, prefix: str) -> None:
        self._fallback.delete_prefix(prefix)
        client = self._get_client()
        if client is None:
            return
        try:
            self._delete_by_pattern(client, f"{self._redis_key(prefix)}*")
        except Exception as error:
            self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))

    def clear(self) -> None:
        self._fallback.clear()
        client = self._get_client()
        if client is None:
            return
        try:
            self._delete_by_pattern(client, f"{self._prefix}*")
        except Exception as error:
            self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))

    def close(self) -> None:
        with self._lock:
            client = self._client
            self._client = None
        if client is None:
            return
        try:
            client.close()
        except RedisError:
            pass


def _build_cache() -> Any:
    if settings.redis_enabled:
        return RedisBackedTTLCache(
            max_entries=settings.simple_cache_max_entries,
            prefix=settings.redis_cache_prefix,
            retry_seconds=settings.redis_connect_retry_seconds,
        )
    return TTLCache(max_entries=settings.simple_cache_max_entries)


cache = _build_cache()


def close_cache() -> None:
    closer = getattr(cache, "close", None)
    if callable(closer):
        closer()
