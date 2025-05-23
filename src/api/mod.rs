use axum::{Json, Router, routing::get};
use crate::manifest::{fetch_version_manifest, get_version_by_id};
use crate::types::VersionManifest;
use crate::cache::get_cached_manifest;

pub fn create_router() -> Router {
    Router::new()
        .route("/manifest", get(get_versions))
        .route("/version/{id}", get(get_version_by_id))
}

pub async fn get_versions() -> Result<Json<VersionManifest>, (axum::http::StatusCode, String)> {
    let manifest = get_cached_manifest(|| async {
        match fetch_version_manifest().await {
            Ok(m) => m,
            Err(_) => VersionManifest {
                latest_release: "".to_string(),
                latest_snapshot: "".to_string(),
                versions: vec![],
            }
        }
    }).await;

    Ok(Json(manifest))
}