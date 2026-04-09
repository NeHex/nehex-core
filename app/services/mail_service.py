from __future__ import annotations

import re
import smtplib
from dataclasses import dataclass
from datetime import datetime
from email.message import EmailMessage
from email.utils import formataddr
from typing import Any

from sqlalchemy import desc, func, select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.models.comment import Comment
from app.models.mail_log import MailLog, MailLogStatus
from app.models.singlepage import SinglePage
from app.models.setting import Setting

MAIL_STATUS_FILTERS = {"all", "success", "failed"}
MAIL_SMTP_SECURITY_VALUES = {"none", "starttls", "ssl"}

MAIL_SETTING_KEYS = {
    "smtp_host": "mail_smtp_host",
    "smtp_port": "mail_smtp_port",
    "smtp_security": "mail_smtp_security",
    "smtp_username": "mail_smtp_username",
    "smtp_password": "mail_smtp_password",
    "smtp_from_email": "mail_smtp_from_email",
    "smtp_from_name": "mail_smtp_from_name",
    "smtp_timeout_seconds": "mail_smtp_timeout_seconds",
    "notify_admin_email": "mail_notify_admin_email",
    "notify_new_comment_enabled": "mail_notify_new_comment_enabled",
    "notify_reply_enabled": "mail_notify_reply_enabled",
    "reply_subject_template": "mail_reply_subject_template",
    "reply_body_template": "mail_reply_body_template",
    "new_comment_subject_template": "mail_new_comment_subject_template",
    "new_comment_body_template": "mail_new_comment_body_template",
}
SITE_TITLE_SETTING_KEY = "site_title"
SITE_URL_SETTING_KEY = "site_url"

DEFAULT_REPLY_SUBJECT_TEMPLATE = "[{{site_title}}] 你的评论有新回复"
DEFAULT_REPLY_BODY_TEMPLATE = (
    "你好，{{parent_nickname}}：\n\n"
    "你在 {{site_title}} 的评论收到了新回复。\n\n"
    "原评论内容：\n{{parent_content}}\n\n"
    "回复者：{{reply_nickname}}\n"
    "回复内容：\n{{reply_content}}\n\n"
    "评论位置：{{target_type}} #{{target_id}}\n"
    "直达链接：{{comment_url}}\n"
    "回复时间：{{reply_time}}\n"
)
DEFAULT_NEW_COMMENT_SUBJECT_TEMPLATE = "[{{site_title}}] 收到新评论提醒"
DEFAULT_NEW_COMMENT_BODY_TEMPLATE = (
    "{{site_title}} 收到了一条新评论。\n\n"
    "评论者：{{comment_nickname}}\n"
    "评论者邮箱：{{comment_email}}\n"
    "评论位置：{{target_type}} #{{target_id}}\n"
    "直达链接：{{comment_url}}\n"
    "评论时间：{{comment_time}}\n\n"
    "评论内容：\n{{comment_content}}\n"
)
TEMPLATE_VAR_PATTERN = re.compile(r"{{\s*([a-zA-Z0-9_]+)\s*}}")


@dataclass
class SmtpRuntimeConfig:
    host: str
    port: int
    security: str
    username: str
    password: str
    from_email: str
    from_name: str
    timeout_seconds: int


@dataclass
class MailNotificationSettings:
    smtp: SmtpRuntimeConfig
    site_title: str
    site_url: str
    notify_admin_email: str
    notify_new_comment_enabled: bool
    notify_reply_enabled: bool
    reply_subject_template: str
    reply_body_template: str
    new_comment_subject_template: str
    new_comment_body_template: str


def _normalize_text(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def _normalize_bool(value: Any, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    text = str(value).strip().lower()
    if not text:
        return default
    return text in {"1", "true", "yes", "on"}


def _normalize_int(value: Any, *, default: int, min_value: int, max_value: int) -> int:
    try:
        parsed = int(str(value).strip())
    except (TypeError, ValueError):
        return default
    return max(min_value, min(max_value, parsed))


def _normalize_smtp_security(value: Any) -> str:
    normalized = _normalize_text(value).lower()
    if normalized in MAIL_SMTP_SECURITY_VALUES:
        return normalized
    return "ssl"


def _normalize_site_url(value: Any) -> str:
    normalized = _normalize_text(value).rstrip("/")
    if not normalized:
        return ""
    if normalized.startswith("http://") or normalized.startswith("https://"):
        return normalized
    return f"https://{normalized.lstrip('/')}"


def _build_target_path(
    session: Session,
    *,
    target_type: str,
    target_id: int,
) -> str:
    normalized_type = _normalize_text(target_type).lower()
    normalized_id = max(0, int(target_id))

    if normalized_type == "article":
        return f"/article/{normalized_id}"
    if normalized_type == "album":
        return f"/album/{normalized_id}"
    if normalized_type == "friend_page":
        return "/friends"
    if normalized_type == "singlepage":
        page_key = session.execute(
            select(SinglePage.page_key)
            .where(SinglePage.id == normalized_id)
            .limit(1),
        ).scalar_one_or_none()
        normalized_page_key = _normalize_text(page_key).strip("/")
        if normalized_page_key:
            return f"/{normalized_page_key}"
        return f"/page/{normalized_id}"

    return "/"


def _join_site_url(site_url: str, path: str) -> str:
    normalized_path = f"/{path.lstrip('/')}" if path else "/"
    if not site_url:
        return normalized_path
    return f"{site_url}{normalized_path}"


def _build_comment_anchor(comment_id: int) -> str:
    return f"comment-{max(1, int(comment_id))}"


def _with_comment_anchor(target_url: str, comment_id: int) -> str:
    if not target_url:
        return ""
    return f"{target_url}#{_build_comment_anchor(comment_id)}"


def _append_direct_link_if_missing(body: str, comment_url: str) -> str:
    normalized_body = body or ""
    normalized_comment_url = _normalize_text(comment_url)
    if not normalized_comment_url:
        return normalized_body
    if normalized_comment_url in normalized_body:
        return normalized_body

    suffix = f"直达链接：{normalized_comment_url}"
    if not normalized_body.strip():
        return suffix
    return f"{normalized_body.rstrip()}\n\n{suffix}\n"


def _render_template(template: str, context: dict[str, Any]) -> str:
    def replace(match: re.Match[str]) -> str:
        key = match.group(1)
        value = context.get(key)
        if value is None:
            return ""
        return str(value)

    return TEMPLATE_VAR_PATTERN.sub(replace, template)


def _build_smtp_runtime_config(
    *,
    smtp_host: Any,
    smtp_port: Any,
    smtp_security: Any,
    smtp_username: Any,
    smtp_password: Any,
    smtp_from_email: Any,
    smtp_from_name: Any,
    smtp_timeout_seconds: Any,
) -> SmtpRuntimeConfig:
    host = _normalize_text(smtp_host)
    port = _normalize_int(smtp_port, default=465, min_value=1, max_value=65535)
    security = _normalize_smtp_security(smtp_security)
    username = _normalize_text(smtp_username)
    password = _normalize_text(smtp_password)
    from_email = _normalize_text(smtp_from_email) or username
    from_name = _normalize_text(smtp_from_name)
    timeout_seconds = _normalize_int(smtp_timeout_seconds, default=12, min_value=3, max_value=120)
    return SmtpRuntimeConfig(
        host=host,
        port=port,
        security=security,
        username=username,
        password=password,
        from_email=from_email,
        from_name=from_name,
        timeout_seconds=timeout_seconds,
    )


def _validate_runtime_smtp_config(config: SmtpRuntimeConfig) -> None:
    if not config.host:
        raise ValueError("SMTP host is required")
    if config.port <= 0:
        raise ValueError("SMTP port is invalid")
    if config.security not in MAIL_SMTP_SECURITY_VALUES:
        raise ValueError("SMTP security must be one of: none/starttls/ssl")
    if not config.from_email:
        raise ValueError("SMTP sender email is required")


def _send_mail(config: SmtpRuntimeConfig, *, to_email: str, subject: str, body: str) -> None:
    _validate_runtime_smtp_config(config)
    recipient = _normalize_text(to_email)
    if not recipient:
        raise ValueError("Recipient email is required")

    message = EmailMessage()
    message["Subject"] = subject.strip() or "NeHex Mail Notification"
    message["To"] = recipient
    message["From"] = (
        formataddr((config.from_name, config.from_email))
        if config.from_name
        else config.from_email
    )
    message.set_content(body or "")

    client: smtplib.SMTP | smtplib.SMTP_SSL
    if config.security == "ssl":
        client = smtplib.SMTP_SSL(config.host, config.port, timeout=config.timeout_seconds)
    else:
        client = smtplib.SMTP(config.host, config.port, timeout=config.timeout_seconds)

    with client:
        client.ehlo()
        if config.security == "starttls":
            client.starttls()
            client.ehlo()
        if config.username:
            client.login(config.username, config.password)
        client.send_message(message)


def _persist_mail_log(
    session: Session,
    *,
    category: str,
    template_key: str,
    to_email: str,
    subject: str,
    body: str,
    status: MailLogStatus,
    trigger_comment_id: int | None,
    error_message: str | None = None,
) -> None:
    log = MailLog(
        category=category,
        template_key=template_key,
        to_email=to_email,
        subject=subject,
        body=body,
        status=status,
        error_message=(error_message or "").strip()[:4000] or None,
        trigger_comment_id=trigger_comment_id,
        sent_at=datetime.utcnow() if status == MailLogStatus.success else None,
    )
    try:
        session.add(log)
        session.commit()
    except SQLAlchemyError:
        session.rollback()


def _send_and_record(
    session: Session,
    *,
    smtp_config: SmtpRuntimeConfig,
    category: str,
    template_key: str,
    to_email: str,
    subject: str,
    body: str,
    trigger_comment_id: int | None = None,
) -> None:
    try:
        _send_mail(smtp_config, to_email=to_email, subject=subject, body=body)
    except Exception as error:
        session.rollback()
        _persist_mail_log(
            session,
            category=category,
            template_key=template_key,
            to_email=to_email,
            subject=subject,
            body=body,
            status=MailLogStatus.failed,
            error_message=str(error),
            trigger_comment_id=trigger_comment_id,
        )
        raise

    _persist_mail_log(
        session,
        category=category,
        template_key=template_key,
        to_email=to_email,
        subject=subject,
        body=body,
        status=MailLogStatus.success,
        trigger_comment_id=trigger_comment_id,
    )


def _load_setting_map(session: Session, keys: set[str]) -> dict[str, Any]:
    try:
        stmt = select(Setting).where(Setting.setting_key.in_(list(keys)))
        rows = session.execute(stmt).scalars().all()
    except SQLAlchemyError:
        session.rollback()
        return {}

    result: dict[str, Any] = {}
    for row in rows:
        result[row.setting_key] = row.setting_content
    return result


def load_mail_notification_settings(session: Session) -> MailNotificationSettings:
    keys = {SITE_TITLE_SETTING_KEY, SITE_URL_SETTING_KEY, *MAIL_SETTING_KEYS.values()}
    setting_map = _load_setting_map(session, keys)
    smtp = _build_smtp_runtime_config(
        smtp_host=setting_map.get(MAIL_SETTING_KEYS["smtp_host"]),
        smtp_port=setting_map.get(MAIL_SETTING_KEYS["smtp_port"]),
        smtp_security=setting_map.get(MAIL_SETTING_KEYS["smtp_security"]),
        smtp_username=setting_map.get(MAIL_SETTING_KEYS["smtp_username"]),
        smtp_password=setting_map.get(MAIL_SETTING_KEYS["smtp_password"]),
        smtp_from_email=setting_map.get(MAIL_SETTING_KEYS["smtp_from_email"]),
        smtp_from_name=setting_map.get(MAIL_SETTING_KEYS["smtp_from_name"]),
        smtp_timeout_seconds=setting_map.get(MAIL_SETTING_KEYS["smtp_timeout_seconds"]),
    )
    site_title = _normalize_text(setting_map.get(SITE_TITLE_SETTING_KEY)) or "NeHex"
    site_url = _normalize_site_url(setting_map.get(SITE_URL_SETTING_KEY))
    return MailNotificationSettings(
        smtp=smtp,
        site_title=site_title,
        site_url=site_url,
        notify_admin_email=_normalize_text(setting_map.get(MAIL_SETTING_KEYS["notify_admin_email"])),
        notify_new_comment_enabled=_normalize_bool(
            setting_map.get(MAIL_SETTING_KEYS["notify_new_comment_enabled"]),
            default=False,
        ),
        notify_reply_enabled=_normalize_bool(
            setting_map.get(MAIL_SETTING_KEYS["notify_reply_enabled"]),
            default=False,
        ),
        reply_subject_template=(
            _normalize_text(setting_map.get(MAIL_SETTING_KEYS["reply_subject_template"]))
            or DEFAULT_REPLY_SUBJECT_TEMPLATE
        ),
        reply_body_template=(
            _normalize_text(setting_map.get(MAIL_SETTING_KEYS["reply_body_template"]))
            or DEFAULT_REPLY_BODY_TEMPLATE
        ),
        new_comment_subject_template=(
            _normalize_text(setting_map.get(MAIL_SETTING_KEYS["new_comment_subject_template"]))
            or DEFAULT_NEW_COMMENT_SUBJECT_TEMPLATE
        ),
        new_comment_body_template=(
            _normalize_text(setting_map.get(MAIL_SETTING_KEYS["new_comment_body_template"]))
            or DEFAULT_NEW_COMMENT_BODY_TEMPLATE
        ),
    )


def send_mail_test_with_config(
    session: Session,
    *,
    smtp_host: Any,
    smtp_port: Any,
    smtp_security: Any,
    smtp_username: Any,
    smtp_password: Any,
    smtp_from_email: Any,
    smtp_from_name: Any,
    smtp_timeout_seconds: Any,
    test_email: str,
) -> None:
    smtp = _build_smtp_runtime_config(
        smtp_host=smtp_host,
        smtp_port=smtp_port,
        smtp_security=smtp_security,
        smtp_username=smtp_username,
        smtp_password=smtp_password,
        smtp_from_email=smtp_from_email,
        smtp_from_name=smtp_from_name,
        smtp_timeout_seconds=smtp_timeout_seconds,
    )
    normalized_test_email = _normalize_text(test_email)
    if not normalized_test_email:
        raise ValueError("Test recipient email is required")

    subject = "NeHex 邮件通信测试"
    body = (
        "这是一封 NeHex 后台发出的 SMTP 测试邮件。\n\n"
        f"时间：{datetime.utcnow().isoformat(timespec='seconds')}Z\n"
        f"服务器：{smtp.host}:{smtp.port} ({smtp.security})\n"
    )
    _send_and_record(
        session,
        smtp_config=smtp,
        category="smtp_test",
        template_key="smtp_test",
        to_email=normalized_test_email,
        subject=subject,
        body=body,
    )


def send_comment_notification_mails(
    session: Session,
    *,
    comment: Comment,
    parent_comment: Comment | None = None,
) -> None:
    settings = load_mail_notification_settings(session)
    target_path = _build_target_path(
        session=session,
        target_type=comment.target_type,
        target_id=comment.target_id,
    )
    target_url = _join_site_url(settings.site_url, target_path)
    comment_url = _with_comment_anchor(target_url, comment.id)
    parent_comment_url = (
        _with_comment_anchor(target_url, parent_comment.id)
        if parent_comment is not None
        else ""
    )

    base_context = {
        "site_title": settings.site_title,
        "target_type": comment.target_type,
        "target_id": comment.target_id,
        "target_url": target_url,
        "comment_url": comment_url,
        "parent_comment_url": parent_comment_url,
    }

    if settings.notify_new_comment_enabled and settings.notify_admin_email:
        new_comment_context = {
            **base_context,
            "comment_nickname": comment.nickname,
            "comment_email": comment.email or "-",
            "comment_content": comment.content,
            "comment_time": comment.create_time.isoformat(sep=" ", timespec="seconds"),
        }
        subject = _render_template(settings.new_comment_subject_template, new_comment_context)
        body = _render_template(settings.new_comment_body_template, new_comment_context)
        body = _append_direct_link_if_missing(body, comment_url)
        try:
            _send_and_record(
                session,
                smtp_config=settings.smtp,
                category="new_comment_notice",
                template_key="new_comment",
                to_email=settings.notify_admin_email,
                subject=subject,
                body=body,
                trigger_comment_id=comment.id,
            )
        except Exception:
            pass

    if not settings.notify_reply_enabled:
        return
    if comment.parent_id <= 0:
        return

    if parent_comment is None:
        parent_comment = session.get(Comment, comment.parent_id)
    if parent_comment is None:
        return

    recipient = _normalize_text(parent_comment.email)
    if not recipient:
        return

    reply_context = {
        **base_context,
        "parent_nickname": parent_comment.nickname,
        "parent_content": parent_comment.content,
        "parent_comment_url": _with_comment_anchor(target_url, parent_comment.id),
        "reply_nickname": comment.nickname,
        "reply_content": comment.content,
        "reply_time": comment.create_time.isoformat(sep=" ", timespec="seconds"),
    }
    subject = _render_template(settings.reply_subject_template, reply_context)
    body = _render_template(settings.reply_body_template, reply_context)
    body = _append_direct_link_if_missing(body, comment_url)
    try:
        _send_and_record(
            session,
            smtp_config=settings.smtp,
            category="reply_notice",
            template_key="reply",
            to_email=recipient,
            subject=subject,
            body=body,
            trigger_comment_id=comment.id,
        )
    except Exception:
        pass


def list_mail_logs(
    session: Session,
    *,
    status: str = "all",
    page: int = 1,
    size: int = 20,
) -> tuple[list[MailLog], int, int, int, int]:
    normalized_status = _normalize_text(status).lower() or "all"
    if normalized_status not in MAIL_STATUS_FILTERS:
        raise ValueError("Invalid mail status filter")

    normalized_page = max(1, int(page))
    normalized_size = max(1, min(100, int(size)))
    offset = (normalized_page - 1) * normalized_size

    stmt = select(MailLog)
    count_stmt = select(func.count(MailLog.id))
    if normalized_status != "all":
        status_value = MailLogStatus.success if normalized_status == "success" else MailLogStatus.failed
        stmt = stmt.where(MailLog.status == status_value)
        count_stmt = count_stmt.where(MailLog.status == status_value)

    try:
        total = int(session.execute(count_stmt).scalar() or 0)
    except SQLAlchemyError:
        session.rollback()
        return [], normalized_page, normalized_size, 0, 0

    if total <= 0:
        return [], normalized_page, normalized_size, 0, 0

    try:
        rows = session.execute(
            stmt.order_by(desc(MailLog.created_at), desc(MailLog.id)).offset(offset).limit(normalized_size),
        ).scalars().all()
    except SQLAlchemyError:
        session.rollback()
        return [], normalized_page, normalized_size, 0, 0

    total_pages = (total + normalized_size - 1) // normalized_size
    return rows, normalized_page, normalized_size, total, total_pages
