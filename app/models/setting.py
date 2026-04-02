from __future__ import annotations

import enum
from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, Enum, String, Text, func
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class SettingType(str, enum.Enum):
    string = "string"
    int = "int"
    float = "float"
    boolean = "boolean"
    json = "json"


class Setting(Base):
    __tablename__ = "settings"

    setting_key: Mapped[str] = mapped_column(String(100), primary_key=True, nullable=False)
    setting_type: Mapped[SettingType] = mapped_column(
        Enum(SettingType, name="setting_type"),
        nullable=False,
        default=SettingType.string,
    )
    setting_content: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
    description: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
        onupdate=func.current_timestamp(),
    )
    created_at: Mapped[datetime] = mapped_column(
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
    )
