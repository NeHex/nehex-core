from __future__ import annotations

import copy
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
from app.schemas.setting import SettingItem, ThemeSettingData

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
REI_THEME_FILE = "rei.json"
THEME_ACTIVE_PROFILE_KEY = "theme_active_profile"
THEME_PROFILES_KEY = "theme_profiles"


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


def _to_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value.strip()
    if isinstance(value, (int, float, bool)):
        return str(value).strip()

    try:
        return json.dumps(value, ensure_ascii=False)
    except (TypeError, ValueError):
        return str(value).strip()


def _normalize_theme_file_name(raw: str) -> str:
    text = raw.strip()
    if not text:
        return ""
    if "/" in text or "\\" in text:
        return ""
    if "." not in text:
        return f"{text}.json"
    return text


def _parse_unknown_json(raw: Any) -> Any:
    if isinstance(raw, str):
        text = raw.strip()
        if not text:
            return None
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return raw
    return raw


def _clone_object(value: dict[str, Any]) -> dict[str, Any]:
    return copy.deepcopy(value)


def _build_rei_theme_default() -> dict[str, Any]:
    return {
        "background_images": "/images/background-2k.png",
        "headmsg": "hi",
        "social_link": {
            "github": "https://github.com/nehex",
            "bilibili": "https://space.bilibili.com",
            "steam": "https://steampowered.com",
            "music": "https://music.163.com",
            "mail": "mailto:i@uegee.com",
            "feed": True,
        },
    }


def _read_setting_text(settings_map: dict[str, SettingItem], key: str) -> str:
    item = settings_map.get(key)
    if item is None:
        return ""
    return _to_text(item.setting_content)


def _build_legacy_theme_default(settings_map: dict[str, SettingItem]) -> dict[str, Any]:
    background = _read_setting_text(settings_map, "theme_background")
    primary = _read_setting_text(settings_map, "theme_primary")
    banner = _read_setting_text(settings_map, "theme_banner")
    card_style = _read_setting_text(settings_map, "theme_card_style")

    return {
        "background_images": background,
        "background": background,
        "primary": primary,
        "banner": banner,
        "card_style": card_style,
    }


def _parse_theme_profiles(raw: Any) -> dict[str, dict[str, Any]]:
    parsed = _parse_unknown_json(raw)
    if not isinstance(parsed, dict):
        return {}

    root = parsed
    themes = root.get("themes")
    source = themes if isinstance(themes, dict) else root

    result: dict[str, dict[str, Any]] = {}
    for raw_file, raw_content in source.items():
        normalized = _normalize_theme_file_name(str(raw_file))
        if not normalized:
            continue
        if not isinstance(raw_content, dict):
            continue
        result[normalized] = _clone_object(raw_content)
    return result


def list_theme_settings(session: Session) -> ThemeSettingData:
    items = list_settings(session)
    settings_map = {item.setting_key: item for item in items}
    legacy_default = _build_legacy_theme_default(settings_map)
    rei_default = _build_rei_theme_default()

    raw_profiles = settings_map.get(THEME_PROFILES_KEY)
    profiles = _parse_theme_profiles(raw_profiles.setting_content if raw_profiles else None)
    if not profiles:
        profiles = {
            REI_THEME_FILE: _clone_object(rei_default),
        }
    else:
        profiles.setdefault(REI_THEME_FILE, _clone_object(rei_default))
        merged_rei = _clone_object(rei_default)
        merged_rei.update(profiles.get(REI_THEME_FILE, {}))
        profiles[REI_THEME_FILE] = merged_rei

    if not profiles.get(REI_THEME_FILE):
        profiles[REI_THEME_FILE] = _clone_object(rei_default)

    for profile in profiles.values():
        if "background_images" not in profile:
            profile_background = _to_text(profile.get("background"))
            profile["background_images"] = profile_background or _to_text(legacy_default.get("background_images"))

    active_setting = settings_map.get(THEME_ACTIVE_PROFILE_KEY)
    active_profile = _normalize_theme_file_name(
        _to_text(active_setting.setting_content if active_setting else REI_THEME_FILE),
    )
    if not active_profile or active_profile not in profiles:
        active_profile = REI_THEME_FILE if REI_THEME_FILE in profiles else next(iter(profiles.keys()))

    current = _clone_object(profiles.get(active_profile, {}))

    if REI_THEME_FILE in profiles:
        ordered_profiles = {REI_THEME_FILE: _clone_object(profiles[REI_THEME_FILE])}
        for file, content in profiles.items():
            if file == REI_THEME_FILE:
                continue
            ordered_profiles[file] = _clone_object(content)
        profiles = ordered_profiles

    return ThemeSettingData(
        active_profile=active_profile,
        profiles=profiles,
        current=current,
    )


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
