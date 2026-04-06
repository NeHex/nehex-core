from contextlib import asynccontextmanager
from pathlib import Path
import shutil
import subprocess
import threading
import time
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
from app.core.simple_cache import close_cache
from app.services.install_service import get_admin_manager_web_path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
ADMIN_PROJECT_DIR = PROJECT_ROOT / "app" / "nehex-admin"
ADMIN_DIST_DIR = ADMIN_PROJECT_DIR / "dist"
ADMIN_DIST_DIR_RESOLVED = ADMIN_DIST_DIR.resolve()
ADMIN_INDEX_FILE = ADMIN_DIST_DIR / "index.html"
ADMIN_BASE_PLACEHOLDER = "__ADMIN_MANAGER_WEB__"
_ADMIN_INDEX_CACHE_LOCK = threading.Lock()
_ADMIN_INDEX_TEMPLATE_CACHE: tuple[float, str] | None = None


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
        if ADMIN_INDEX_FILE.exists():
            print(
                "[startup] `npm` not found in PATH, skip admin frontend build "
                "and use prebuilt dist files.",
            )
            return
        raise RuntimeError(
            "Admin frontend build failed: `npm` not found in PATH and no prebuilt "
            "admin dist found. Install Node.js/npm or set "
            "ADMIN_MANAGER_BUILD_ON_STARTUP=false with prebuilt app/nehex-admin/dist."
        )

    _run_startup_command([npm_executable, "install"], ADMIN_PROJECT_DIR)
    _run_startup_command([npm_executable, "run", "build"], ADMIN_PROJECT_DIR)


def _wait_for_database_ready() -> None:
    max_retries = max(1, int(settings.db_startup_max_retries))
    retry_interval = max(1, int(settings.db_startup_retry_interval_seconds))
    last_error: Optional[Exception] = None

    for attempt in range(1, max_retries + 1):
        try:
            check_database_connection()
            if attempt > 1:
                print(f"[startup] database ready after retry {attempt}/{max_retries}")
            return
        except Exception as error:
            last_error = error
            if attempt >= max_retries:
                break
            print(
                f"[startup] database not ready ({attempt}/{max_retries}) "
                f"{settings.db_host}:{settings.db_port}, retry in {retry_interval}s: {error}",
            )
            time.sleep(retry_interval)

    raise RuntimeError(
        "[startup] database unavailable after retries "
        f"({max_retries} attempts, host={settings.db_host}, port={settings.db_port}): {last_error}",
    )


@asynccontextmanager
async def lifespan(_: FastAPI):
    if settings.admin_manager_build_on_startup:
        _build_admin_frontend()
    if not ADMIN_INDEX_FILE.exists():
        raise RuntimeError(
            "Admin manager frontend dist not found. Set ADMIN_MANAGER_BUILD_ON_STARTUP=true or build app/nehex-admin first.",
        )

    _wait_for_database_ready()
    if settings.db_auto_create_tables:
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
    else:
        print("[startup] skip schema DDL/index auto-create (DB_AUTO_CREATE_TABLES=false)")
    yield
    close_cache()
    close_database()


app = FastAPI(
    title=settings.app_name,
    version=settings.app_version,
    lifespan=lifespan,
)

cors_origins = settings.cors_allow_origins_list
allow_credentials = settings.cors_allow_credentials and "*" not in cors_origins

app.add_middleware(
    CORSMiddleware,
    allow_origins=cors_origins,
    allow_credentials=allow_credentials,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(api_router)


def _admin_base_with_slash(admin_base_path: str) -> str:
    return f"{admin_base_path.rstrip('/')}/"


def _render_admin_index(admin_base_path: str) -> str:
    index_html = _read_admin_index_template()
    admin_base = _admin_base_with_slash(admin_base_path)
    rendered = index_html.replace(ADMIN_BASE_PLACEHOLDER, admin_base)

    base_tag = f'<base href="{admin_base}">'
    if "<base " not in rendered:
        rendered = rendered.replace("<head>", f"<head>\n    {base_tag}", 1)

    return rendered


def _read_admin_index_template() -> str:
    global _ADMIN_INDEX_TEMPLATE_CACHE

    mtime = ADMIN_INDEX_FILE.stat().st_mtime
    with _ADMIN_INDEX_CACHE_LOCK:
        cached = _ADMIN_INDEX_TEMPLATE_CACHE
        if cached is not None and cached[0] == mtime:
            return cached[1]

        content = ADMIN_INDEX_FILE.read_text(encoding="utf-8")
        _ADMIN_INDEX_TEMPLATE_CACHE = (mtime, content)
        return content


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


def _admin_path_candidates() -> list[str]:
    return [get_admin_manager_web_path()]


def _extract_admin_relative_path(request_path: str, admin_base_path: str) -> Optional[str]:
    normalized_request_path = request_path if request_path.startswith("/") else f"/{request_path}"
    normalized_admin_base = admin_base_path.rstrip("/")
    if not normalized_admin_base.startswith("/"):
        normalized_admin_base = f"/{normalized_admin_base}"

    if normalized_request_path == normalized_admin_base:
        return ""

    base_prefix = f"{normalized_admin_base}/"
    if normalized_request_path == base_prefix:
        return ""
    if not normalized_request_path.startswith(base_prefix):
        return None

    return normalized_request_path[len(base_prefix):]


@app.get("/health", tags=["system"], summary="Health check")
async def health() -> dict[str, str]:
    return {
        "status": "ok",
        "version": settings.app_version,
    }


@app.get("/version", tags=["system"], summary="Application version")
async def version() -> dict[str, str]:
    return {"version": settings.app_version}


@app.get("/{full_path:path}", include_in_schema=False)
async def admin_manager(full_path: str = "") -> Response:
    if not ADMIN_INDEX_FILE.exists():
        raise HTTPException(
            status_code=404,
            detail="Admin manager frontend not found. Build app/nehex-admin first.",
        )

    request_path = f"/{full_path}" if full_path else "/"

    for admin_base_path in _admin_path_candidates():
        relative_path = _extract_admin_relative_path(request_path, admin_base_path)
        if relative_path is None:
            continue

        if relative_path == "" and request_path != f"{admin_base_path}/":
            return RedirectResponse(url=f"{admin_base_path}/", status_code=308)

        admin_file = _resolve_admin_file(relative_path)
        if admin_file is not None:
            return FileResponse(admin_file)

        if relative_path and Path(relative_path).suffix:
            raise HTTPException(status_code=404, detail="Admin asset not found.")

        return HTMLResponse(_render_admin_index(admin_base_path))

    raise HTTPException(status_code=404, detail="Not Found")
