from __future__ import annotations

import json
from typing import Any
from typing import Optional

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.core.admin_security import hash_admin_password
from app.core.simple_cache import cache
from app.models.setting import Setting, SettingType
from app.schemas.setting import SettingItem
from app.services.admin_service_parts.common import (
    SENSITIVE_ADMIN_SETTING_KEYS,
    _invalidate_settings_cache,
    _map_setting_item,
    _normalize_optional_text,
)


def get_admin_credentials(session: Session) -> tuple[str, str]:
    stmt = select(Setting).where(
        Setting.setting_key.in_(["user_account", "user_account_password"]),
    )
    rows = session.execute(stmt).scalars().all()
    setting_map = {row.setting_key: row.setting_content or "" for row in rows}
    account = str(setting_map.get("user_account", "")).strip()
    password_hash = str(setting_map.get("user_account_password", "")).strip().lower()
    return account, password_hash


def list_admin_settings(session: Session) -> list[SettingItem]:
    stmt = select(Setting).order_by(Setting.setting_key.asc())
    rows = session.execute(stmt).scalars().all()
    rows = [row for row in rows if row.setting_key not in SENSITIVE_ADMIN_SETTING_KEYS]
    return [_map_setting_item(row) for row in rows]


def _serialize_setting_content(setting_type: SettingType, value: Any) -> Optional[str]:
    if value is None:
        return None

    if setting_type == SettingType.string:
        return str(value)

    if setting_type == SettingType.int:
        return str(int(value))

    if setting_type == SettingType.float:
        return str(float(value))

    if setting_type == SettingType.boolean:
        if isinstance(value, bool):
            return "true" if value else "false"
        normalized = str(value).strip().lower()
        return "true" if normalized in {"1", "true", "yes", "on"} else "false"

    if setting_type == SettingType.json:
        if isinstance(value, str):
            text = value.strip()
            if not text:
                return None
            try:
                parsed = json.loads(text)
                return json.dumps(parsed, ensure_ascii=False)
            except json.JSONDecodeError:
                return text
        return json.dumps(value, ensure_ascii=False)

    return str(value)


def update_admin_settings(
    session: Session,
    items: list[dict[str, Any]],
) -> list[SettingItem]:
    should_invalidate_admin_path = False
    for item in items:
        setting_key = str(item.get("setting_key") or "").strip()
        if not setting_key:
            continue
        if setting_key == "admin_manager_web":
            should_invalidate_admin_path = True

        existing = session.get(Setting, setting_key)
        incoming_type = item.get("setting_type")
        setting_type = incoming_type if isinstance(incoming_type, SettingType) else None

        if existing is None:
            existing = Setting(
                setting_key=setting_key,
                setting_type=setting_type or SettingType.string,
            )
            session.add(existing)
        elif setting_type is not None:
            existing.setting_type = setting_type

        effective_type = existing.setting_type
        existing.setting_content = _serialize_setting_content(
            effective_type,
            item.get("setting_content"),
        )

        if bool(item.get("has_description")):
            existing.description = _normalize_optional_text(item.get("description"))

    session.commit()
    _invalidate_settings_cache()
    if should_invalidate_admin_path:
        cache.delete("admin:manager:web:path")
    return list_admin_settings(session)


def update_admin_account_settings(
    session: Session,
    *,
    account: Optional[str] = None,
    new_password: Optional[str] = None,
) -> list[SettingItem]:
    updates: list[dict[str, Any]] = []

    normalized_account = _normalize_optional_text(account)
    if normalized_account is not None:
        updates.append(
            {
                "setting_key": "user_account",
                "setting_type": SettingType.string,
                "setting_content": normalized_account,
                "has_description": False,
            },
        )

    normalized_new_password = _normalize_optional_text(new_password)

    if normalized_new_password is not None:
        hashed = hash_admin_password(normalized_new_password)
        updates.append(
            {
                "setting_key": "user_account_password",
                "setting_type": SettingType.string,
                "setting_content": hashed,
                "has_description": False,
            },
        )

    if not updates:
        return list_admin_settings(session)

    return update_admin_settings(session, updates)
