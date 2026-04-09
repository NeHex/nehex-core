from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime
from typing import Optional

from sqlalchemy import select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.core.admin_security import hash_admin_password
from app.core.config import normalize_admin_manager_web_path, settings
from app.core.database import database_table_exists, ensure_all_tables, list_database_tables
from app.core.simple_cache import cache
from app.models.article import Article
from app.models.singlepage import SinglePage
from app.models.setting import Setting, SettingType

INSTALL_STATUS_CACHE_KEY = "admin:install:status"
ADMIN_MANAGER_WEB_CACHE_KEY = "admin:manager:web:path"
_INSTALL_COMPLETED_KEY = "install_completed"
_ADMIN_MANAGER_WEB_SETTING_KEY = "admin_manager_web"


@dataclass(frozen=True)
class InstallStatus:
    installed: bool
    schema_ready: bool
    table_count: int
    admin_manager_web: str


def _normalize_optional_text(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    normalized = value.strip()
    return normalized or None


def _serialize_setting_content(setting_type: SettingType, value: object) -> Optional[str]:
    if value is None:
        return None

    if setting_type == SettingType.string:
        return str(value)

    if setting_type == SettingType.boolean:
        if isinstance(value, bool):
            return "true" if value else "false"
        normalized = str(value).strip().lower()
        return "true" if normalized in {"1", "true", "yes", "on"} else "false"

    if setting_type == SettingType.int:
        return str(int(value))

    if setting_type == SettingType.float:
        return str(float(value))

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


def _parse_boolean(raw: object) -> bool:
    text = str(raw or "").strip().lower()
    return text in {"1", "true", "yes", "on"}


def _build_default_theme_profiles() -> dict[str, dict[str, object]]:
    return {
        "rei.json": {
            "head_pic": "/images/head.jpg",
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
            "nav_border": {
                "关于": "/about",
                "友链": "/friends",
                "游戏室": "/games",
            },
            "about_page": {
                "welcome": {
                    "text": "hi👋 我是",
                    "name": "UEGEE",
                    "desc": "是一个无业游民，一个穷孩子生活在有钱人的城市。",
                },
                "map": {
                    "天津": "117.200983, 39.084158",
                    "山东": "x118.000923, 36.675807",
                },
                "slogan": {
                    "text": "希望",
                    "main": "我的人生可以早点",
                    "more": [
                        "顺利",
                        "暴富",
                        "退休",
                    ],
                },
                "skills": {
                    "title": "创造,源于热爱",
                    "programlanguage": [
                        "python",
                        "vue",
                        "nuxt",
                        "docker",
                        "ubuntu",
                        "linux mint",
                        "mysql",
                        "redis",
                    ],
                },
                "education": {
                    "text": "好好学习,天天向上！————毛泽东",
                    "university": "山东曲阜师范大学",
                    "time": "2020/2023",
                },
                "visitor_data": {
                    "title": "访问数据",
                    "tips": "本站自主统计",
                },
                "hobby": [
                    "jk",
                    "computer",
                    "hardware",
                    "linux",
                ],
                "life_target": {
                    "text": "人生目标",
                    "target": {
                        "not_yet": [
                            "拥有一辆自己的汽车",
                            "有一份稳定的工作",
                            "拥有9950x3d",
                            "月均收入达8000",
                            "与爱人结婚",
                            "有一套属于自己的房子",
                            "MacBookPro",
                            "活到100岁",
                        ],
                        "finish": [
                            "建造属于自己的HomeLab",
                            "每年回一次老家2026",
                        ],
                    },
                },
                "wifes_card": {
                    "Aihara Enju": {
                        "cn_name": "蓝原延珠",
                        "other_name": "藍原（あいはら） 延珠（えんじゅ）",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Aihara_Enju-half.png",
                    },
                    "Alisa Mikhailovna Kujō": {
                        "cn_name": "艾莉莎·米哈伊羅芙娜·九條",
                        "other_name": "Алиса Михайловна Кудзё",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Alisa_Mikhaylovna_Kujō.png",
                    },
                    "Ijichi Nijika": {
                        "cn_name": "伊地知虹夏",
                        "other_name": "伊地知（いじち） 虹夏（にじか）",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/IjichiNijika-half.png",
                    },
                    "Perlica": {
                        "cn_name": "佩丽卡",
                        "other_name": "Perlica",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Perlica-half.png",
                    },
                    "Sento Isuzu": {
                        "cn_name": "千斗五十鈴",
                        "other_name": "Isuzuruha Centollusia",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Sento_Isuzu-half.png",
                    },
                    "Togawa Sakiko": {
                        "cn_name": "丰川祥子",
                        "other_name": "豊川（とがわ） 祥子（さきこ）",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Togawa Sakiko-top.png",
                    },
                    "Nao Tomori": {
                        "cn_name": "友利奈绪",
                        "other_name": "友利（ともり）  奈緒（なお）",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Tomori_Nao-half.png",
                    },
                    "Suō Yuki": {
                        "cn_name": "周防有希",
                        "other_name": "周防(すおう) 有希(ゆき)",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/Yuki_Suou_1.png",
                    },
                    "Takagi": {
                        "cn_name": "高木同学",
                        "other_name": "高木（たかぎ）",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/takagi3-half.png",
                    },
                    "Zhuangfangyi": {
                        "cn_name": "庄方宜",
                        "other_name": "ZhuangFangYi",
                        "image": "https://s3.hi168.com/hi168-31358-3621l8yj/wifes/zhuangfangyi.png",
                    },
                },
            },
        },
    }


def _seed_default_content(
    session: Session,
    *,
    site_title: str,
    primary_article_class: str,
) -> None:
    article_exists = session.execute(select(Article.id).limit(1)).scalars().first()
    if article_exists is None:
        session.add(
            Article(
                title=f"{site_title} 已完成初始化",
                article_class=primary_article_class,
                article_top_image=None,
                read_count=0,
                like_count=0,
                tag="公告,示例",
                top=1,
                status=1,
                content=(
                    "欢迎使用 NeHex。\n\n"
                    "这是系统在首次安装时自动创建的示例文章，你可以在后台管理中编辑或删除。"
                ),
            ),
        )

    page_exists = session.execute(select(SinglePage.id).limit(1)).scalars().first()
    if page_exists is None:
        session.add(
            SinglePage(
                page_key="about",
                title="关于本站",
                cover_image=None,
                content=(
                    f"# 关于 {site_title}\n\n"
                    "本站已完成首次安装。\n\n"
                    "这是系统自动创建的示例页面，你可以在后台管理中继续完善内容。"
                ),
                sort=0,
                status=1,
            ),
        )


def get_admin_manager_web_path() -> str:
    cached = cache.get(ADMIN_MANAGER_WEB_CACHE_KEY)
    if isinstance(cached, str) and cached:
        return cached

    admin_path = settings.admin_manager_web_path
    try:
        if not database_table_exists("settings"):
            cache.set(ADMIN_MANAGER_WEB_CACHE_KEY, admin_path, ttl_seconds=10)
            return admin_path
    except SQLAlchemyError:
        cache.set(ADMIN_MANAGER_WEB_CACHE_KEY, admin_path, ttl_seconds=10)
        return admin_path

    try:
        # Lazy import to avoid circular import in startup.
        from app.core.database import SessionLocal

        with SessionLocal() as session:
            value = session.execute(
                select(Setting.setting_content).where(Setting.setting_key == _ADMIN_MANAGER_WEB_SETTING_KEY),
            ).scalar_one_or_none()
            admin_path = normalize_admin_manager_web_path(
                value if isinstance(value, str) else None,
                settings.admin_manager_web_path,
            )
    except SQLAlchemyError:
        admin_path = settings.admin_manager_web_path

    cache.set(ADMIN_MANAGER_WEB_CACHE_KEY, admin_path, ttl_seconds=10)
    return admin_path


def get_install_status(session: Session) -> InstallStatus:
    cached = cache.get(INSTALL_STATUS_CACHE_KEY)
    if isinstance(cached, InstallStatus):
        return cached

    try:
        table_names = list_database_tables()
    except SQLAlchemyError:
        status = InstallStatus(
            installed=False,
            schema_ready=False,
            table_count=0,
            admin_manager_web=settings.admin_manager_web_path,
        )
        cache.set(INSTALL_STATUS_CACHE_KEY, status, ttl_seconds=5)
        return status

    schema_ready = "settings" in table_names
    admin_manager_web = settings.admin_manager_web_path
    installed = False

    if schema_ready:
        try:
            rows = session.execute(
                select(Setting.setting_key, Setting.setting_content).where(
                    Setting.setting_key.in_(
                        [
                            _INSTALL_COMPLETED_KEY,
                            _ADMIN_MANAGER_WEB_SETTING_KEY,
                            "user_account",
                            "user_account_password",
                        ],
                    ),
                ),
            ).all()
            setting_map = {str(key): (content or "") for key, content in rows}
            install_completed = _parse_boolean(setting_map.get(_INSTALL_COMPLETED_KEY))
            has_account = bool(str(setting_map.get("user_account", "")).strip())
            has_password = bool(str(setting_map.get("user_account_password", "")).strip())
            installed = install_completed or (has_account and has_password)
            admin_manager_web = normalize_admin_manager_web_path(
                setting_map.get(_ADMIN_MANAGER_WEB_SETTING_KEY),
                settings.admin_manager_web_path,
            )
        except SQLAlchemyError:
            schema_ready = False
            table_names = set()
            admin_manager_web = settings.admin_manager_web_path

    status = InstallStatus(
        installed=installed,
        schema_ready=schema_ready,
        table_count=len(table_names),
        admin_manager_web=admin_manager_web,
    )
    cache.set(INSTALL_STATUS_CACHE_KEY, status, ttl_seconds=5)
    cache.set(ADMIN_MANAGER_WEB_CACHE_KEY, admin_manager_web, ttl_seconds=10)
    return status


def bootstrap_installation(
    session: Session,
    *,
    account: str,
    password: str,
    admin_manager_web: Optional[str],
    site_title: Optional[str],
    site_sub_title: Optional[str],
    site_api_base: Optional[str],
    article_class_items: list[dict[str, str]],
    site_url: Optional[str],
    site_description: Optional[str],
    site_keywords: Optional[str],
    site_icp: Optional[str],
    site_notice: Optional[str],
) -> InstallStatus:
    # Installation flow should be able to bootstrap a fresh database.
    # If CREATE TABLE privileges are missing, return a clear conflict error.
    try:
        ensure_all_tables()
    except SQLAlchemyError as error:
        raise ValueError(
            "Database schema initialization failed during install. "
            "Please run DB migrations first or grant CREATE TABLE privilege.",
        ) from error

    required_tables = {"settings", "article", "singlepage"}
    existing_tables = list_database_tables()
    missing_tables = sorted(required_tables - existing_tables)
    if missing_tables:
        raise ValueError(
            "Database schema is not initialized. Missing tables: "
            f"{', '.join(missing_tables)}. Run DB migrations first.",
        )

    status = get_install_status(session)
    if status.installed:
        raise ValueError("System is already installed")

    normalized_account = _normalize_optional_text(account)
    normalized_password = _normalize_optional_text(password)
    if normalized_account is None:
        raise ValueError("Admin account is required")
    if normalized_password is None:
        raise ValueError("Admin password is required")

    normalized_admin_manager_web = normalize_admin_manager_web_path(
        admin_manager_web,
        settings.admin_manager_web_path,
    )

    class_map: dict[str, str] = {}
    for item in article_class_items:
        value = str(item.get("value") or "").strip()
        label = str(item.get("label") or "").strip() or value
        if not value:
            continue
        class_map[value] = label
    if not class_map:
        class_map = {"default": "默认分类"}

    nehex_article_class = {"class": class_map}
    default_theme_profiles = _build_default_theme_profiles()
    default_theme = default_theme_profiles["rei.json"]
    normalized_site_title = _normalize_optional_text(site_title) or "NeHex"
    primary_article_class = next(iter(class_map.keys()))

    updates: list[tuple[str, SettingType, object, Optional[str]]] = [
        ("user_account", SettingType.string, normalized_account, "管理员账号"),
        (
            "user_account_password",
            SettingType.string,
            hash_admin_password(normalized_password),
            "管理员密码（哈希）",
        ),
        (_ADMIN_MANAGER_WEB_SETTING_KEY, SettingType.string, normalized_admin_manager_web, "后台路径"),
        (_INSTALL_COMPLETED_KEY, SettingType.boolean, True, "首次安装完成标记"),
        ("installed_at", SettingType.string, datetime.utcnow().isoformat(), "首次安装完成时间"),
        ("site_title", SettingType.string, normalized_site_title, "站点标题"),
        ("site_sub_title", SettingType.string, _normalize_optional_text(site_sub_title) or "", "站点副标题"),
        ("site_api_base", SettingType.string, _normalize_optional_text(site_api_base) or "", "API基础路径"),
        ("nehex_article_class", SettingType.json, nehex_article_class, "文章分类配置"),
        ("site_url", SettingType.string, _normalize_optional_text(site_url) or "", "站点地址"),
        ("site_description", SettingType.string, _normalize_optional_text(site_description) or "", "站点描述"),
        ("site_keywords", SettingType.string, _normalize_optional_text(site_keywords) or "", "站点关键词"),
        ("site_icp", SettingType.string, _normalize_optional_text(site_icp) or "", "ICP备案"),
        (
            "site_notice",
            SettingType.string,
            _normalize_optional_text(site_notice) or "站点初始化完成，欢迎使用 NeHex。",
            "站点公告",
        ),
        ("theme_background", SettingType.string, str(default_theme.get("background_images", "")), "主题背景"),
        ("theme_primary", SettingType.string, str(default_theme.get("primary", "")), "主题主色"),
        ("theme_banner", SettingType.string, str(default_theme.get("banner", "")), "主题横幅"),
        ("theme_card_style", SettingType.string, str(default_theme.get("card_style", "")), "主题卡片风格"),
        ("theme_active_profile", SettingType.string, "rei.json", "主题当前配置文件"),
        (
            "theme_profiles",
            SettingType.json,
            default_theme_profiles,
            "主题配置集合",
        ),
    ]

    for setting_key, setting_type, setting_content, description in updates:
        row = session.get(Setting, setting_key)
        if row is None:
            row = Setting(setting_key=setting_key, setting_type=setting_type)
            session.add(row)
        row.setting_type = setting_type
        row.setting_content = _serialize_setting_content(setting_type, setting_content)
        row.description = description

    _seed_default_content(
        session,
        site_title=normalized_site_title,
        primary_article_class=primary_article_class,
    )

    session.commit()
    cache.delete("settings:list")
    cache.delete("settings:list:with-theme-details")
    cache.delete(ADMIN_MANAGER_WEB_CACHE_KEY)
    cache.delete(INSTALL_STATUS_CACHE_KEY)
    return get_install_status(session)
