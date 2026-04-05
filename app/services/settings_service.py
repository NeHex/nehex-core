from __future__ import annotations

import json
from datetime import datetime
from typing import Any
from typing import Optional

from sqlalchemy import select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.core.database import database_table_exists
from app.core.simple_cache import cache
from app.models.setting import Setting, SettingType
from app.schemas.setting import SettingItem

SETTINGS_CACHE_KEY = "settings:list"
SETTINGS_CACHE_TTL_SECONDS = 60
PUBLIC_VISIBLE_SETTING_KEYS = {
    "site_title",
    "site_sub_title",
    "site_api_base",
    "site_description",
    "site_keywords",
    "site_icp",
    "site_notice",
    "site_url",
    "site_desc",
    "site_favicon",
    "theme_background",
    "theme_primary",
    "theme_banner",
    "theme_card_style",
    "theme_active_profile",
    "theme_profiles",
    "theme_nav",
    "nehex_article_class",
    "user_social_link",
}
COMPAT_SETTING_DEFAULTS: dict[str, tuple[SettingType, Any]] = {
    "site_title": (SettingType.string, ""),
    "site_desc": (SettingType.string, ""),
    "site_favicon": (SettingType.string, "/favicon.ico"),
    "site_url": (SettingType.string, ""),
    "theme_background": (SettingType.string, ""),
    "theme_nav": (SettingType.json, {}),
    "user_social_link": (SettingType.json, []),
}
COMPAT_SETTING_ALIASES: dict[str, str] = {
    "site_desc": "site_description",
}


def _is_public_setting_key(setting_key: str) -> bool:
    return setting_key in PUBLIC_VISIBLE_SETTING_KEYS


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

    try:
        if not database_table_exists("settings"):
            mapped = _with_compatibility_keys([])
            cache.set(SETTINGS_CACHE_KEY, mapped, SETTINGS_CACHE_TTL_SECONDS)
            return [item.model_copy(deep=True) for item in mapped]

        stmt = select(Setting).order_by(Setting.setting_key.asc())
        result = session.execute(stmt)
        rows = result.scalars().all()
        rows = [row for row in rows if _is_public_setting_key(row.setting_key)]
    except SQLAlchemyError:
        mapped = _with_compatibility_keys([])
        cache.set(SETTINGS_CACHE_KEY, mapped, SETTINGS_CACHE_TTL_SECONDS)
        return [item.model_copy(deep=True) for item in mapped]

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
    mapped = _with_compatibility_keys(mapped)
    cache.set(SETTINGS_CACHE_KEY, mapped, SETTINGS_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def _with_compatibility_keys(items: list[SettingItem]) -> list[SettingItem]:
    if not items:
        now = datetime.utcnow()
        return [
            SettingItem(
                setting_key=key,
                setting_type=setting_type,
                setting_content=default_content,
                description="compat default",
                updated_at=now,
                created_at=now,
            )
            for key, (setting_type, default_content) in COMPAT_SETTING_DEFAULTS.items()
        ]

    item_map: dict[str, SettingItem] = {item.setting_key: item for item in items}
    latest_updated_at = max((item.updated_at for item in items), default=datetime.utcnow())
    latest_created_at = min((item.created_at for item in items), default=latest_updated_at)

    for setting_key, (setting_type, default_content) in COMPAT_SETTING_DEFAULTS.items():
        if setting_key in item_map:
            continue

        alias_key = COMPAT_SETTING_ALIASES.get(setting_key)
        aliased_value = item_map.get(alias_key).setting_content if alias_key and alias_key in item_map else default_content
        item_map[setting_key] = SettingItem(
            setting_key=setting_key,
            setting_type=setting_type,
            setting_content=aliased_value,
            description="compat default",
            updated_at=latest_updated_at,
            created_at=latest_created_at,
        )

    return sorted(item_map.values(), key=lambda item: item.setting_key)
