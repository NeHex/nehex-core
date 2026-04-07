from __future__ import annotations

import asyncio
from urllib.parse import urlsplit

from fastapi import WebSocket


def normalize_online_page_key(page_path: str | None, page_full: str | None) -> str:
    path = (page_path or "").strip()
    full = (page_full or "").strip()

    if not path and full:
        if "://" in full:
            path = (urlsplit(full).path or "").strip()
        else:
            normalized_full = full if full.startswith("/") else f"/{full}"
            path = (urlsplit(f"http://local{normalized_full}").path or "").strip()

    if not path:
        return "/"

    if "://" in path:
        path = (urlsplit(path).path or "").strip() or "/"
    else:
        normalized_path = path if path.startswith("/") else f"/{path}"
        path = (urlsplit(f"http://local{normalized_path}").path or "").strip() or "/"

    if not path.startswith("/"):
        path = f"/{path}"

    return path[:512]


class OnlinePresenceHub:
    def __init__(self) -> None:
        self._lock = asyncio.Lock()
        self._connections: dict[str, set[WebSocket]] = {}

    async def connect(self, websocket: WebSocket, *, page_key: str) -> None:
        await websocket.accept()

        async with self._lock:
            page_connections = self._connections.setdefault(page_key, set())
            page_connections.add(websocket)
            current_targets = list(page_connections)
            current_count = len(page_connections)

        await self._broadcast_online_count(
            page_key=page_key,
            count=current_count,
            targets=current_targets,
        )

    async def disconnect(self, websocket: WebSocket, *, page_key: str) -> None:
        async with self._lock:
            page_connections = self._connections.get(page_key)
            if not page_connections:
                return

            page_connections.discard(websocket)
            if not page_connections:
                self._connections.pop(page_key, None)
                return

            current_targets = list(page_connections)
            current_count = len(page_connections)

        await self._broadcast_online_count(
            page_key=page_key,
            count=current_count,
            targets=current_targets,
        )

    async def _broadcast_online_count(
        self,
        *,
        page_key: str,
        count: int,
        targets: list[WebSocket],
    ) -> None:
        stale_connections: list[WebSocket] = []
        payload = {"online": count}

        for ws in targets:
            try:
                await ws.send_json(payload)
            except Exception:
                stale_connections.append(ws)

        if not stale_connections:
            return

        async with self._lock:
            page_connections = self._connections.get(page_key)
            if not page_connections:
                return
            for ws in stale_connections:
                page_connections.discard(ws)
            if not page_connections:
                self._connections.pop(page_key, None)


online_presence_hub = OnlinePresenceHub()
