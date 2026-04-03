from __future__ import annotations

from sqlalchemy import desc, select
from sqlalchemy.orm import Session

from app.core.simple_cache import cache
from app.models.project import Project
from app.schemas.project import ProjectItem

PROJECTS_CACHE_KEY = "projects:list"
PROJECTS_CACHE_TTL_SECONDS = 20


def _map_project_item(row: Project) -> ProjectItem:
    return ProjectItem(
        id=row.id,
        title=row.title,
        cover=row.cover,
        category=row.category,
        description=row.description,
        content=row.content,
        tech_stack=row.tech_stack,
        project_url=row.project_url,
        github_url=row.github_url,
        sort=row.sort,
        status=row.status,
        create_time=row.create_time,
        update_time=row.update_time,
    )


def list_projects(session: Session) -> list[ProjectItem]:
    cached = cache.get(PROJECTS_CACHE_KEY)
    if cached is not None:
        return [item.model_copy(deep=True) for item in cached]

    stmt = (
        select(Project)
        .where(Project.status == 1)
        .order_by(Project.sort.asc(), desc(Project.update_time), desc(Project.id))
    )
    rows = session.execute(stmt).scalars().all()
    mapped = [_map_project_item(row) for row in rows]
    cache.set(PROJECTS_CACHE_KEY, mapped, PROJECTS_CACHE_TTL_SECONDS)
    return [item.model_copy(deep=True) for item in mapped]


def get_project_by_id(session: Session, project_id: int) -> ProjectItem | None:
    stmt = (
        select(Project)
        .where(Project.id == project_id, Project.status == 1)
        .limit(1)
    )
    row = session.execute(stmt).scalars().first()
    if row is None:
        return None
    return _map_project_item(row)
