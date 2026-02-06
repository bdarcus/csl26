mod archive;
mod cli;
mod config;
mod drift;
mod error;
mod github;
mod graph;
mod storage;
mod task;
mod validation;

use anyhow::Result;
use archive::Archiver;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Command, GraphFormat, OutputFormat};
use storage::TaskStorage;
use task::{Task, TaskStatus};
use validation::validate_tasks;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let storage = TaskStorage::new(&cli.task_dir);

    match cli.command {
        Command::List { status, format } => {
            let tasks = storage.load_all()?;
            let filtered: Vec<_> = if let Some(status_filter) = status {
                tasks
                    .into_iter()
                    .filter(|t| matches_status(&t.status, &status_filter))
                    .collect()
            } else {
                tasks
            };

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&filtered)?);
                }
                OutputFormat::Table => {
                    use tabled::{
                        Table, Tabled,
                        settings::{
                            Color, Modify, Style,
                            object::{Columns, Object, Rows},
                        },
                    };

                    #[derive(Tabled)]
                    struct TaskRow {
                        #[tabled(rename = "ID")]
                        id: String,
                        #[tabled(rename = "Subject")]
                        subject: String,
                        #[tabled(rename = "Priority")]
                        priority: String,
                        #[tabled(rename = "Status")]
                        status: String,
                        #[tabled(rename = "Blocked")]
                        blocked: String,
                    }

                    let rows: Vec<TaskRow> = filtered
                        .iter()
                        .map(|task| {
                            let priority = task
                                .metadata
                                .get("priority")
                                .and_then(|v| v.as_str())
                                .unwrap_or("none");

                            let status_str = match task.status {
                                task::TaskStatus::Pending => "pending",
                                task::TaskStatus::InProgress => "in-progress",
                                task::TaskStatus::Completed => "completed",
                                task::TaskStatus::Deleted => "deleted",
                            };

                            let blocked_str = if task.blocked_by.is_empty() {
                                "No".to_string()
                            } else {
                                format!("Yes ({})", task.blocked_by.len())
                            };

                            TaskRow {
                                id: task.id.to_string(),
                                subject: truncate(&task.subject, 45),
                                priority: priority.to_string(),
                                status: status_str.to_string(),
                                blocked: blocked_str,
                            }
                        })
                        .collect();

                    let mut table = Table::new(rows);
                    table.with(Style::modern());

                    // Apply colors using tabled's Color modifier
                    // Priority column (index 2): red for high/highest, yellow for medium, dim for low
                    for (idx, task) in filtered.iter().enumerate() {
                        let row = idx + 1; // +1 because row 0 is header
                        let priority = task
                            .metadata
                            .get("priority")
                            .and_then(|v| v.as_str())
                            .unwrap_or("none");

                        let priority_color = match priority {
                            "highest" | "high" => Color::FG_RED,
                            "medium" => Color::FG_YELLOW,
                            "low" => Color::FG_BRIGHT_BLACK,
                            _ => Color::FG_WHITE,
                        };

                        let status_color = match task.status {
                            task::TaskStatus::Pending => Color::FG_BRIGHT_BLUE,
                            task::TaskStatus::InProgress => Color::FG_BRIGHT_GREEN,
                            task::TaskStatus::Completed => Color::FG_BRIGHT_BLACK,
                            task::TaskStatus::Deleted => Color::FG_BRIGHT_BLACK,
                        };

                        let blocked_color = if task.blocked_by.is_empty() {
                            Color::FG_BRIGHT_BLACK
                        } else {
                            Color::FG_RED
                        };

                        table
                            .with(
                                Modify::new(Rows::single(row).and(Columns::single(2)))
                                    .with(priority_color),
                            )
                            .with(
                                Modify::new(Rows::single(row).and(Columns::single(3)))
                                    .with(status_color),
                            )
                            .with(
                                Modify::new(Rows::single(row).and(Columns::single(4)))
                                    .with(blocked_color),
                            );
                    }

                    println!("{}", table);
                }
                _ => unreachable!(),
            }
        }

        Command::Get { id, format } => {
            let task = storage.load(id)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&task)?);
                }
                OutputFormat::Text => {
                    println!("Task #{}", task.id);
                    println!("Subject: {}", task.subject);
                    println!("Status: {}", status_str(&task.status));
                    if let Some(ref active_form) = task.active_form {
                        println!("Active Form: {}", active_form);
                    }
                    if !task.blocks.is_empty() {
                        println!("Blocks: {:?}", task.blocks);
                    }
                    if !task.blocked_by.is_empty() {
                        println!("Blocked By: {:?}", task.blocked_by);
                    }
                    if !task.metadata.is_empty() {
                        println!(
                            "Metadata: {}",
                            serde_json::to_string_pretty(&task.metadata)?
                        );
                    }
                    println!("\nDescription:\n{}", task.description);
                }
                _ => unreachable!(),
            }
        }

        Command::Create {
            subject,
            description,
            active_form,
            metadata,
        } => {
            let id = storage.next_id()?;
            let mut task = Task::new(id, subject, description);
            task.active_form = active_form;

            for (key, value) in metadata {
                task.metadata.insert(key, value);
            }

            task.update_hash();
            storage.save(&task)?;
            println!("Created task #{}", id);
        }

        Command::Update {
            id,
            subject,
            description,
            status,
            active_form,
            add_blocks,
            add_blocked_by,
            metadata,
        } => {
            let mut task = storage.load(id)?;

            if let Some(s) = subject {
                task.subject = s;
            }
            if let Some(d) = description {
                task.description = d;
            }
            if let Some(s) = status {
                task.status = parse_status(&s)?;
            }
            if let Some(af) = active_form {
                task.active_form = Some(af);
            }

            for block_id in add_blocks {
                if !task.blocks.contains(&block_id) {
                    task.blocks.push(block_id);
                }
            }

            for blocked_by_id in add_blocked_by {
                if !task.blocked_by.contains(&blocked_by_id) {
                    task.blocked_by.push(blocked_by_id);
                }
            }

            for (key, value) in metadata {
                task.metadata.insert(key, value);
            }

            task.update_hash();
            storage.save(&task)?;
            println!("Updated task #{}", id);
        }

        Command::Delete { id } => {
            storage.delete(id)?;
            println!("Deleted task #{}", id);
        }

        Command::Next { format } => {
            let tasks = storage.load_all()?;
            let next = tasks
                .into_iter()
                .filter(|t| t.is_available())
                .min_by_key(|t| t.id);

            if let Some(task) = next {
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&task)?);
                    }
                    OutputFormat::Text => {
                        println!("Next available task: #{} - {}", task.id, task.subject);
                    }
                    _ => unreachable!(),
                }
            } else {
                println!("No available tasks");
            }
        }

        Command::Claim { id } => {
            let mut task = storage.load(id)?;
            task.status = TaskStatus::InProgress;
            storage.save(&task)?;
            println!("Claimed task #{}", id);
        }

        Command::Complete { id } => {
            let mut task = storage.load(id)?;
            task.status = TaskStatus::Completed;
            storage.save(&task)?;
            println!("Completed task #{}", id);
        }

        Command::Validate => {
            let tasks = storage.load_all()?;
            validate_tasks(&tasks)?;
            println!("All tasks valid (no circular dependencies or invalid references)");
        }

        Command::Sync {
            dry_run,
            direction,
            github_token,
            github_repo,
        } => {
            use cli::SyncDirection;

            let token = github_token
                .or_else(|| std::env::var("GITHUB_TOKEN").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!("GitHub token required (--github-token or GITHUB_TOKEN env)")
                })?;

            let repo_str = github_repo
                .or_else(|| std::env::var("GITHUB_REPO").ok())
                .ok_or_else(|| anyhow::anyhow!("GitHub repo required (--github-repo or GITHUB_REPO env, format: owner/repo)"))?;

            let parts: Vec<&str> = repo_str.split('/').collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid repo format, expected: owner/repo"));
            }
            let (owner, repo) = (parts[0].to_string(), parts[1].to_string());

            let github = github::GitHubSync::new(token, owner, repo)?;

            match direction {
                SyncDirection::ToGh => {
                    let tasks = storage.load_all()?;
                    println!("Syncing {} tasks to GitHub...", tasks.len());

                    if dry_run {
                        for task in &tasks {
                            println!("  [DRY RUN] Would sync task #{}: {}", task.id, task.subject);
                        }
                        println!("Dry run complete (no changes made)");
                    } else {
                        let mut handles = Vec::new();

                        for task in tasks.clone() {
                            let github_clone = github.clone();
                            let storage_clone = storage.clone();

                            let handle = tokio::spawn(async move {
                                if let Some(issue_num) = task.github_issue {
                                    println!(
                                        "  Updating issue #{} for task #{}",
                                        issue_num, task.id
                                    );
                                    github_clone.update_issue(issue_num as u64, &task).await?;
                                } else {
                                    println!("  Creating issue for task #{}", task.id);
                                    let issue_num = github_clone.create_issue(&task).await?;
                                    let mut updated_task = task.clone();
                                    updated_task.github_issue = Some(issue_num as u32);
                                    storage_clone.save(&updated_task)?;
                                    println!("    Created issue #{}", issue_num);
                                }
                                Ok::<_, anyhow::Error>(())
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await??;
                        }

                        println!("Sync complete!");
                    }
                }
                SyncDirection::FromGh => {
                    println!("Syncing from GitHub...");
                    let issues = github.list_all_open_issues().await?;
                    println!("Found {} open issues", issues.len());

                    if dry_run {
                        for issue in &issues {
                            println!(
                                "  [DRY RUN] Would import issue #{}: {}",
                                issue.number, issue.title
                            );
                        }
                        println!("Dry run complete (no changes made)");
                    } else {
                        let existing_tasks = storage.load_all().unwrap_or_default();
                        let mut next_id =
                            existing_tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;

                        let mut imported = 0;
                        let mut skipped = 0;

                        for issue in issues {
                            // Check if we already have this issue
                            if existing_tasks
                                .iter()
                                .any(|t| t.github_issue == Some(issue.number as u32))
                            {
                                skipped += 1;
                                continue;
                            }

                            let task = github.issue_to_task(&issue, next_id)?;
                            storage.save(&task)?;
                            println!("  Imported issue #{} as task #{}", issue.number, task.id);

                            if github::GitHubSync::extract_task_id(&issue).is_none() {
                                next_id += 1;
                            }

                            imported += 1;
                        }

                        println!("\nSync complete!");
                        println!("  Imported: {}", imported);
                        println!("  Skipped (already exists): {}", skipped);
                    }
                }
                SyncDirection::Both => {
                    println!("Bidirectional sync not yet implemented");
                }
            }
        }

        Command::SyncStatus {
            github_token,
            github_repo,
        } => {
            let token = github_token
                .or_else(|| std::env::var("GITHUB_TOKEN").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!("GitHub token required (--github-token or GITHUB_TOKEN env)")
                })?;

            let repo_str = github_repo
                .or_else(|| std::env::var("GITHUB_REPO").ok())
                .ok_or_else(|| anyhow::anyhow!("GitHub repo required (--github-repo or GITHUB_REPO env, format: owner/repo)"))?;

            let parts: Vec<&str> = repo_str.split('/').collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid repo format, expected: owner/repo"));
            }
            let (owner, repo) = (parts[0].to_string(), parts[1].to_string());

            let github = github::GitHubSync::new(token, owner, repo)?;
            let local_tasks = storage.load_all()?;

            println!("Fetching GitHub issues...");
            let issues = github.list_task_issues().await?;

            let mut remote_tasks = Vec::new();
            for issue in &issues {
                if let Some(task_id) = github::GitHubSync::extract_task_id(issue)
                    && let Some(body) = &issue.body
                    && let Ok(remote_task) = parse_issue_to_task(task_id, body, issue)
                {
                    let hash = extract_content_hash(body).unwrap_or_default();
                    remote_tasks.push((issue.number, remote_task, hash));
                }
            }

            let report = drift::detect_drift(&local_tasks, &remote_tasks);

            println!("\nDrift Summary:");
            println!("  Content Drift: {} tasks", report.content_drift_count());
            println!(
                "  Status Mismatch: {} tasks",
                report.status_mismatch_count()
            );
            println!(
                "  Dependency Drift: {} tasks",
                report.dependency_drift_count()
            );
            println!("  Orphaned Tasks: {}", report.orphaned_task_count());
            println!("  Orphaned Issues: {}", report.orphaned_issue_count());

            if report.has_drift() {
                println!("\nDetails:");
                for drift in &report.drifts {
                    match drift {
                        drift::DriftType::ContentDrift {
                            task_id,
                            issue_num,
                            local_hash,
                            remote_hash,
                        } => {
                            println!("  Task {} ↔ Issue #{}", task_id, *issue_num as u32);
                            println!("    Local hash:  {}", &local_hash[..16]);
                            println!("    Remote hash: {}", &remote_hash[..16]);
                        }
                        drift::DriftType::StatusMismatch {
                            task_id,
                            issue_num,
                            local_status,
                            remote_status,
                        } => {
                            println!("  Task {} ↔ Issue #{}", task_id, *issue_num as u32);
                            println!("    Local:  {:?}", local_status);
                            println!("    Remote: {:?}", remote_status);
                        }
                        drift::DriftType::DependencyDrift {
                            task_id,
                            issue_num,
                            local_blocks,
                            remote_blocks,
                        } => {
                            println!("  Task {} ↔ Issue #{}", task_id, *issue_num as u32);
                            println!("    Local blocks:  {:?}", local_blocks);
                            println!("    Remote blocks: {:?}", remote_blocks);
                        }
                        drift::DriftType::OrphanedTask { task_id } => {
                            println!("  Task {} (no matching GitHub issue)", task_id);
                        }
                        drift::DriftType::OrphanedIssue { issue_num, title } => {
                            println!(
                                "  Issue #{} \"{}\" (no matching local task)",
                                issue_num, title
                            );
                        }
                    }
                }
            } else {
                println!("\nNo drift detected!");
            }
        }

        Command::Graph { format } => {
            let tasks = storage.load_all()?;

            let output = match format {
                GraphFormat::Ascii => graph::render_ascii(&tasks),
                GraphFormat::Dot => graph::render_dot(&tasks),
            };

            println!("{}", output);
        }

        Command::Archive { dry_run } => {
            let archiver = Archiver::new(&cli.task_dir);
            let archived = archiver.archive_completed(dry_run)?;

            if dry_run {
                println!(
                    "\nDry run complete. {} tasks would be archived.",
                    archived.len()
                );
            } else {
                println!("\nArchived {} completed tasks.", archived.len());
            }
        }

        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        }
    }

    Ok(())
}

fn parse_issue_to_task(
    task_id: u32,
    body: &str,
    issue: &octocrab::models::issues::Issue,
) -> anyhow::Result<Task> {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let parsed = matter.parse(body);

    let status = if issue.state == octocrab::models::IssueState::Closed {
        TaskStatus::Completed
    } else {
        TaskStatus::Pending
    };

    let description = parsed.content.trim().to_string();

    Ok(Task {
        id: task_id,
        subject: issue.title.clone(),
        description,
        active_form: None,
        status,
        blocks: Vec::new(),
        blocked_by: Vec::new(),
        metadata: std::collections::HashMap::new(),
        github_issue: Some(issue.number as u32),
        content_hash: String::new(),
    })
}

fn extract_content_hash(body: &str) -> Option<String> {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let parsed = matter.parse(body);

    parsed
        .data
        .and_then(|d| {
            d.deserialize::<std::collections::HashMap<String, serde_json::Value>>()
                .ok()
        })
        .and_then(|fm| fm.get("content_hash")?.as_str().map(String::from))
}

fn matches_status(status: &TaskStatus, filter: &str) -> bool {
    matches!(
        (status, filter),
        (TaskStatus::Pending, "pending")
            | (TaskStatus::InProgress, "in_progress" | "inprogress")
            | (TaskStatus::Completed, "completed")
            | (TaskStatus::Deleted, "deleted")
    )
}

fn parse_status(s: &str) -> Result<TaskStatus> {
    match s {
        "pending" => Ok(TaskStatus::Pending),
        "in_progress" | "inprogress" => Ok(TaskStatus::InProgress),
        "completed" => Ok(TaskStatus::Completed),
        "deleted" => Ok(TaskStatus::Deleted),
        _ => Err(anyhow::anyhow!("invalid status: {}", s)),
    }
}

fn status_str(status: &TaskStatus) -> &str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Deleted => "deleted",
    }
}

fn strip_emoji(s: &str) -> String {
    s.chars()
        .filter(|c| {
            let c = *c as u32;
            // Filter out emoji ranges
            !((0x1F300..=0x1F9FF).contains(&c) // Emoticons, symbols, pictographs
                || (0x2600..=0x26FF).contains(&c) // Miscellaneous symbols
                || (0x2700..=0x27BF).contains(&c) // Dingbats
                || (0xFE00..=0xFE0F).contains(&c) // Variation selectors
                || (0x1F000..=0x1F02F).contains(&c) // Mahjong tiles
                || (0x1F0A0..=0x1F0FF).contains(&c)) // Playing cards
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = strip_emoji(s);
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
