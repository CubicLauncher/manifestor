use std::{collections::HashMap, time::Duration};

use axum::{extract::Path, response::IntoResponse, Json};
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::{sync::RwLock, time::Instant};

use crate::types::{
    AssetIndex, Downloadable, ExtractionHint, Library, MinecraftVersion,
    NativeLibrary, NormalizedArguments, NormalizedVersion, VersionManifest, MOJANG_URL,
};

static VERSION_CACHE: Lazy<RwLock<HashMap<String, (NormalizedVersion, Instant)>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
const VERSION_TTL: Duration = Duration::from_secs(60 * 30); // 30 minutos

pub async fn fetch_version_manifest() -> Result<VersionManifest, Box<dyn std::error::Error>> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct MojangVersion {
        id: String,
        url: String,
        #[serde(rename = "releaseTime")]
        release_time: String,
        #[serde(rename = "type")]
        version_type: String,
    }

    #[derive(Debug, Deserialize)]
    struct MojangManifest {
        latest: HashMap<String, String>,
        versions: Vec<MojangVersion>,
    }

    let resp = Client::new()
        .get(MOJANG_URL)
        .send()
        .await?
        .error_for_status()?
        .json::<MojangManifest>()
        .await?;

    Ok(VersionManifest {
        latest_release: resp.latest.get("release").cloned().unwrap_or_default(),
        latest_snapshot: resp.latest.get("snapshot").cloned().unwrap_or_default(),
        versions: resp
            .versions
            .into_iter()
            .map(|v| MinecraftVersion {
                id: v.id,
                url: v.url,
                release_time: v.release_time,
                version_type: v.version_type,
            })
            .collect(),
    })
}

pub async fn get_version_by_id(Path(version_id): Path<String>) -> impl IntoResponse {
    // Revisar caché
    {
        let cache = VERSION_CACHE.read().await;
        if let Some((cached, timestamp)) = cache.get(&version_id) {
            if timestamp.elapsed() < VERSION_TTL {
                return Json(cached.clone()).into_response();
            }
        }
    }

    let manifest = match fetch_version_manifest().await {
        Ok(m) => m,
        Err(_) => return (StatusCode::BAD_GATEWAY, "Error obteniendo manifest").into_response(),
    };

    let version_url = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .map(|v| v.url.clone());

    let Some(version_url) = version_url else {
        return (StatusCode::NOT_FOUND, format!("Versión '{}' no encontrada", version_id)).into_response();
    };

    let version_json = match Client::new().get(&version_url).send().await {
        Ok(resp) => match resp.error_for_status().unwrap().json::<Value>().await {
            Ok(json) => json,
            Err(_) => return (StatusCode::BAD_GATEWAY, "Error parseando JSON de la versión").into_response(),
        },
        Err(_) => return (StatusCode::BAD_GATEWAY, "Error descargando JSON de la versión").into_response(),
    };

    let result = match parse_version_json(&version_json) {
        Ok(v) => v,
        Err(msg) => return (StatusCode::BAD_GATEWAY, msg).into_response(),
    };

    // Guardar en caché
    {
        let mut cache = VERSION_CACHE.write().await;
        cache.insert(version_id, (result.clone(), Instant::now()));
    }

    Json(result).into_response()
}

fn parse_version_json(version_json: &Value) -> Result<NormalizedVersion, &'static str> {
    let id = version_json.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let release_time = version_json
        .get("releaseTime")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let java_version = version_json
        .get("javaVersion")
        .and_then(|v| v.get("majorVersion"))
        .and_then(Value::as_u64)
        .map(|v| v as u8);

    let extract_downloadable = |v: &Value| -> Option<Downloadable> {
        Some(Downloadable {
            url: v.get("url")?.as_str()?.to_string(),
            sha1: v.get("sha1")?.as_str()?.to_string(),
            size: v.get("size")?.as_u64()?,
        })
    };

    let client_jar = version_json
        .get("downloads")
        .and_then(|d| d.get("client"))
        .and_then(extract_downloadable);

    let server_jar = version_json
        .get("downloads")
        .and_then(|d| d.get("server"))
        .and_then(extract_downloadable);

    let asset_index = version_json.get("assetIndex").map(|a| AssetIndex {
        id: a.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
        url: a.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        sha1: a.get("sha1").and_then(Value::as_str).unwrap_or_default().to_string(),
        size: a.get("size").and_then(Value::as_u64).unwrap_or(0),
    });

    let mut libraries = vec![];
    let mut natives = vec![];
    let mut requires_extraction = vec![];

    if let Some(Value::Array(libs)) = version_json.get("libraries") {
        for lib in libs {
            let name = lib.get("name").and_then(Value::as_str).unwrap_or_default().to_string();

            if let Some(natives_map) = lib.get("natives").and_then(Value::as_object) {
                for (_os, classifier_val) in natives_map {
                    if let Some(classifier_str) = classifier_val.as_str() {
                        if let Some(downloads) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                            if let Some(native) = downloads.get(classifier_str) {
                                if let (Some(url), Some(sha1), Some(size), Some(path)) = (
                                    native.get("url").and_then(Value::as_str),
                                    native.get("sha1").and_then(Value::as_str),
                                    native.get("size").and_then(Value::as_u64),
                                    native.get("path").and_then(Value::as_str),
                                ) {
                                    natives.push(NativeLibrary {
                                        name: name.clone(),
                                        classifier: classifier_str.to_string(),
                                        url: url.to_string(),
                                        sha1: sha1.to_string(),
                                        size,
                                        path: path.to_string(),
                                    });

                                    let extract = lib
                                        .get("extract")
                                        .and_then(|e| e.get("exclude"))
                                        .is_some();

                                    requires_extraction.push(ExtractionHint {
                                        path: path.to_string(),
                                        requires_extraction: extract,
                                    });
                                }
                            }
                        }
                    }
                }
            } else if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                libraries.push(Library {
                    name,
                    url: artifact.get("url").and_then(Value::as_str).map(String::from),
                    sha1: artifact.get("sha1").and_then(Value::as_str).map(String::from),
                    size: artifact.get("size").and_then(Value::as_u64),
                    path: artifact.get("path").and_then(Value::as_str).map(String::from),
                });
            }
        }
    }

    let arguments = if let Some(args) = version_json.get("arguments") {
        let game = extract_args(args.get("game"));
        let jvm = extract_args(args.get("jvm"));
        NormalizedArguments { game, jvm }
    } else if let Some(args) = version_json.get("minecraftArguments").and_then(Value::as_str) {
        let game = args.split_whitespace().map(String::from).collect();
        NormalizedArguments { game, jvm: vec![] }
    } else {
        NormalizedArguments { game: vec![], jvm: vec![] }
    };

    Ok(NormalizedVersion {
        id,
        release_time,
        java_version,
        client_jar,
        server_jar,
        asset_index,
        libraries,
        natives,
        arguments,
        requires_extraction,
    })
}

fn extract_args(value: Option<&Value>) -> Vec<String> {
    let mut result = vec![];

    if let Some(Value::Array(entries)) = value {
        for entry in entries {
            match entry {
                Value::String(s) => result.push(s.clone()),
                Value::Object(obj) => {
                    if let Some(Value::String(val)) = obj.get("value") {
                        result.push(val.clone());
                    } else if let Some(Value::Array(arr)) = obj.get("value") {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                result.push(s.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}
