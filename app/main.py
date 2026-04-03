from contextlib import asynccontextmanager
from pathlib import Path
import shutil
import subprocess
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, HTMLResponse, RedirectResponse, Response

from app.api.router import api_router
from app.core.config import settings
from app.core.database import (
    check_database_connection,
    close_database,
    ensure_system_tables,
    ensure_performance_indexes,
)

PROJECT_ROOT = Path(__file__).resolve().parents[1]
ADMIN_PROJECT_DIR = PROJECT_ROOT / "app" / "nehex-admin"
ADMIN_DIST_DIR = ADMIN_PROJECT_DIR / "dist"
ADMIN_DIST_DIR_RESOLVED = ADMIN_DIST_DIR.resolve()
ADMIN_INDEX_FILE = ADMIN_DIST_DIR / "index.html"
ADMIN_BASE_PLACEHOLDER = "__ADMIN_MANAGER_WEB__"


def _run_startup_command(command: list[str], cwd: Path) -> None:
    cmd_display = " ".join(command)
    print(f"[startup] running `{cmd_display}` in {cwd}")
    completed = subprocess.run(command, cwd=str(cwd), check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            f"Admin frontend build failed: `{cmd_display}` exited with code {completed.returncode}."
        )


def _build_admin_frontend() -> None:
    npm_executable = shutil.which("npm")
    if npm_executable is None:
        raise RuntimeError(
            "Admin frontend build failed: `npm` not found in PATH. Please install Node.js/npm first."
        )

    _run_startup_command([npm_executable, "install"], ADMIN_PROJECT_DIR)
    _run_startup_command([npm_executable, "run", "build"], ADMIN_PROJECT_DIR)


@asynccontextmanager
async def lifespan(_: FastAPI):
    _build_admin_frontend()
    check_database_connection()
    try:
        ensure_system_tables()
    except Exception as error:
        # Do not block API startup when DB account lacks DDL privileges.
        print(f"[startup] skip ensure_system_tables: {error}")
    try:
        ensure_performance_indexes()
    except Exception as error:
        # Do not block API startup when DB account lacks index privileges.
        print(f"[startup] skip ensure_performance_indexes: {error}")
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


def _admin_base_with_slash() -> str:
    return f"{settings.admin_manager_web_path}/"


def _render_admin_index() -> str:
    index_html = ADMIN_INDEX_FILE.read_text(encoding="utf-8")
    admin_base = _admin_base_with_slash()
    rendered = index_html.replace(ADMIN_BASE_PLACEHOLDER, admin_base)

    base_tag = f'<base href="{admin_base}">'
    if "<base " not in rendered:
        rendered = rendered.replace("<head>", f"<head>\n    {base_tag}", 1)

    return rendered


def _resolve_admin_file(full_path: str) -> Optional[Path]:
    if not full_path:
        return None

    candidate = (ADMIN_DIST_DIR / full_path).resolve()
    try:
        candidate.relative_to(ADMIN_DIST_DIR_RESOLVED)
    except ValueError:
        return None

    if candidate.is_file():
        return candidate
    return None


@app.get(settings.admin_manager_web_path, include_in_schema=False)
async def admin_manager_root() -> Response:
    return RedirectResponse(url=f"{settings.admin_manager_web_path}/", status_code=308)


@app.get(f"{settings.admin_manager_web_path}/", include_in_schema=False)
@app.get(f"{settings.admin_manager_web_path}" + "/{full_path:path}", include_in_schema=False)
async def admin_manager(full_path: str = "") -> Response:
    if not ADMIN_INDEX_FILE.exists():
        raise HTTPException(
            status_code=404,
            detail="Admin manager frontend not found. Build app/nehex-admin first.",
        )

    admin_file = _resolve_admin_file(full_path)
    if admin_file is not None:
        return FileResponse(admin_file)

    if full_path and Path(full_path).suffix:
        raise HTTPException(status_code=404, detail="Admin asset not found.")

    return HTMLResponse(_render_admin_index())


@app.get("/health", tags=["system"], summary="Health check")
async def health() -> dict[str, str]:
    return {"status": "ok"}
