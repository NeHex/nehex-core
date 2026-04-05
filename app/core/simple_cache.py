from __future__ import annotations

import pickle
import threading
import time
from typing import Any

from app.core.config import settings

try:
    import redis
    from redis.exceptions import RedisError
except Exception:  # pragma: no cover - fallback path for missing optional dependency
    redis = None

    class RedisError(Exception):
        pass


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

    def _redis_key(self, key: str) -> str:
        return f"{self._prefix}{key}"

    def _log_error(self, message: str) -> None:
        now = time.monotonic()
        if now - self._last_error_log_at >= 30:
            print(message)
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
                    print("[cache] redis dependency missing, fallback to in-memory cache.")
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
                print("[cache] redis cache enabled.")
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
            return pickle.loads(payload)
        except Exception as error:
            self._mark_client_failed(error if isinstance(error, Exception) else Exception(str(error)))
            return self._fallback.get(key)

    def set(self, key: str, payload: Any, ttl_seconds: int) -> Any:
        ttl = max(1, int(ttl_seconds))
        client = self._get_client()
        if client is None:
            return self._fallback.set(key, payload, ttl)

        try:
            serialized = pickle.dumps(payload, protocol=pickle.HIGHEST_PROTOCOL)
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
