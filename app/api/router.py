from fastapi import APIRouter

from app.api.routes.albums import router as albums_router
from app.api.routes.articles import router as articles_router
from app.api.routes.settings import router as settings_router

api_router = APIRouter()
api_router.include_router(albums_router)
api_router.include_router(articles_router)
api_router.include_router(settings_router)
