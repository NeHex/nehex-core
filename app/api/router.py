from fastapi import APIRouter

from app.api.routes.albums import router as albums_router
from app.api.routes.admin_manager import router as admin_manager_router
from app.api.routes.articles import router as articles_router
from app.api.routes.comments import router as comments_router
from app.api.routes.dailies import router as dailies_router
from app.api.routes.friends import router as friends_router
from app.api.routes.pages import router as pages_router
from app.api.routes.projects import router as projects_router
from app.api.routes.settings import router as settings_router

api_router = APIRouter()
api_router.include_router(albums_router)
api_router.include_router(admin_manager_router)
api_router.include_router(articles_router)
api_router.include_router(comments_router)
api_router.include_router(dailies_router)
api_router.include_router(friends_router)
api_router.include_router(pages_router)
api_router.include_router(projects_router)
api_router.include_router(settings_router)
