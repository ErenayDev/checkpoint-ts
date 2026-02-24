use crate::utils::project_context::ProjectContext;
use crate::utils::transform_cache::TransformCache;
use checkpoint_parser::transform_code;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

        for entry in WalkDir::new(&self.ctx.root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !self.is_transformable(path) {
                continue;
            }

            match self.transform_file(path, minify) {
                Ok(output_path) => transformed.push(output_path),
                Err(e) => eprintln!("[WARN] Failed to transform {}: {}", path.display(), e),
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

        if self.cache.is_cached(relative_path.to_str().unwrap(), &hash) {
            if let Some(cached_path) = self
                .cache
                .get_transform_path(relative_path.to_str().unwrap())
            {
                if cached_path.exists() {
                    return Ok(cached_path.clone());
                }
            }
        }

        let transformed = transform_code(&content, file_path.to_str().unwrap(), minify)
            .map_err(|e| format!("Transform failed: {:?}", e))?;

        let output_path = self
            .ctx
            .checkpoint_dir
            .join("transforms")
            .join(relative_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&output_path, transformed)?;

        self.cache.update(
            relative_path.to_string_lossy().to_string(),
            hash,
            output_path.clone(),
        );

        Ok(output_path)
    }

    fn is_transformable(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if path.starts_with(&self.ctx.checkpoint_dir) {
            return false;
        }

        if path.components().any(|c| c.as_os_str() == "node_modules") {
            return false;
        }

        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts") | Some("js") | Some("tsx") | Some("jsx") // modify later
        )
    }
}
