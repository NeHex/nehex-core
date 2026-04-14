use std::{
    path::PathBuf,
    sync::Arc,
    time::{Instant, SystemTime},
};

use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::{
    cache::RuntimeCache,
    config::Settings,
    routes::{ws_content_updates::ContentUpdatesHub, ws_online::OnlinePresenceHub},
};

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Settings>,
    pub db_pool: PgPool,
    pub paths: Arc<ProjectPaths>,
    pub admin_index_cache: Arc<RwLock<Option<AdminIndexTemplateCache>>>,
    pub admin_path_cache: Arc<RwLock<Option<AdminPathCache>>>,
    pub runtime_cache: Arc<RuntimeCache>,
    pub online_presence_hub: Arc<OnlinePresenceHub>,
    pub content_updates_hub: Arc<ContentUpdatesHub>,
}

#[derive(Debug)]
pub struct ProjectPaths {
    pub project_root: PathBuf,
    pub admin_project_dir: PathBuf,
    pub admin_dist_dir: PathBuf,
    pub admin_dist_dir_resolved: PathBuf,
    pub admin_index_file: PathBuf,
}

impl ProjectPaths {
    pub fn discover() -> Self {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let admin_project_dir = project_root.join("app").join("nehex-admin");
        let admin_dist_dir = admin_project_dir.join("dist");
        let admin_dist_dir_resolved =
            std::fs::canonicalize(&admin_dist_dir).unwrap_or_else(|_| admin_dist_dir.clone());
        let admin_index_file = admin_dist_dir.join("index.html");

        Self {
            project_root,
            admin_project_dir,
            admin_dist_dir,
            admin_dist_dir_resolved,
            admin_index_file,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdminIndexTemplateCache {
    pub mtime: Option<SystemTime>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct AdminPathCache {
    pub expires_at: Instant,
    pub value: String,
}
