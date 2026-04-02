from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.api.router import api_router
from app.core.config import settings
from app.core.database import check_database_connection, close_database


@asynccontextmanager
async def lifespan(_: FastAPI):
    check_database_connection()
    yield
    close_database()


app = FastAPI(
    title=settings.app_name,
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(api_router)


@app.get("/health", tags=["system"], summary="健康检查")
async def health() -> dict[str, str]:
    return {"status": "ok"}
