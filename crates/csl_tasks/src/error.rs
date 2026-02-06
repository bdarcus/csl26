use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("task {0} not found")]
    TaskNotFound(u32),

    #[error("circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("invalid task reference: task {0} does not exist")]
    InvalidReference(u32),

    #[error("invalid status: {0}")]
    InvalidStatus(String),

    #[error("GitHub sync error: {0}")]
    GitHubSync(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for TaskError {
    fn from(e: serde_json::Error) -> Self {
        TaskError::Serialization(e.to_string())
    }
}

impl From<serde_yaml::Error> for TaskError {
    fn from(e: serde_yaml::Error) -> Self {
        TaskError::Serialization(e.to_string())
    }
}

impl From<toml::de::Error> for TaskError {
    fn from(e: toml::de::Error) -> Self {
        TaskError::Config(e.to_string())
    }
}

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, TaskError>;
