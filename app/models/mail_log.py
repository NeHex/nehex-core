from __future__ import annotations

import enum
from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, Enum, Integer, String, Text, func
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class MailLogStatus(str, enum.Enum):
    success = "success"
    failed = "failed"


class MailLog(Base):
    __tablename__ = "mail_log"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True, nullable=False)
    category: Mapped[str] = mapped_column(String(40), nullable=False, default="unknown")
    template_key: Mapped[str] = mapped_column(String(40), nullable=False, default="custom")
    to_email: Mapped[str] = mapped_column(String(255), nullable=False, index=True)
    subject: Mapped[str] = mapped_column(String(255), nullable=False)
    body: Mapped[str] = mapped_column(Text, nullable=False)
    status: Mapped[MailLogStatus] = mapped_column(
        Enum(MailLogStatus, name="mail_log_status"),
        nullable=False,
        default=MailLogStatus.success,
        index=True,
    )
    error_message: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    trigger_comment_id: Mapped[Optional[int]] = mapped_column(Integer, nullable=True, index=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
    )
    sent_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
