use crate::task::{Task, TaskStatus};
use anyhow::{Context, Result};
use gray_matter::Matter;
use gray_matter::engine::YAML;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct TaskStorage {
    pub task_dir: PathBuf,
}

impl TaskStorage {
    pub fn new(task_dir: impl AsRef<Path>) -> Self {
        Self {
            task_dir: task_dir.as_ref().to_path_buf(),
        }
    }

    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.task_dir)
            .with_context(|| format!("failed to create task directory: {:?}", self.task_dir))
    }

    pub fn task_path(&self, id: u32) -> PathBuf {
        self.task_dir.join(format!("{}.md", id))
    }

    pub fn load(&self, id: u32) -> Result<Task> {
        let path = self.task_path(id);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read task file: {:?}", path))?;

        self.parse_task(&content)
            .with_context(|| format!("failed to parse task {}", id))
    }

    pub fn load_all(&self) -> Result<Vec<Task>> {
        self.ensure_dir()?;
        let mut tasks = Vec::new();

        for entry in fs::read_dir(&self.task_dir)
            .with_context(|| format!("failed to read task directory: {:?}", self.task_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read file: {:?}", path))?;
                match self.parse_task(&content) {
                    Ok(task) => tasks.push(task),
                    Err(e) => eprintln!("Warning: failed to parse {:?}: {}", path, e),
                }
            }
        }

        tasks.sort_by_key(|t| t.id);
        Ok(tasks)
    }

    pub fn save(&self, task: &Task) -> Result<()> {
        self.ensure_dir()?;
        let path = self.task_path(task.id);
        let content = self.serialize_task(task)?;

        fs::write(&path, content).with_context(|| format!("failed to write task file: {:?}", path))
    }

    pub fn delete(&self, id: u32) -> Result<()> {
        let path = self.task_path(id);
        fs::remove_file(&path).with_context(|| format!("failed to delete task file: {:?}", path))
    }

    fn parse_task(&self, content: &str) -> Result<Task> {
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse(content);

        let mut frontmatter: HashMap<String, serde_json::Value> = parsed
            .data
            .map(|d| d.deserialize())
            .transpose()?
            .unwrap_or_default();

        let id = frontmatter
            .remove("id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .context("missing or invalid 'id' field")?;

        let subject = frontmatter
            .remove("subject")
            .and_then(|v| v.as_str().map(String::from))
            .context("missing or invalid 'subject' field")?;

        let status_str = frontmatter
            .remove("status")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "pending".to_string());

        let status = match status_str.as_str() {
            "pending" => TaskStatus::Pending,
            "in_progress" | "inprogress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "deleted" => TaskStatus::Deleted,
            _ => TaskStatus::Pending,
        };

        let active_form = frontmatter
            .remove("active_form")
            .and_then(|v| v.as_str().map(String::from));

        let blocks = frontmatter
            .remove("blocks")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let blocked_by = frontmatter
            .remove("blocked_by")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let github_issue = frontmatter
            .remove("github_issue")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let content_hash = frontmatter
            .remove("content_hash")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        let description = parsed.content.trim().to_string();

        Ok(Task {
            id,
            subject,
            description,
            active_form,
            status,
            blocks,
            blocked_by,
            metadata: frontmatter,
            github_issue,
            content_hash,
        })
    }

    fn serialize_task(&self, task: &Task) -> Result<String> {
        let mut frontmatter = task.metadata.clone();
        frontmatter.insert("id".to_string(), serde_json::json!(task.id));
        frontmatter.insert("subject".to_string(), serde_json::json!(task.subject));
        frontmatter.insert(
            "status".to_string(),
            serde_json::json!(match task.status {
                TaskStatus::Pending => "pending",
                TaskStatus::InProgress => "in_progress",
                TaskStatus::Completed => "completed",
                TaskStatus::Deleted => "deleted",
            }),
        );

        if let Some(ref active_form) = task.active_form {
            frontmatter.insert("active_form".to_string(), serde_json::json!(active_form));
        }

        if !task.blocks.is_empty() {
            frontmatter.insert("blocks".to_string(), serde_json::json!(task.blocks));
        }

        if !task.blocked_by.is_empty() {
            frontmatter.insert("blocked_by".to_string(), serde_json::json!(task.blocked_by));
        }

        if let Some(issue) = task.github_issue {
            frontmatter.insert("github_issue".to_string(), serde_json::json!(issue));
        }

        frontmatter.insert(
            "content_hash".to_string(),
            serde_json::json!(task.content_hash),
        );

        let yaml = serde_yaml::to_string(&frontmatter)?;
        Ok(format!("---\n{}---\n\n{}\n", yaml, task.description))
    }

    pub fn next_id(&self) -> Result<u32> {
        let tasks = self.load_all()?;
        Ok(tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1)
    }
}
