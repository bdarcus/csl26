use crate::task::{Task, TaskStatus};
use anyhow::{Context, Result};
use octocrab::Octocrab;
use std::collections::HashMap;

#[derive(Clone)]
pub struct GitHubSync {
    octocrab: Octocrab,
    owner: String,
    repo: String,
    label: String,
}

impl GitHubSync {
    pub fn new(token: String, owner: String, repo: String) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("failed to build octocrab client")?;

        Ok(Self {
            octocrab,
            owner,
            repo,
            label: "task".to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn with_label(mut self, label: String) -> Self {
        self.label = label;
        self
    }

    pub async fn create_issue(&self, task: &Task) -> Result<u64> {
        let body = self.serialize_task_to_issue_body(task)?;

        let issue = self
            .octocrab
            .issues(&self.owner, &self.repo)
            .create(&task.subject)
            .body(&body)
            .labels(vec![
                self.label.clone(),
                format!("task-id:{}", task.id),
                status_to_label(&task.status),
            ])
            .send()
            .await
            .context("failed to create GitHub issue")?;

        Ok(issue.number)
    }

    pub async fn update_issue(&self, issue_number: u64, task: &Task) -> Result<()> {
        let body = self.serialize_task_to_issue_body(task)?;

        self.octocrab
            .issues(&self.owner, &self.repo)
            .update(issue_number)
            .title(&task.subject)
            .body(&body)
            .state(match task.status {
                TaskStatus::Completed => octocrab::models::IssueState::Closed,
                _ => octocrab::models::IssueState::Open,
            })
            .send()
            .await
            .context("failed to update GitHub issue")?;

        let labels = vec![
            self.label.clone(),
            format!("task-id:{}", task.id),
            status_to_label(&task.status),
        ];

        self.octocrab
            .issues(&self.owner, &self.repo)
            .replace_all_labels(issue_number, &labels)
            .await
            .context("failed to update issue labels")?;

        Ok(())
    }

    fn serialize_task_to_issue_body(&self, task: &Task) -> Result<String> {
        let mut frontmatter = task.metadata.clone();
        frontmatter.insert("task_id".to_string(), serde_json::json!(task.id));
        frontmatter.insert(
            "content_hash".to_string(),
            serde_json::json!(task.content_hash),
        );

        if let Some(ref active_form) = task.active_form {
            frontmatter.insert("active_form".to_string(), serde_json::json!(active_form));
        }

        let yaml = serde_yaml::to_string(&frontmatter)?;

        let mut body = format!("---\n{}---\n\n{}", yaml, task.description);

        if !task.blocks.is_empty() || !task.blocked_by.is_empty() {
            body.push_str("\n\n---\n");
            if !task.blocks.is_empty() {
                body.push_str(&format!(
                    "**Blocks**: {}\n",
                    task.blocks
                        .iter()
                        .map(|id| format!("#{}", id))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !task.blocked_by.is_empty() {
                body.push_str(&format!(
                    "**Blocked By**: {}\n",
                    task.blocked_by
                        .iter()
                        .map(|id| format!("#{}", id))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        Ok(body)
    }

    #[allow(dead_code)]
    pub async fn list_task_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>> {
        let mut issues = Vec::new();
        let mut page = 1u32;

        loop {
            let page_issues = self
                .octocrab
                .issues(&self.owner, &self.repo)
                .list()
                .labels(std::slice::from_ref(&self.label))
                .state(octocrab::params::State::All)
                .per_page(100)
                .page(page)
                .send()
                .await
                .context("failed to list GitHub issues")?;

            let items = page_issues.items;
            if items.is_empty() {
                break;
            }

            issues.extend(items);
            page += 1;
        }

        Ok(issues)
    }

    #[allow(dead_code)]
    pub fn extract_task_id(issue: &octocrab::models::issues::Issue) -> Option<u32> {
        issue
            .labels
            .iter()
            .find_map(|label| {
                label
                    .name
                    .strip_prefix("task-id:")
                    .and_then(|s| s.parse().ok())
            })
            .or_else(|| {
                issue
                    .body
                    .as_ref()
                    .and_then(|body| extract_task_id_from_body(body))
            })
    }

    pub async fn list_all_open_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>> {
        let mut issues = Vec::new();
        let mut page = 1u32;

        loop {
            let page_issues = self
                .octocrab
                .issues(&self.owner, &self.repo)
                .list()
                .state(octocrab::params::State::Open)
                .per_page(100)
                .page(page)
                .send()
                .await
                .context("failed to list GitHub issues")?;

            let items = page_issues.items;
            if items.is_empty() {
                break;
            }

            issues.extend(items);
            page += 1;
        }

        Ok(issues)
    }

    pub fn issue_to_task(
        &self,
        issue: &octocrab::models::issues::Issue,
        next_id: u32,
    ) -> Result<Task> {
        let id = Self::extract_task_id(issue).unwrap_or(next_id);
        let subject = issue.title.clone();
        let description = issue.body.as_deref().unwrap_or("").to_string();

        let status = if issue.state == octocrab::models::IssueState::Closed {
            TaskStatus::Completed
        } else {
            TaskStatus::Pending
        };

        let mut metadata = HashMap::new();

        // Extract priority from labels
        for label in &issue.labels {
            if label.name.starts_with("priority-") {
                metadata.insert(
                    "priority".to_string(),
                    serde_json::json!(label.name.strip_prefix("priority-").unwrap()),
                );
            }
        }

        // Store GitHub issue number
        let github_issue = Some(issue.number as u32);

        let content_hash = Task::compute_hash(&subject, &description, &metadata);

        Ok(Task {
            id,
            subject,
            description,
            active_form: None,
            status,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            metadata,
            github_issue,
            content_hash,
        })
    }
}

fn status_to_label(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "status:pending".to_string(),
        TaskStatus::InProgress => "status:in-progress".to_string(),
        TaskStatus::Completed => "status:completed".to_string(),
        TaskStatus::Deleted => "status:deleted".to_string(),
    }
}

#[allow(dead_code)]
fn extract_task_id_from_body(body: &str) -> Option<u32> {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let parsed = matter.parse(body);

    parsed
        .data
        .and_then(|d| d.deserialize::<HashMap<String, serde_json::Value>>().ok())
        .and_then(|fm| fm.get("task_id")?.as_u64())
        .map(|v| v as u32)
}
