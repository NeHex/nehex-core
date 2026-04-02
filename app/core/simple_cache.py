from __future__ import annotations

import threading
import time
from typing import Any


class TTLCache:
    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._store: dict[str, tuple[float, Any]] = {}

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
        expires_at = time.monotonic() + max(1, int(ttl_seconds))
        with self._lock:
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


cache = TTLCache()
