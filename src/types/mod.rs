use serde::Serialize;

pub const MOJANG_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Serialize, Clone)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename="sha1")]
    pub hash: String,
    pub release_time: String,
    pub url: String,
    #[serde(rename="type")]
    pub version_type: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VersionManifest {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NormalizedVersion {
    pub id: String,
    pub release_time: Option<String>,
    pub java_version: Option<u8>,
    pub client_jar: Option<Downloadable>,
    pub server_jar: Option<Downloadable>,
    pub asset_index: Option<AssetIndex>,
    pub libraries: Vec<Library>,
    pub natives: Vec<NativeLibrary>,
    pub arguments: NormalizedArguments,
    pub requires_extraction: Vec<ExtractionHint>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Downloadable {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Library {
    pub name: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NativeLibrary {
    pub name: String,
    pub classifier: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExtractionHint {
    pub path: String,
    pub requires_extraction: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct NormalizedArguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}