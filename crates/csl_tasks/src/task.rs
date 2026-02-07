use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ID range for local-only tasks (not synced to GitHub)
pub const LOCAL_ONLY_ID_START: u32 = 10000;

/// Maximum ID for GitHub-backed tasks
pub const GITHUB_ID_MAX: u32 = 9999;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub subject: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<u32>,
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_issue: Option<u32>,
    pub content_hash: String,
}

impl Task {
    pub fn new(id: u32, subject: String, description: String) -> Self {
        let content_hash = Self::compute_hash(&subject, &description, &HashMap::new());
        Self {
            id,
            subject,
            description,
            active_form: None,
            status: TaskStatus::Pending,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            metadata: HashMap::new(),
            github_issue: None,
            content_hash,
        }
    }

    pub fn compute_hash(
        subject: &str,
        description: &str,
        metadata: &HashMap<String, serde_json::Value>,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(subject.as_bytes());
        hasher.update(b":");
        hasher.update(description.as_bytes());
        hasher.update(b":");
        hasher.update(
            serde_json::to_string(metadata)
                .unwrap_or_default()
                .as_bytes(),
        );
        format!("{:x}", hasher.finalize())
    }

    pub fn update_hash(&mut self) {
        self.content_hash = Self::compute_hash(&self.subject, &self.description, &self.metadata);
    }

    pub fn is_available(&self) -> bool {
        self.status == TaskStatus::Pending && self.blocked_by.is_empty()
    }

    /// Returns true if this task is local-only (not synced to GitHub)
    #[allow(dead_code)]
    pub fn is_local_only(&self) -> bool {
        self.id >= LOCAL_ONLY_ID_START
    }

    /// Returns true if this task is backed by a GitHub issue
    #[allow(dead_code)]
    pub fn is_github_backed(&self) -> bool {
        self.github_issue.is_some()
    }
}
