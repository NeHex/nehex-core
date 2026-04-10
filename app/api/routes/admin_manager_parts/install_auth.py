from __future__ import annotations

from datetime import datetime
from typing import Optional

from fastapi import APIRouter, Depends, HTTPException, Request, Response, status
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from app.core.admin_security import (
    ADMIN_PUBLIC_MARKER_COOKIE_KEY,
    ADMIN_TOKEN_COOKIE_KEY,
    AdminPrincipal,
    create_admin_public_marker,
    create_admin_token,
    require_admin_principal,
    verify_admin_password,
)
from app.core.config import settings
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminActionResponse,
    AdminInstallRequest,
    AdminInstallResponse,
    AdminInstallStatusData,
    AdminInstallStatusResponse,
    AdminLoginData,
    AdminLoginRequest,
    AdminLoginResponse,
    AdminPublicMarkerData,
    AdminPublicMarkerResponse,
)
from app.services.admin_service import get_admin_credentials, update_admin_account_settings
from app.services.install_service import bootstrap_installation, get_install_status

router = APIRouter()


def _invalid_admin_credentials() -> HTTPException:
    return HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Invalid admin credentials",
    )


def _extract_forwarded_value(header_value: str | None) -> str:
    return str(header_value or "").split(",")[0].strip()


def _is_request_secure(request: Request) -> bool:
    forwarded_proto = _extract_forwarded_value(request.headers.get("x-forwarded-proto")).lower()
    if forwarded_proto:
        return forwarded_proto == "https"
    return request.url.scheme == "https"


def _normalize_request_host(value: Optional[str]) -> str:
    host = str(value or "").strip().lower()
    if not host:
        return ""
    if host.startswith("["):
        end_index = host.find("]")
        if end_index > 0:
            return host[1:end_index]
    if host.count(":") == 1:
        return host.split(":", 1)[0].strip()
    return host


def _parse_cookie_domain_candidates(raw_value: str) -> list[str]:
    normalized = str(raw_value or "").strip().lower()
    if not normalized:
        return []

    parsed: list[str] = []
    for chunk in normalized.replace(";", ",").split(","):
        value = chunk.strip().rstrip(".")
        if not value:
            continue
        if value.startswith("*."):
            value = value[2:]
        value = value.lstrip(".")
        if value:
            parsed.append(value)
    return parsed


def _resolve_cookie_domain(raw_setting: str, request: Request) -> str | None:
    candidates = _parse_cookie_domain_candidates(raw_setting)
    if not candidates:
        return None

    forwarded_host = _extract_forwarded_value(request.headers.get("x-forwarded-host"))
    request_host = _normalize_request_host(forwarded_host) or _normalize_request_host(
        request.url.hostname,
    )
    if not request_host:
        return candidates[0]

    for domain in candidates:
        if request_host == domain or request_host.endswith(f".{domain}"):
            return domain

    # Ignore unmatched configured domains to avoid invalid Set-Cookie domain headers.
    return None


def _resolve_admin_cookie_domain(request: Request) -> str | None:
    return _resolve_cookie_domain(settings.admin_cookie_domain, request)


def _resolve_public_cookie_domain(request: Request) -> str | None:
    return _resolve_cookie_domain(
        settings.admin_public_cookie_domain or settings.admin_cookie_domain,
        request,
    )


def _set_admin_token_cookie(
    response: Response,
    request: Request,
    token: str,
    max_age: int,
) -> None:
    response.set_cookie(
        key=ADMIN_TOKEN_COOKIE_KEY,
        value=token,
        max_age=max_age,
        httponly=True,
        secure=_is_request_secure(request),
        samesite="lax",
        path="/",
        domain=_resolve_admin_cookie_domain(request),
    )


def _set_public_marker_cookie(
    response: Response,
    request: Request,
    marker_token: str,
    max_age: int,
) -> None:
    response.set_cookie(
        key=ADMIN_PUBLIC_MARKER_COOKIE_KEY,
        value=marker_token,
        max_age=max_age,
        httponly=False,
        secure=_is_request_secure(request),
        samesite="lax",
        path="/",
        domain=_resolve_public_cookie_domain(request),
    )


def _delete_cookie_with_optional_domain(
    response: Response,
    request: Request,
    key: str,
    raw_domain_setting: str,
) -> None:
    # Always clear host-only cookies first.
    response.delete_cookie(key=key, path="/")
    response.delete_cookie(
        key=key,
        path="/",
        secure=_is_request_secure(request),
    )

    # Clear all configured domain variants to avoid stale multi-domain cookies.
    candidates = _parse_cookie_domain_candidates(raw_domain_setting)
    for domain in candidates:
        response.delete_cookie(
            key=key,
            path="/",
            domain=domain,
        )
        response.delete_cookie(
            key=key,
            path="/",
            domain=domain,
            secure=_is_request_secure(request),
        )


def _map_install_status_response() -> AdminInstallStatusResponse:
    return AdminInstallStatusResponse(
        data=AdminInstallStatusData(
            installed=False,
            schema_ready=False,
            table_count=0,
            admin_manager_web=settings.admin_manager_web_path,
        ),
    )


@router.get(
    "/install/status",
    response_model=AdminInstallStatusResponse,
    summary="Get installation status",
)
def admin_install_status(session: Session = Depends(get_db_session)) -> AdminInstallStatusResponse:
    try:
        status_data = get_install_status(session)
    except SQLAlchemyError:
        return _map_install_status_response()

    return AdminInstallStatusResponse(
        data=AdminInstallStatusData(
            installed=status_data.installed,
            schema_ready=status_data.schema_ready,
            table_count=status_data.table_count,
            admin_manager_web=status_data.admin_manager_web,
        ),
    )


@router.post(
    "/install",
    response_model=AdminInstallResponse,
    summary="Run first installation",
)
def admin_install(
    payload: AdminInstallRequest,
    session: Session = Depends(get_db_session),
) -> AdminInstallResponse:
    try:
        status_data = bootstrap_installation(
            session=session,
            account=payload.admin.account,
            password=payload.admin.password,
            admin_manager_web=payload.admin.admin_manager_web,
            site_title=payload.nehex.site_title,
            site_sub_title=payload.nehex.site_sub_title,
            site_api_base=payload.nehex.site_api_base,
            article_class_items=[
                {"value": item.value, "label": item.label or item.value}
                for item in payload.nehex.article_classes
            ],
            site_url=payload.site.site_url,
            site_description=payload.site.site_description,
            site_keywords=payload.site.site_keywords,
            site_icp=payload.site.site_icp,
            site_notice=payload.site.site_notice,
        )
    except ValueError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail=str(error)) from error
    except SQLAlchemyError as error:
        session.rollback()
        raise HTTPException(status_code=500, detail="Failed to initialize installation") from error

    return AdminInstallResponse(
        message="Installation completed",
        data=AdminInstallStatusData(
            installed=status_data.installed,
            schema_ready=status_data.schema_ready,
            table_count=status_data.table_count,
            admin_manager_web=status_data.admin_manager_web,
        ),
    )


@router.post("/auth/login", response_model=AdminLoginResponse, summary="Admin login")
def admin_login(
    payload: AdminLoginRequest,
    request: Request,
    response: Response,
    session: Session = Depends(get_db_session),
) -> AdminLoginResponse:
    try:
        install_status = get_install_status(session)
    except SQLAlchemyError as error:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="System database is unavailable",
        ) from error

    if not install_status.installed:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="System is not installed yet",
        )

    expected_account, expected_password_hash = get_admin_credentials(session)
    if not expected_account or not expected_password_hash:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Admin account is not configured",
        )

    account_matches = payload.account.strip() == expected_account
    password_matches, should_upgrade_hash = verify_admin_password(
        payload.password.strip(),
        expected_password_hash,
    )
    if not account_matches or not password_matches:
        raise _invalid_admin_credentials()

    if should_upgrade_hash:
        update_admin_account_settings(
            session=session,
            new_password=payload.password.strip(),
        )

    token, expires_at = create_admin_token(expected_account)
    marker_token, marker_expires_at = create_admin_public_marker(expected_account)
    max_age = max(60, expires_at - int(datetime.utcnow().timestamp()))
    marker_max_age = max(60, marker_expires_at - int(datetime.utcnow().timestamp()))
    _set_admin_token_cookie(
        response=response,
        request=request,
        token=token,
        max_age=max_age,
    )
    _set_public_marker_cookie(
        response=response,
        request=request,
        marker_token=marker_token,
        max_age=marker_max_age,
    )
    return AdminLoginResponse(
        data=AdminLoginData(
            token=token,
            account=expected_account,
            expires_at=datetime.utcfromtimestamp(expires_at),
        ),
    )


@router.get("/auth/me", response_model=AdminLoginResponse, summary="Get admin session")
def admin_me(principal: AdminPrincipal = Depends(require_admin_principal)) -> AdminLoginResponse:
    return AdminLoginResponse(
        data=AdminLoginData(
            account=principal.account,
            expires_at=datetime.utcfromtimestamp(principal.expires_at),
        ),
    )


@router.get(
    "/auth/public-marker",
    response_model=AdminPublicMarkerResponse,
    summary="Get admin public marker",
)
def admin_public_marker(
    request: Request,
    response: Response,
    principal: AdminPrincipal = Depends(require_admin_principal),
) -> AdminPublicMarkerResponse:
    marker_token, marker_expires_at = create_admin_public_marker(principal.account)
    marker_max_age = max(60, marker_expires_at - int(datetime.utcnow().timestamp()))

    _set_public_marker_cookie(
        response=response,
        request=request,
        marker_token=marker_token,
        max_age=marker_max_age,
    )

    return AdminPublicMarkerResponse(
        data=AdminPublicMarkerData(
            marker=marker_token,
            account=principal.account,
            expires_at=datetime.utcfromtimestamp(marker_expires_at),
        ),
    )


@router.post("/auth/logout", response_model=AdminActionResponse, summary="Admin logout")
def admin_logout(request: Request, response: Response) -> AdminActionResponse:
    _delete_cookie_with_optional_domain(
        response=response,
        request=request,
        key=ADMIN_TOKEN_COOKIE_KEY,
        raw_domain_setting=settings.admin_cookie_domain,
    )
    _delete_cookie_with_optional_domain(
        response=response,
        request=request,
        key=ADMIN_PUBLIC_MARKER_COOKIE_KEY,
        raw_domain_setting=settings.admin_public_cookie_domain or settings.admin_cookie_domain,
    )
    return AdminActionResponse(message="Logged out")
