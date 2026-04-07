from __future__ import annotations

from typing import Any

from sqlalchemy.orm import Session

from app.models.mail_log import MailLog
from app.schemas.admin import AdminMailLogItem
from app.services.mail_service import list_mail_logs, send_mail_test_with_config


def _map_admin_mail_log_item(row: MailLog) -> AdminMailLogItem:
    return AdminMailLogItem(
        id=row.id,
        category=row.category,
        template_key=row.template_key,
        to_email=row.to_email,
        subject=row.subject,
        body=row.body,
        status=row.status.value,
        error_message=row.error_message,
        trigger_comment_id=row.trigger_comment_id,
        created_at=row.created_at,
        sent_at=row.sent_at,
    )


def list_admin_mail_logs(
    session: Session,
    *,
    status: str = "all",
    page: int = 1,
    size: int = 20,
) -> tuple[list[AdminMailLogItem], int, int, int, int]:
    rows, normalized_page, normalized_size, total, total_pages = list_mail_logs(
        session=session,
        status=status,
        page=page,
        size=size,
    )
    return (
        [_map_admin_mail_log_item(row) for row in rows],
        normalized_page,
        normalized_size,
        total,
        total_pages,
    )


def test_admin_mail_smtp(session: Session, payload: dict[str, Any]) -> None:
    send_mail_test_with_config(
        session=session,
        smtp_host=payload.get("smtp_host"),
        smtp_port=payload.get("smtp_port"),
        smtp_security=payload.get("smtp_security"),
        smtp_username=payload.get("smtp_username"),
        smtp_password=payload.get("smtp_password"),
        smtp_from_email=payload.get("smtp_from_email"),
        smtp_from_name=payload.get("smtp_from_name"),
        smtp_timeout_seconds=payload.get("smtp_timeout_seconds"),
        test_email=payload.get("test_email") or "",
    )
