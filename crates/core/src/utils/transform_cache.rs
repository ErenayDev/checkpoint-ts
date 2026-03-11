use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCache {
    pub hash: String,
    pub last_transform: String,
    pub transform_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TransformCache {
    pub version: String,
    pub files: HashMap<String, FileCache>,
}

impl TransformCache {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    pub fn load(cache_path: &Path) -> Self {
        if !cache_path.exists() {
            return Self::default();
        }

        fs::read_to_string(cache_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .filter(|cache: &TransformCache| cache.version == Self::VERSION)
            .unwrap_or_default()
    }

    pub fn save(&self, cache_path: &Path) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn is_cached(&self, file_path: &str, current_hash: &str) -> bool {
        self.files
            .get(file_path)
            .map(|cache| cache.hash == current_hash)
            .unwrap_or(false)
    }

    pub fn get_transform_path(&self, file_path: &str) -> Option<&PathBuf> {
        self.files.get(file_path).map(|cache| &cache.transform_path)
    }

    pub fn update(&mut self, file_path: String, hash: String, transform_path: PathBuf) {
        self.version = Self::VERSION.to_string();
        self.files.insert(
            file_path,
            FileCache {
                hash,
                last_transform: chrono::Utc::now().to_rfc3339(),
                transform_path,
            },
        );
    }
}
