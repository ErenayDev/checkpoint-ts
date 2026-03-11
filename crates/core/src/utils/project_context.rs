use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CheckpointConfig {
    pub output_dir: Option<PathBuf>,
    pub minify: Option<bool>,
    pub preserve_sessions: Option<u32>,
}

#[derive(Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub checkpoint_dir: PathBuf,
    pub config: CheckpointConfig,
}

impl ProjectContext {
    pub fn discover(entry_file: &Path) -> Self {
        let root = Self::find_project_root(entry_file);
        let checkpoint_dir = root.join(".checkpoint");
        let config = Self::load_config(&root);

        Self {
            root,
            checkpoint_dir,
            config,
        }
    }

    fn find_project_root(entry_file: &Path) -> PathBuf {
        let start_dir = if entry_file.is_file() {
            entry_file.parent().unwrap_or(entry_file)
        } else {
            entry_file
        };

        let mut current = start_dir.to_path_buf();

        loop {
            if current.join("package.json").exists() {
                return current;
            }

            if !current.pop() {
                return start_dir.to_path_buf();
            }
        }
    }

    fn load_config(root: &Path) -> CheckpointConfig {
        let local_config = root.join(".checkpointrc.json");
        let global_config = dirs::home_dir()
            .map(|h| h.join(".checkpointrc.json"))
            .unwrap_or_default();

        if local_config.exists() {
            return Self::parse_config(&local_config).unwrap_or_default();
        }

        if global_config.exists() {
            return Self::parse_config(&global_config).unwrap_or_default();
        }

        CheckpointConfig::default()
    }

    fn parse_config(path: &Path) -> Option<CheckpointConfig> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn ensure_checkpoint_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.checkpoint_dir)?;
        fs::create_dir_all(self.checkpoint_dir.join("sessions"))?;
        fs::create_dir_all(self.checkpoint_dir.join("transforms"))?;
        Ok(())
    }
}
