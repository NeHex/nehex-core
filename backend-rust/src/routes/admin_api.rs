use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::routes::{
    admin_auth, admin_backup, admin_comments, admin_content, admin_dashboard, admin_developer,
    admin_friends, admin_mail, admin_media, admin_settings,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/install/status", get(admin_auth::admin_install_status))
        .route("/install", post(admin_auth::admin_install))
        .route("/auth/login", post(admin_auth::admin_login))
        .route("/auth/me", get(admin_auth::admin_me))
        .route("/auth/public-marker", get(admin_auth::admin_public_marker))
        .route("/auth/logout", post(admin_auth::admin_logout))
        .route("/dashboard", get(admin_dashboard::admin_dashboard_overview))
        .route(
            "/articles",
            get(admin_content::admin_list_articles).post(admin_content::admin_create_article),
        )
        .route(
            "/articles/{article_id}",
            get(admin_content::admin_get_article)
                .put(admin_content::admin_update_article)
                .delete(admin_content::admin_delete_article),
        )
        .route("/dailies", post(admin_content::admin_create_daily))
        .route(
            "/dailies/{daily_id}",
            get(admin_content::admin_get_daily)
                .put(admin_content::admin_update_daily)
                .delete(admin_content::admin_delete_daily),
        )
        .route("/albums", post(admin_content::admin_create_album))
        .route(
            "/albums/{album_id}",
            put(admin_content::admin_update_album).delete(admin_content::admin_delete_album),
        )
        .route(
            "/pages",
            get(admin_content::admin_list_pages).post(admin_content::admin_create_page),
        )
        .route(
            "/pages/{page_id}",
            get(admin_content::admin_get_page)
                .put(admin_content::admin_update_page)
                .delete(admin_content::admin_delete_page),
        )
        .route(
            "/projects",
            get(admin_content::admin_list_projects).post(admin_content::admin_create_project),
        )
        .route(
            "/projects/{project_id}",
            put(admin_content::admin_update_project).delete(admin_content::admin_delete_project),
        )
        .route(
            "/comments",
            get(admin_comments::admin_list_comments).post(admin_comments::admin_create_comment),
        )
        .route(
            "/comments/{comment_id}",
            put(admin_comments::admin_update_comment).delete(admin_comments::admin_delete_comment),
        )
        .route(
            "/friends",
            get(admin_friends::admin_list_friends).post(admin_friends::admin_create_friend),
        )
        .route(
            "/friends/{friend_id}",
            put(admin_friends::admin_update_friend).delete(admin_friends::admin_delete_friend),
        )
        .route(
            "/friend-applies",
            get(admin_friends::admin_list_friend_applies),
        )
        .route(
            "/friend-applies/{apply_id}/status",
            put(admin_friends::admin_update_friend_apply_status),
        )
        .route("/media/library", get(admin_media::admin_get_media_library))
        .route(
            "/media/folders",
            post(admin_media::admin_create_media_folder),
        )
        .route(
            "/media/folders/{folder_id}",
            put(admin_media::admin_rename_media_folder)
                .delete(admin_media::admin_delete_media_folder),
        )
        .route(
            "/media/folders/{folder_id}/images",
            get(admin_media::admin_get_media_folder_images),
        )
        .route(
            "/media/images/upload",
            post(admin_media::admin_upload_media_image),
        )
        .route(
            "/media/images/move",
            post(admin_media::admin_move_media_images),
        )
        .route(
            "/media/images/{image_id}",
            delete(admin_media::admin_delete_media_image),
        )
        .route(
            "/storage/upload",
            post(admin_media::admin_upload_storage_image),
        )
        .route(
            "/settings",
            get(admin_settings::admin_list_settings).put(admin_settings::admin_update_settings),
        )
        .route(
            "/developer/cli/execute",
            post(admin_developer::admin_execute_developer_cli),
        )
        .route(
            "/developer/logs",
            get(admin_developer::admin_list_developer_logs),
        )
        .route(
            "/settings/account",
            put(admin_settings::admin_update_account_settings),
        )
        .route(
            "/backups",
            get(admin_backup::admin_list_backups).post(admin_backup::admin_create_backup),
        )
        .route(
            "/backups/upload-restore",
            post(admin_backup::admin_upload_and_restore_backup),
        )
        .route(
            "/backups/{filename}",
            delete(admin_backup::admin_delete_backup),
        )
        .route(
            "/backups/{filename}/download",
            get(admin_backup::admin_download_backup),
        )
        .route(
            "/backups/{filename}/restore",
            post(admin_backup::admin_restore_backup),
        )
        .route(
            "/settings/mail/test",
            post(admin_mail::admin_test_mail_smtp),
        )
        .route("/mail-logs", get(admin_mail::admin_list_mail_logs))
}
