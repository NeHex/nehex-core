from __future__ import annotations

from datetime import datetime

from fastapi import APIRouter, Depends, HTTPException, Query, Response, status
from sqlalchemy.exc import IntegrityError, SQLAlchemyError
from sqlalchemy.orm import Session

from app.core.admin_security import (
    ADMIN_TOKEN_COOKIE_KEY,
    AdminPrincipal,
    create_admin_token,
    require_admin_principal,
    verify_admin_password,
)
from app.core.config import settings
from app.core.database import get_db_session
from app.schemas.admin import (
    AdminAccountSettingsUpdateRequest,
    AdminActionResponse,
    AdminAlbumCreateRequest,
    AdminAlbumDetailResponse,
    AdminAlbumUpdateRequest,
    AdminArticleCreateRequest,
    AdminArticleDetailResponse,
    AdminArticleListResponse,
    AdminArticleUpdateRequest,
    AdminCommentCreateRequest,
    AdminCommentDetailResponse,
    AdminCommentListResponse,
    AdminCommentUpdateRequest,
    AdminDailyCreateRequest,
    AdminDailyDetailResponse,
    AdminDailyUpdateRequest,
    AdminInstallRequest,
    AdminInstallResponse,
    AdminInstallStatusData,
    AdminInstallStatusResponse,
    AdminFriendApplyDetailResponse,
    AdminFriendApplyListResponse,
    AdminFriendApplyStatusUpdateRequest,
    AdminFriendCreateRequest,
    AdminFriendDetailResponse,
    AdminFriendListResponse,
    AdminFriendUpdateRequest,
    AdminLoginData,
    AdminLoginRequest,
    AdminLoginResponse,
    AdminPageCreateRequest,
    AdminPageDetailResponse,
    AdminPageListResponse,
    AdminPageUpdateRequest,
    AdminProjectCreateRequest,
    AdminProjectDetailResponse,
    AdminProjectListResponse,
    AdminProjectUpdateRequest,
    AdminSettingListResponse,
    AdminSettingsUpdateRequest,
)
from app.services.install_service import bootstrap_installation, get_install_status
from app.services.admin_service import (
    create_admin_comment,
    create_admin_friend,
    create_album,
    create_article,
    create_daily,
    create_page,
    create_project,
    delete_admin_comment,
    delete_admin_friend,
    delete_album,
    delete_article,
    delete_daily,
    delete_page,
    delete_project,
    get_admin_credentials,
    list_admin_comments,
    list_admin_friend_applies,
    list_admin_settings,
    list_admin_friends,
    list_pages,
    list_projects,
    SENSITIVE_ADMIN_SETTING_KEYS,
    update_admin_account_settings,
    update_admin_comment,
    update_admin_friend,
    update_admin_friend_apply_status,
    update_album,
    update_article,
    update_daily,
    update_page,
    update_project,
    update_admin_settings,
)

router = APIRouter(prefix="/admin-api", tags=["admin"])


def _invalid_admin_credentials() -> HTTPException:
    return HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Invalid admin credentials",
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
    max_age = max(60, expires_at - int(datetime.utcnow().timestamp()))
    response.set_cookie(
        key=ADMIN_TOKEN_COOKIE_KEY,
        value=token,
        max_age=max_age,
        httponly=True,
        secure=False,
        samesite="lax",
        path="/",
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


@router.post("/auth/logout", response_model=AdminActionResponse, summary="Admin logout")
def admin_logout(response: Response) -> AdminActionResponse:
    response.delete_cookie(
        key=ADMIN_TOKEN_COOKIE_KEY,
        path="/",
    )
    return AdminActionResponse(message="Logged out")


@router.get("/settings", response_model=AdminSettingListResponse, summary="List settings")
def admin_list_settings_api(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    data = list_admin_settings(session=session)
    return AdminSettingListResponse(data=data)


@router.put("/settings", response_model=AdminSettingListResponse, summary="Update settings")
def admin_update_settings_api(
    payload: AdminSettingsUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    items_payload: list[dict] = []
    for item in payload.items:
        if item.setting_key in SENSITIVE_ADMIN_SETTING_KEYS:
            raise HTTPException(
                status_code=422,
                detail=f"Setting {item.setting_key} must be updated via /admin-api/settings/account",
            )
        items_payload.append(
            {
                "setting_key": item.setting_key,
                "setting_content": item.setting_content,
                "setting_type": item.setting_type,
                "description": item.description,
                "has_description": "description" in item.model_fields_set,
            },
        )

    try:
        data = update_admin_settings(session=session, items=items_payload)
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    return AdminSettingListResponse(data=data)


@router.put(
    "/settings/account",
    response_model=AdminSettingListResponse,
    summary="Update account settings",
)
def admin_update_account_settings_api(
    payload: AdminAccountSettingsUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminSettingListResponse:
    has_account = "account" in payload.model_fields_set
    has_new_password = "new_password" in payload.model_fields_set

    if not has_account and not has_new_password:
        raise HTTPException(status_code=422, detail="No account settings fields to update")

    try:
        data = update_admin_account_settings(
            session=session,
            account=payload.account if has_account else None,
            new_password=payload.new_password if has_new_password else None,
        )
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    return AdminSettingListResponse(data=data)


@router.post("/articles", response_model=AdminArticleDetailResponse, summary="Create article")
def admin_create_article(
    payload: AdminArticleCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminArticleDetailResponse:
    item = create_article(
        session=session,
        title=payload.title,
        article_class=payload.class_,
        article_top_image=payload.articleTopImage,
        read_count=payload.read,
        tag=payload.tag,
        top=payload.top,
        content=payload.content,
    )
    return AdminArticleDetailResponse(data=item)


@router.put("/articles/{article_id}", response_model=AdminArticleDetailResponse, summary="Update article")
def admin_update_article(
    article_id: int,
    payload: AdminArticleUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminArticleDetailResponse:
    data = payload.model_dump(exclude_unset=True, by_alias=False)
    item = update_article(
        session=session,
        article_id=article_id,
        title=data.get("title"),
        article_class=data.get("class_"),
        article_top_image=data.get("articleTopImage"),
        read_count=data.get("read"),
        tag=data.get("tag"),
        top=data.get("top"),
        content=data.get("content"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Article not found")
    return AdminArticleDetailResponse(data=item)


@router.delete("/articles/{article_id}", response_model=AdminActionResponse, summary="Delete article")
def admin_delete_article(
    article_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_article(session=session, article_id=article_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Article not found")
    return AdminActionResponse(message="Article deleted")


@router.post("/dailies", response_model=AdminDailyDetailResponse, summary="Create daily")
def admin_create_daily(
    payload: AdminDailyCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminDailyDetailResponse:
    item = create_daily(
        session=session,
        title=payload.title,
        content=payload.content,
        weather=payload.weather,
    )
    return AdminDailyDetailResponse(data=item)


@router.put("/dailies/{daily_id}", response_model=AdminDailyDetailResponse, summary="Update daily")
def admin_update_daily(
    daily_id: int,
    payload: AdminDailyUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminDailyDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    item = update_daily(
        session=session,
        daily_id=daily_id,
        title=data.get("title"),
        content=data.get("content"),
        weather=data.get("weather"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Daily not found")
    return AdminDailyDetailResponse(data=item)


@router.delete("/dailies/{daily_id}", response_model=AdminActionResponse, summary="Delete daily")
def admin_delete_daily(
    daily_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_daily(session=session, daily_id=daily_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Daily not found")
    return AdminActionResponse(message="Daily deleted")


@router.post("/albums", response_model=AdminAlbumDetailResponse, summary="Create album")
def admin_create_album(
    payload: AdminAlbumCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminAlbumDetailResponse:
    item = create_album(
        session=session,
        title=payload.title,
        album_class=payload.class_,
        cover=payload.cover,
        like_count=payload.like_count,
        img_urls=payload.img_urls,
    )
    return AdminAlbumDetailResponse(data=item)


@router.put("/albums/{album_id}", response_model=AdminAlbumDetailResponse, summary="Update album")
def admin_update_album(
    album_id: int,
    payload: AdminAlbumUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminAlbumDetailResponse:
    data = payload.model_dump(exclude_unset=True, by_alias=False)
    item = update_album(
        session=session,
        album_id=album_id,
        title=data.get("title"),
        album_class=data.get("class_"),
        cover=data.get("cover"),
        like_count=data.get("like_count"),
        img_urls=data.get("img_urls"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Album not found")
    return AdminAlbumDetailResponse(data=item)


@router.delete("/albums/{album_id}", response_model=AdminActionResponse, summary="Delete album")
def admin_delete_album(
    album_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_album(session=session, album_id=album_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Album not found")
    return AdminActionResponse(message="Album deleted")


@router.get("/pages", response_model=AdminPageListResponse, summary="List standalone pages")
def admin_list_pages(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminPageListResponse:
    data = list_pages(session=session)
    return AdminPageListResponse(data=data)


@router.post("/pages", response_model=AdminPageDetailResponse, summary="Create standalone page")
def admin_create_page(
    payload: AdminPageCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminPageDetailResponse:
    try:
        item = create_page(
            session=session,
            page_key=payload.page_key,
            title=payload.title,
            cover_image=payload.cover_image,
            content=payload.content,
            sort=payload.sort,
            status=payload.status,
        )
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Page key already exists") from error
    return AdminPageDetailResponse(data=item)


@router.put("/pages/{page_id}", response_model=AdminPageDetailResponse, summary="Update standalone page")
def admin_update_page(
    page_id: int,
    payload: AdminPageUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminPageDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    try:
        item = update_page(
            session=session,
            page_id=page_id,
            page_key=data.get("page_key"),
            title=data.get("title"),
            cover_image=data.get("cover_image"),
            content=data.get("content"),
            sort=data.get("sort"),
            status=data.get("status"),
        )
    except IntegrityError as error:
        session.rollback()
        raise HTTPException(status_code=409, detail="Page key already exists") from error
    if item is None:
        raise HTTPException(status_code=404, detail="Standalone page not found")
    return AdminPageDetailResponse(data=item)


@router.delete("/pages/{page_id}", response_model=AdminActionResponse, summary="Delete standalone page")
def admin_delete_page(
    page_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_page(session=session, page_id=page_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Standalone page not found")
    return AdminActionResponse(message="Standalone page deleted")


@router.get("/projects", response_model=AdminProjectListResponse, summary="List projects")
def admin_list_projects(
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminProjectListResponse:
    data = list_projects(session=session)
    return AdminProjectListResponse(data=data)


@router.post("/projects", response_model=AdminProjectDetailResponse, summary="Create project")
def admin_create_project(
    payload: AdminProjectCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminProjectDetailResponse:
    item = create_project(
        session=session,
        title=payload.title,
        cover=payload.cover,
        category=payload.category,
        description=payload.description,
        content=payload.content,
        tech_stack=payload.tech_stack,
        project_url=payload.project_url,
        github_url=payload.github_url,
        sort=payload.sort,
        status=payload.status,
    )
    return AdminProjectDetailResponse(data=item)


@router.put("/projects/{project_id}", response_model=AdminProjectDetailResponse, summary="Update project")
def admin_update_project(
    project_id: int,
    payload: AdminProjectUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminProjectDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    item = update_project(
        session=session,
        project_id=project_id,
        title=data.get("title"),
        cover=data.get("cover"),
        category=data.get("category"),
        description=data.get("description"),
        content=data.get("content"),
        tech_stack=data.get("tech_stack"),
        project_url=data.get("project_url"),
        github_url=data.get("github_url"),
        sort=data.get("sort"),
        status=data.get("status"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Project not found")
    return AdminProjectDetailResponse(data=item)


@router.delete("/projects/{project_id}", response_model=AdminActionResponse, summary="Delete project")
def admin_delete_project(
    project_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_project(session=session, project_id=project_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Project not found")
    return AdminActionResponse(message="Project deleted")


@router.get("/friends", response_model=AdminFriendListResponse, summary="List friends")
def admin_list_friends_api(
    keyword: str = Query(default="", max_length=200),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendListResponse:
    data = list_admin_friends(session=session, keyword=keyword)
    return AdminFriendListResponse(data=data)


@router.post("/friends", response_model=AdminFriendDetailResponse, summary="Create friend")
def admin_create_friend_api(
    payload: AdminFriendCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendDetailResponse:
    try:
        item = create_admin_friend(
            session=session,
            title=payload.title,
            description=payload.description,
            category=payload.category,
            favicon=payload.favicon,
            url=payload.url,
            status=payload.status,
        )
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error
    return AdminFriendDetailResponse(data=item)


@router.put("/friends/{friend_id}", response_model=AdminFriendDetailResponse, summary="Update friend")
def admin_update_friend_api(
    friend_id: int,
    payload: AdminFriendUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    try:
        item = update_admin_friend(
            session=session,
            friend_id=friend_id,
            title=data.get("title"),
            description=data.get("description"),
            category=data.get("category"),
            favicon=data.get("favicon"),
            url=data.get("url"),
            status=data.get("status"),
        )
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    if item is None:
        raise HTTPException(status_code=404, detail="Friend not found")
    return AdminFriendDetailResponse(data=item)


@router.delete("/friends/{friend_id}", response_model=AdminActionResponse, summary="Delete friend")
def admin_delete_friend_api(
    friend_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_admin_friend(session=session, friend_id=friend_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Friend not found")
    return AdminActionResponse(message="Friend deleted")


@router.get(
    "/friend-applies",
    response_model=AdminFriendApplyListResponse,
    summary="List friend applications",
)
def admin_list_friend_applies_api(
    status_filter: str = Query(default="", max_length=20, alias="status"),
    keyword: str = Query(default="", max_length=200),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendApplyListResponse:
    try:
        data = list_admin_friend_applies(
            session=session,
            status=status_filter,
            keyword=keyword,
        )
    except (ValueError, TypeError) as error:
        raise HTTPException(status_code=422, detail=str(error)) from error
    return AdminFriendApplyListResponse(data=data)


@router.put(
    "/friend-applies/{apply_id}/status",
    response_model=AdminFriendApplyDetailResponse,
    summary="Update friend application status",
)
def admin_update_friend_apply_status_api(
    apply_id: int,
    payload: AdminFriendApplyStatusUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminFriendApplyDetailResponse:
    try:
        item = update_admin_friend_apply_status(
            session=session,
            apply_id=apply_id,
            status=payload.status,
            create_friend=payload.create_friend,
            friend_category=payload.friend_category,
        )
    except (ValueError, TypeError) as error:
        session.rollback()
        raise HTTPException(status_code=422, detail=str(error)) from error

    if item is None:
        raise HTTPException(status_code=404, detail="Friend application not found")
    return AdminFriendApplyDetailResponse(data=item)


@router.post("/comments", response_model=AdminCommentDetailResponse, summary="Create comment")
def admin_create_comment(
    payload: AdminCommentCreateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentDetailResponse:
    item = create_admin_comment(
        session=session,
        parent_id=payload.parent_id,
        target_type=payload.target_type,
        target_id=payload.target_id,
        content=payload.content,
        nickname=payload.nickname,
        email=payload.email,
        website=payload.website,
        status=payload.status,
    )
    return AdminCommentDetailResponse(data=item)


@router.get("/comments", response_model=AdminCommentListResponse, summary="List comments")
def admin_list_comments_api(
    keyword: str = Query(default="", max_length=200),
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentListResponse:
    data = list_admin_comments(session=session, keyword=keyword)
    return AdminCommentListResponse(data=data)


@router.put("/comments/{comment_id}", response_model=AdminCommentDetailResponse, summary="Update comment")
def admin_update_comment(
    comment_id: int,
    payload: AdminCommentUpdateRequest,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminCommentDetailResponse:
    data = payload.model_dump(exclude_unset=True)
    item = update_admin_comment(
        session=session,
        comment_id=comment_id,
        parent_id=data.get("parent_id"),
        content=data.get("content"),
        nickname=data.get("nickname"),
        email=data.get("email"),
        website=data.get("website"),
        status=data.get("status"),
    )
    if item is None:
        raise HTTPException(status_code=404, detail="Comment not found")
    return AdminCommentDetailResponse(data=item)


@router.delete("/comments/{comment_id}", response_model=AdminActionResponse, summary="Delete comment")
def admin_delete_comment(
    comment_id: int,
    _: AdminPrincipal = Depends(require_admin_principal),
    session: Session = Depends(get_db_session),
) -> AdminActionResponse:
    deleted = delete_admin_comment(session=session, comment_id=comment_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Comment not found")
    return AdminActionResponse(message="Comment deleted")
