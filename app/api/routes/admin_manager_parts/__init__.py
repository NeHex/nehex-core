from fastapi import APIRouter

from app.api.routes.admin_manager_parts import comments, content, dashboard, friends, install_auth, settings

router = APIRouter(prefix="/admin-api", tags=["admin"])
router.include_router(install_auth.router)
router.include_router(settings.router)
router.include_router(dashboard.router)
router.include_router(content.router)
router.include_router(friends.router)
router.include_router(comments.router)

__all__ = ["router"]
