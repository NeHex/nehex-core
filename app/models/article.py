from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import DateTime, Integer, String, Text, func, text
from sqlalchemy.orm import Mapped, mapped_column

from app.models.base import Base


class Article(Base):
    __tablename__ = "article"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True, nullable=False)
    title: Mapped[str] = mapped_column(String(255), nullable=False)
    article_top_image: Mapped[Optional[str]] = mapped_column(
        "articleTopImage",
        String(500),
        nullable=True,
    )
    article_class: Mapped[str] = mapped_column("class", String(100), nullable=False, index=True)
    read_count: Mapped[int] = mapped_column(
        "read",
        Integer,
        nullable=False,
        server_default=text("0"),
        default=0,
    )
    like_count: Mapped[int] = mapped_column(
        Integer,
        nullable=False,
        server_default=text("0"),
        default=0,
    )
    last_edit_time: Mapped[datetime] = mapped_column(
        "lastEditTime",
        DateTime,
        nullable=False,
        server_default=func.current_timestamp(),
        onupdate=func.current_timestamp(),
    )
    tag: Mapped[Optional[str]] = mapped_column(String(255), nullable=True)
    top: Mapped[int] = mapped_column(Integer, nullable=False, server_default=text("0"), default=0)
    content: Mapped[Optional[str]] = mapped_column(Text, nullable=True)
