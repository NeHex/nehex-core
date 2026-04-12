use axum::{Json, extract::State};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: String,
}

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: state.settings.app_version.clone(),
    })
}

pub async fn version(State(state): State<AppState>) -> Json<VersionResponse> {
    Json(VersionResponse {
        version: state.settings.app_version.clone(),
    })
}
