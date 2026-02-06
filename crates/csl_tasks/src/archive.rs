use crate::storage::TaskStorage;
use crate::task::TaskStatus;
use anyhow::Result;
use std::path::Path;

pub struct Archiver {
    storage: TaskStorage,
}

impl Archiver {
    pub fn new(task_dir: &Path) -> Self {
        Self {
            storage: TaskStorage::new(task_dir),
        }
    }

    pub fn archive_completed(&self, dry_run: bool) -> Result<Vec<u32>> {
        let tasks = self.storage.load_all()?;
        let mut archived = Vec::new();

        let archive_dir = self.storage.task_dir.join("archive");
        if !dry_run && !archive_dir.exists() {
            std::fs::create_dir_all(&archive_dir)?;
        }

        for task in tasks {
            if task.status == TaskStatus::Completed {
                if dry_run {
                    println!(
                        "  [DRY RUN] Would archive task #{}: {}",
                        task.id, task.subject
                    );
                } else {
                    let source_path = self.storage.task_path(task.id);
                    let dest_path = archive_dir.join(format!("{}.md", task.id));

                    std::fs::rename(&source_path, &dest_path)?;
                    println!("  Archived task #{}: {}", task.id, task.subject);
                }

                archived.push(task.id);
            }
        }

        Ok(archived)
    }
}
