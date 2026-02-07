use crate::storage::TaskStorage;
use crate::task::{LOCAL_ONLY_ID_START, Task};
use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Information about a task ID change during migration
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MigrationChange {
    pub old_id: u32,
    pub new_id: u32,
    pub filename: String,
    pub reason: String,
}

/// Handles migration of task IDs to align with GitHub issue numbers
pub struct Migration {
    backup_dir: PathBuf,
    task_dir: PathBuf,
}

impl Migration {
    /// Create a new migration handler
    pub fn new(task_dir: PathBuf) -> Self {
        Self {
            backup_dir: task_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            task_dir,
        }
    }

    /// Preview what the migration would change without applying it
    pub fn preview_migration(&self) -> Result<Vec<MigrationChange>> {
        let storage = TaskStorage::new(&self.task_dir);
        let tasks = storage.load_all()?;

        let mut changes = Vec::new();
        for task in tasks {
            let new_id = self.compute_new_id(&task);
            if new_id != task.id {
                let reason = if let Some(issue) = task.github_issue {
                    format!("Matches GitHub issue #{}", issue)
                } else {
                    format!("Local-only, assigned {}", new_id)
                };

                changes.push(MigrationChange {
                    old_id: task.id,
                    new_id,
                    filename: format!("{}.md", task.id),
                    reason,
                });
            }
        }

        changes.sort_by_key(|c| c.old_id);
        Ok(changes)
    }

    /// Execute the migration: backup, renumber files, and update references
    pub fn execute(&self) -> Result<()> {
        let changes = self.preview_migration()?;

        if changes.is_empty() {
            println!("No changes needed - all task IDs are already aligned");
            return Ok(());
        }

        // Create backup directory
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let backup_path = self.backup_dir.join(format!("tasks-backup-{}", timestamp));
        fs::create_dir_all(&backup_path)
            .with_context(|| format!("failed to create backup directory: {:?}", backup_path))?;

        // Backup all existing tasks
        let storage = TaskStorage::new(&self.task_dir);
        let tasks = storage.load_all()?;
        for task in &tasks {
            let src = storage.task_path(task.id);
            let dst = backup_path.join(format!("{}.md", task.id));
            fs::copy(&src, &dst).with_context(|| format!("failed to backup task {}", task.id))?;
        }

        println!(
            "Backed up {} tasks to {}",
            tasks.len(),
            backup_path.display()
        );

        // Build mapping from old IDs to new IDs
        let old_to_new: HashMap<u32, u32> = changes.iter().map(|c| (c.old_id, c.new_id)).collect();

        // Renumber task files and update references
        for task in tasks {
            let new_id = old_to_new.get(&task.id).copied().unwrap_or(task.id);

            let mut new_task = task.clone();
            new_task.id = new_id;

            // Update blocker/blocked_by references
            new_task.blocks = new_task
                .blocks
                .iter()
                .map(|id| old_to_new.get(id).copied().unwrap_or(*id))
                .collect();

            new_task.blocked_by = new_task
                .blocked_by
                .iter()
                .map(|id| old_to_new.get(id).copied().unwrap_or(*id))
                .collect();

            storage.save(&new_task)?;

            // Delete old file if ID changed
            if new_id != task.id {
                storage.delete(task.id)?;
                println!(
                    "  Migrated: {} -> {} ({})",
                    task.id,
                    new_id,
                    if let Some(issue) = task.github_issue {
                        format!("GitHub issue #{}", issue)
                    } else {
                        "local-only".to_string()
                    }
                );
            }
        }

        println!("\nMigration complete!");
        println!("Backup: {}", backup_path.display());
        Ok(())
    }

    /// Compute the new ID for a task based on GitHub issue number or local-only range
    fn compute_new_id(&self, task: &Task) -> u32 {
        if let Some(issue) = task.github_issue {
            // Use GitHub issue number as task ID
            issue
        } else {
            // Assign to local-only range (10000+)
            // For now, keep the original ID if it's already in local-only range
            if task.id >= LOCAL_ONLY_ID_START {
                task.id
            } else {
                // Find next available local-only ID
                // This is a simplified approach; in practice, we'd coordinate across all tasks
                LOCAL_ONLY_ID_START + (task.id % 1000)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_new_id_github_backed() {
        let migration = Migration::new(PathBuf::from("tasks"));
        let mut task = Task::new(5, "Test".to_string(), "Description".to_string());
        task.github_issue = Some(42);

        assert_eq!(migration.compute_new_id(&task), 42);
    }

    #[test]
    fn test_compute_new_id_local_only() {
        let migration = Migration::new(PathBuf::from("tasks"));
        let task = Task::new(10001, "Test".to_string(), "Description".to_string());

        assert_eq!(migration.compute_new_id(&task), 10001);
    }

    #[test]
    fn test_compute_new_id_unaligned() {
        let migration = Migration::new(PathBuf::from("tasks"));
        let task = Task::new(7, "Test".to_string(), "Description".to_string());

        assert!(migration.compute_new_id(&task) >= LOCAL_ONLY_ID_START);
    }
}
