from __future__ import annotations

import json
from typing import Any
from typing import Optional

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.setting import Setting, SettingType
from app.schemas.setting import SettingItem

SETTINGS_CACHE_KEY = "settings:list"
SETTINGS_CACHE_TTL_SECONDS = 60


def parse_setting_content(setting_type: SettingType, raw_content: Optional[str]) -> Any:
    if raw_content is None:
        return None

    try:
        if setting_type == SettingType.string:
            return raw_content
        if setting_type == SettingType.int:
            return int(raw_content)
        if setting_type == SettingType.float:
            return float(raw_content)
        if setting_type == SettingType.boolean:
            return raw_content.strip().lower() in {"1", "true", "yes", "on"}
        if setting_type == SettingType.json:
            return json.loads(raw_content)
    except (ValueError, json.JSONDecodeError):
        # 解析失败时回退原始字符串，避免接口直接报错
        return raw_content

    return raw_content


def list_settings(session: Session) -> list[SettingItem]:
    cached = cache.get(SETTINGS_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = select(Setting).order_by(Setting.setting_key.asc())
    result = session.execute(stmt)
    rows = result.scalars().all()

    mapped = [
        SettingItem(
            setting_key=row.setting_key,
            setting_type=row.setting_type,
            setting_content=parse_setting_content(row.setting_type, row.setting_content),
            description=row.description,
            updated_at=row.updated_at,
            created_at=row.created_at,
        )
        for row in rows
    ]
    cache.set(SETTINGS_CACHE_KEY, mapped, SETTINGS_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]
