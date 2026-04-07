from __future__ import annotations

from fastapi import APIRouter, WebSocket, WebSocketDisconnect

from app.services.online_presence_service import (
    normalize_online_page_key,
    online_presence_hub,
)

router = APIRouter()


@router.websocket("/ws/online")
async def online_presence_websocket(websocket: WebSocket) -> None:
    page_key = normalize_online_page_key(
        websocket.query_params.get("page_path"),
        websocket.query_params.get("page_full"),
    )

    await online_presence_hub.connect(websocket, page_key=page_key)

    try:
        while True:
            # Keep the connection open and rely on disconnect exceptions for cleanup.
            await websocket.receive()
    except WebSocketDisconnect:
        pass
    except Exception:
        pass
    finally:
        await online_presence_hub.disconnect(websocket, page_key=page_key)

