use crate::task::{Task, TaskStatus};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum DriftType {
    ContentDrift {
        task_id: u32,
        issue_num: u64,
        local_hash: String,
        remote_hash: String,
    },
    StatusMismatch {
        task_id: u32,
        issue_num: u64,
        local_status: TaskStatus,
        remote_status: TaskStatus,
    },
    OrphanedTask {
        task_id: u32,
    },
    OrphanedIssue {
        issue_num: u64,
        title: String,
    },
    DependencyDrift {
        task_id: u32,
        issue_num: u64,
        local_blocks: Vec<u32>,
        remote_blocks: Vec<u32>,
    },
}

#[derive(Debug)]
pub struct DriftReport {
    pub drifts: Vec<DriftType>,
}

impl DriftReport {
    pub fn new() -> Self {
        Self { drifts: Vec::new() }
    }

    pub fn add(&mut self, drift: DriftType) {
        self.drifts.push(drift);
    }

    pub fn has_drift(&self) -> bool {
        !self.drifts.is_empty()
    }

    pub fn content_drift_count(&self) -> usize {
        self.drifts
            .iter()
            .filter(|d| matches!(d, DriftType::ContentDrift { .. }))
            .count()
    }

    pub fn status_mismatch_count(&self) -> usize {
        self.drifts
            .iter()
            .filter(|d| matches!(d, DriftType::StatusMismatch { .. }))
            .count()
    }

    pub fn orphaned_task_count(&self) -> usize {
        self.drifts
            .iter()
            .filter(|d| matches!(d, DriftType::OrphanedTask { .. }))
            .count()
    }

    pub fn orphaned_issue_count(&self) -> usize {
        self.drifts
            .iter()
            .filter(|d| matches!(d, DriftType::OrphanedIssue { .. }))
            .count()
    }

    pub fn dependency_drift_count(&self) -> usize {
        self.drifts
            .iter()
            .filter(|d| matches!(d, DriftType::DependencyDrift { .. }))
            .count()
    }
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DriftError {
    #[error("drift resolution cancelled by user")]
    Cancelled,
}

pub fn detect_drift(local_tasks: &[Task], remote_issues: &[(u64, Task, String)]) -> DriftReport {
    let mut report = DriftReport::new();

    let local_map: HashMap<u32, &Task> = local_tasks.iter().map(|t| (t.id, t)).collect();

    let remote_map: HashMap<u32, (u64, &Task, &str)> = remote_issues
        .iter()
        .map(|(issue_num, task, hash)| (task.id, (*issue_num, task, hash.as_str())))
        .collect();

    for local_task in local_tasks {
        if let Some(issue_num) = local_task.github_issue {
            if let Some((_, remote_task, remote_hash)) = remote_map.get(&local_task.id) {
                if local_task.content_hash != *remote_hash {
                    report.add(DriftType::ContentDrift {
                        task_id: local_task.id,
                        issue_num: issue_num as u64,
                        local_hash: local_task.content_hash.clone(),
                        remote_hash: remote_hash.to_string(),
                    });
                }

                if local_task.status != remote_task.status {
                    report.add(DriftType::StatusMismatch {
                        task_id: local_task.id,
                        issue_num: issue_num as u64,
                        local_status: local_task.status.clone(),
                        remote_status: remote_task.status.clone(),
                    });
                }

                if local_task.blocks != remote_task.blocks {
                    report.add(DriftType::DependencyDrift {
                        task_id: local_task.id,
                        issue_num: issue_num as u64,
                        local_blocks: local_task.blocks.clone(),
                        remote_blocks: remote_task.blocks.clone(),
                    });
                }
            }
        } else if local_map.contains_key(&local_task.id) {
            report.add(DriftType::OrphanedTask {
                task_id: local_task.id,
            });
        }
    }

    for (issue_num, remote_task, _) in remote_issues {
        if !local_map.contains_key(&remote_task.id) {
            report.add(DriftType::OrphanedIssue {
                issue_num: *issue_num,
                title: remote_task.subject.clone(),
            });
        }
    }

    report
}
