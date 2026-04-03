from __future__ import annotations

import base64
import hashlib
import json
import secrets
import time
from dataclasses import dataclass
from typing import Optional

from cryptography.fernet import Fernet, InvalidToken
from fastapi import Header, HTTPException, Request, status

from app.core.config import settings

ADMIN_TOKEN_COOKIE_KEY = "nehex_admin_token"


def _build_fernet(secret: str) -> Fernet:
    key_seed = hashlib.sha256(secret.encode("utf-8")).digest()
    key = base64.urlsafe_b64encode(key_seed)
    return Fernet(key)


_TOKEN_FERNET = _build_fernet(settings.admin_api_secret)


@dataclass(frozen=True)
class AdminPrincipal:
    account: str
    expires_at: int
    token_id: str


def double_sha256(value: str) -> str:
    first = hashlib.sha256(value.encode("utf-8")).hexdigest()
    return hashlib.sha256(first.encode("utf-8")).hexdigest()


def create_admin_token(account: str) -> tuple[str, int]:
    now = int(time.time())
    expires_at = now + max(300, int(settings.admin_api_token_ttl_seconds))
    payload = {
        "account": account,
        "client": settings.admin_api_client_id,
        "exp": expires_at,
        "iat": now,
        "jti": secrets.token_urlsafe(16),
    }
    token = _TOKEN_FERNET.encrypt(
        json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("utf-8"),
    ).decode("utf-8")
    return token, expires_at


def decode_admin_token(token: str) -> AdminPrincipal:
    try:
        raw = _TOKEN_FERNET.decrypt(token.encode("utf-8"))
    except InvalidToken as error:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid admin token",
        ) from error

    try:
        payload = json.loads(raw.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid admin token payload",
        ) from error

    account = str(payload.get("account") or "").strip()
    token_client = str(payload.get("client") or "").strip()
    token_id = str(payload.get("jti") or "").strip()
    expires_at = int(payload.get("exp") or 0)
    now = int(time.time())

    if not account or not token_id:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid admin token fields",
        )

    if token_client != settings.admin_api_client_id:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid admin token client",
        )

    if expires_at <= now:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Admin token expired",
        )

    return AdminPrincipal(account=account, expires_at=expires_at, token_id=token_id)


def _extract_bearer_token(authorization: Optional[str]) -> Optional[str]:
    if not authorization:
        return None

    scheme, _, token = authorization.partition(" ")
    if scheme.lower() != "bearer" or not token.strip():
        return None
    return token.strip()


def require_admin_principal(
    request: Request,
    authorization: Optional[str] = Header(default=None, alias="Authorization"),
    admin_client: Optional[str] = Header(default=None, alias="X-NeHex-Admin-Client"),
) -> AdminPrincipal:
    if (admin_client or "").strip() != settings.admin_api_client_id:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin client is not allowed",
        )

    bearer_token = _extract_bearer_token(authorization)
    token = bearer_token or request.cookies.get(ADMIN_TOKEN_COOKIE_KEY)

    if not token:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing admin token",
        )

    return decode_admin_token(token)
