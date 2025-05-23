use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use crate::types::VersionManifest;

static VERSION_MANIFEST_CACHE: Lazy<RwLock<ManifestCache>> = Lazy::new(|| {
    RwLock::new(ManifestCache {
        data: None,
        updated_at: None,
    })
});

struct ManifestCache {
    data: Option<VersionManifest>,
    updated_at: Option<Instant>,
}

const TTL: Duration = Duration::from_secs(60 * 50); // 50 minutos

pub async fn get_cached_manifest<F, Fut>(fetch_fn: F) -> VersionManifest
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = VersionManifest>,
{
    let now = Instant::now();

    {
        let read_guard = VERSION_MANIFEST_CACHE.read().await;
        if let (Some(data), Some(updated)) = (&read_guard.data, read_guard.updated_at) {
            if now.duration_since(updated) < TTL {
                return data.clone();
            }
        }
    }

    let new_manifest = fetch_fn().await;

    let mut write_guard = VERSION_MANIFEST_CACHE.write().await;
    write_guard.data = Some(new_manifest.clone());
    write_guard.updated_at = Some(now);

    new_manifest
}
