use crate::task::Task;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("invalid task reference: task {referencer} references non-existent task {referenced}")]
    InvalidReference { referencer: u32, referenced: u32 },
}

pub fn validate_tasks(tasks: &[Task]) -> Result<(), ValidationError> {
    let task_ids: HashSet<u32> = tasks.iter().map(|t| t.id).collect();

    for task in tasks {
        for &blocked_by_id in &task.blocked_by {
            if !task_ids.contains(&blocked_by_id) {
                return Err(ValidationError::InvalidReference {
                    referencer: task.id,
                    referenced: blocked_by_id,
                });
            }
        }

        for &blocks_id in &task.blocks {
            if !task_ids.contains(&blocks_id) {
                return Err(ValidationError::InvalidReference {
                    referencer: task.id,
                    referenced: blocks_id,
                });
            }
        }
    }

    detect_cycles(tasks)?;

    Ok(())
}

fn detect_cycles(tasks: &[Task]) -> Result<(), ValidationError> {
    let graph: HashMap<u32, Vec<u32>> =
        tasks.iter().map(|t| (t.id, t.blocked_by.clone())).collect();

    for task in tasks {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if has_cycle(&graph, task.id, &mut visited, &mut path) {
            let cycle_str = path
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(ValidationError::CircularDependency(cycle_str));
        }
    }

    Ok(())
}

fn has_cycle(
    graph: &HashMap<u32, Vec<u32>>,
    node: u32,
    visited: &mut HashSet<u32>,
    path: &mut Vec<u32>,
) -> bool {
    if path.contains(&node) {
        path.push(node);
        return true;
    }

    if visited.contains(&node) {
        return false;
    }

    visited.insert(node);
    path.push(node);

    if let Some(dependencies) = graph.get(&node) {
        for &dep in dependencies {
            if has_cycle(graph, dep, visited, path) {
                return true;
            }
        }
    }

    path.pop();
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskStatus;

    #[test]
    fn test_valid_dependencies() {
        let tasks = vec![
            Task {
                id: 1,
                subject: "Task 1".to_string(),
                description: "".to_string(),
                active_form: None,
                status: TaskStatus::Pending,
                blocks: vec![2],
                blocked_by: vec![],
                metadata: Default::default(),
                github_issue: None,
                content_hash: "".to_string(),
            },
            Task {
                id: 2,
                subject: "Task 2".to_string(),
                description: "".to_string(),
                active_form: None,
                status: TaskStatus::Pending,
                blocks: vec![],
                blocked_by: vec![1],
                metadata: Default::default(),
                github_issue: None,
                content_hash: "".to_string(),
            },
        ];

        assert!(validate_tasks(&tasks).is_ok());
    }

    #[test]
    fn test_circular_dependency() {
        let tasks = vec![
            Task {
                id: 1,
                subject: "Task 1".to_string(),
                description: "".to_string(),
                active_form: None,
                status: TaskStatus::Pending,
                blocks: vec![],
                blocked_by: vec![2],
                metadata: Default::default(),
                github_issue: None,
                content_hash: "".to_string(),
            },
            Task {
                id: 2,
                subject: "Task 2".to_string(),
                description: "".to_string(),
                active_form: None,
                status: TaskStatus::Pending,
                blocks: vec![],
                blocked_by: vec![1],
                metadata: Default::default(),
                github_issue: None,
                content_hash: "".to_string(),
            },
        ];

        assert!(matches!(
            validate_tasks(&tasks),
            Err(ValidationError::CircularDependency(_))
        ));
    }

    #[test]
    fn test_invalid_reference() {
        let tasks = vec![Task {
            id: 1,
            subject: "Task 1".to_string(),
            description: "".to_string(),
            active_form: None,
            status: TaskStatus::Pending,
            blocks: vec![],
            blocked_by: vec![99],
            metadata: Default::default(),
            github_issue: None,
            content_hash: "".to_string(),
        }];

        assert!(matches!(
            validate_tasks(&tasks),
            Err(ValidationError::InvalidReference { .. })
        ));
    }
}
