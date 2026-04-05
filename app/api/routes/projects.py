from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from app.core.database import get_db_session
from app.schemas.project import ProjectDetailResponse, ProjectListResponse
from app.services.projects_service import get_project_by_id, list_projects

router = APIRouter(tags=["project"])


@router.get("/project", response_model=ProjectListResponse, summary="获取全部项目")
def get_projects(session: Session = Depends(get_db_session)) -> ProjectListResponse:
    data = list_projects(session)
    return ProjectListResponse(data=data)


@router.get("/project/{project_id}", response_model=ProjectDetailResponse, summary="获取项目详情")
def get_project_detail(
    project_id: int,
    session: Session = Depends(get_db_session),
) -> ProjectDetailResponse:
    project = get_project_by_id(session, project_id)
    if project is None:
        raise HTTPException(status_code=404, detail="Project not found")
    return ProjectDetailResponse(data=project)
