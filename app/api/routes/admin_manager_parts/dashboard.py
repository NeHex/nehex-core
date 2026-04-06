from __future__ import annotations

from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.core.admin_security import AdminPrincipal, require_admin_principal
from app.core.database import get_db_session
from app.schemas.admin import AdminDashboardResponse
from app.services.admin_service import get_admin_dashboard_data

router = APIRouter()


@router.get("/dashboard", response_model=AdminDashboardResponse, summary="Dashboard overview")
def admin_dashboard_overview_api(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminDashboardResponse:
    data = get_admin_dashboard_data(session=session)
    return AdminDashboardResponse(data=data)
