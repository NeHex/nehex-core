from __future__ import annotations

from datetime import datetime
from typing import Optional

from pydantic import BaseModel


class ProjectItem(BaseModel):
    id: int
    title: str
    cover: Optional[str] = None
    category: Optional[str] = None
    description: Optional[str] = None
    content: Optional[str] = None
    tech_stack: Optional[str] = None
    project_url: Optional[str] = None
    github_url: Optional[str] = None
    sort: int
    status: int
    create_time: datetime
    update_time: datetime


class ProjectListResponse(BaseModel):
    data: list[ProjectItem]


class ProjectDetailResponse(BaseModel):
    data: ProjectItem
