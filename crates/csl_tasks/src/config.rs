use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub github: GitHubConfig,

    #[serde(default)]
    pub local: LocalConfig,

    #[serde(default)]
    pub sync: SyncConfig,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,

    #[serde(default = "default_label")]
    pub label: String,

    #[serde(default = "default_true")]
    pub sync_metadata: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default = "default_task_dir")]
    pub task_dir: String,

    #[serde(default)]
    pub archive_completed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default)]
    pub auto_sync: bool,

    #[serde(default = "default_conflict_strategy")]
    pub conflict_strategy: String,

    #[serde(default = "default_true")]
    pub preserve_github_labels: bool,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            repo: None,
            label: default_label(),
            sync_metadata: true,
        }
    }
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            task_dir: default_task_dir(),
            archive_completed: false,
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync: false,
            conflict_strategy: default_conflict_strategy(),
            preserve_github_labels: true,
        }
    }
}

fn default_label() -> String {
    "task".to_string()
}

fn default_task_dir() -> String {
    "tasks".to_string()
}

fn default_conflict_strategy() -> String {
    "prompt".to_string()
}

fn default_true() -> bool {
    true
}

#[allow(dead_code)]
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_from_project() -> Result<Option<Self>> {
        let config_paths = [Path::new(".csl-tasks.toml"), Path::new("tasks/config.toml")];

        for path in &config_paths {
            if path.exists() {
                return Ok(Some(Self::load(path)?));
            }
        }

        Ok(None)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
