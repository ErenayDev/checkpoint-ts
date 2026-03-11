use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EntryError {
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),
    #[error("Cannot read package.json: {0}")]
    PackageJsonReadError(#[from] std::io::Error),
    #[error("Invalid package.json format")]
    InvalidPackageJson,
    #[error(
        "No entry file found in {0}. Looked for: package.json main, index.ts, index.js, main.ts, main.js"
    )]
    NoEntryFound(PathBuf),
}

pub fn find_entry_file(path: Option<&str>) -> Result<PathBuf, EntryError> {
    let path = path.ok_or_else(|| EntryError::PathNotFound(PathBuf::new()))?;
    let base = Path::new(path);

    if !base.exists() {
        return Err(EntryError::PathNotFound(base.to_path_buf()));
    }

    if base.is_file() {
        return Ok(base.to_path_buf());
    }

    if let Some(entry) = try_package_json(base)? {
        return Ok(entry);
    }

    let candidates = ["index.ts", "index.js", "main.ts", "main.js"];

    for candidate in candidates {
        let candidate_path = base.join(candidate);
        if candidate_path.is_file() {
            return Ok(candidate_path);
        }
    }

    Err(EntryError::NoEntryFound(base.to_path_buf()))
}

fn try_package_json(base: &Path) -> Result<Option<PathBuf>, EntryError> {
    let pkg_path = base.join("package.json");

    if !pkg_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&pkg_path)?;
    let pkg: Value = serde_json::from_str(&content).map_err(|_| EntryError::InvalidPackageJson)?;

    let main_field = match pkg.get("main").and_then(|v| v.as_str()) {
        Some(m) => m.trim(),
        None => return Ok(None),
    };

    let main_path = base.join(main_field);

    if main_path.is_file() {
        return Ok(Some(main_path));
    }

    Ok(None)
}
