use crate::utils::project_context::ProjectContext;
use crate::utils::transform_cache::TransformCache;
use checkpoint_parser::swc::transform_code;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct PromotedManifest {
    #[serde(flatten)]
    entries: HashMap<String, Vec<String>>,
}

impl PromotedManifest {
    fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn update(&mut self, relative_path: String, promoted: HashSet<String>) {
        if promoted.is_empty() {
            self.entries.remove(&relative_path);
        } else {
            let mut sorted: Vec<String> = promoted.into_iter().collect();
            sorted.sort();
            self.entries.insert(relative_path, sorted);
        }
    }

    pub fn all_promoted(&self) -> HashSet<String> {
        self.entries.values().flatten().cloned().collect()
    }
}

pub struct TransformService {
    ctx: ProjectContext,
    cache: TransformCache,
}

impl TransformService {
    pub fn new(ctx: ProjectContext) -> Self {
        let cache_path = ctx.checkpoint_dir.join("cache.json");
        let cache = TransformCache::load(&cache_path);

        Self { ctx, cache }
    }

    pub fn transform_project(
        &mut self,
        minify: bool,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let mut transformed = Vec::new();

        let walker = WalkBuilder::new(&self.ctx.root)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .hidden(false)
            .follow_links(true)
            .filter_entry(|entry| {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name != "node_modules" && name != ".checkpoint" && name != ".git"
            })
            .build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("[WARN] Walk error: {}", err);
                    continue;
                }
            };

            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if self.is_transformable_extension(path) {
                match self.transform_file(path, minify) {
                    Ok(output_path) => transformed.push(output_path),
                    Err(e) => eprintln!("[WARN] Failed to transform {}: {}", path.display(), e),
                }
            } else if let Err(e) = self.copy_asset(path) {
                eprintln!("[WARN] Failed to copy asset {}: {}", path.display(), e);
            }
        }

        let cache_path = self.ctx.checkpoint_dir.join("cache.json");
        self.cache.save(&cache_path)?;

        Ok(transformed)
    }

    pub fn transform_file(
        &mut self,
        file_path: &Path,
        minify: bool,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(file_path)?;

        let hash = TransformCache::compute_hash(&content);
        let relative_path = file_path.strip_prefix(&self.ctx.root).unwrap_or(file_path);
        let relative_path_str = relative_path.to_string_lossy();

        if self.cache.is_cached(&relative_path_str, &hash) {
            if let Some(cached_path) = self.cache.get_transform_path(&relative_path_str) {
                if cached_path.exists() {
                    return Ok(cached_path.clone());
                }
            }
        }

        let runtime_import_path = compute_runtime_import_path(relative_path);

        let file_path_str = file_path.to_string_lossy();
        let result = transform_code(&content, &file_path_str, minify, &runtime_import_path)
            .map_err(|e| format!("Transform failed: {:?}", e))?;
        let transformed = result.code;

        let output_path = self
            .ctx
            .checkpoint_dir
            .join("transforms")
            .join(relative_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&output_path, transformed)?;

        let manifest_path = self.ctx.checkpoint_dir.join("promoted.json");
        let mut manifest = PromotedManifest::load(&manifest_path);
        manifest.update(
            relative_path.to_string_lossy().to_string(),
            result.promoted_functions,
        );
        manifest.save(&manifest_path)?;

        self.cache.update(
            relative_path.to_string_lossy().to_string(),
            hash,
            output_path.clone(),
        );

        Ok(output_path)
    }

    fn copy_asset(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let relative_path = file_path.strip_prefix(&self.ctx.root).unwrap_or(file_path);
        let output_path = self
            .ctx
            .checkpoint_dir
            .join("transforms")
            .join(relative_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(file_path, &output_path)?;
        Ok(())
    }

    fn is_transformable_extension(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts") | Some("js") | Some("tsx") | Some("jsx")
        )
    }
}

pub fn load_promoted_manifest(checkpoint_dir: &Path) -> HashSet<String> {
    let manifest_path = checkpoint_dir.join("promoted.json");
    PromotedManifest::load(&manifest_path).all_promoted()
}

fn compute_runtime_import_path(relative_path: &Path) -> String {
    let depth = relative_path.components().count();
    let dots: Vec<&str> = std::iter::repeat_n("..", depth).collect();
    format!("{}/runtime/checkpoint-runtime", dots.join("/"))
}

