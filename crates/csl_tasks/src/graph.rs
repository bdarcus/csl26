use crate::task::Task;
use std::collections::HashMap;

pub fn render_ascii(tasks: &[Task]) -> String {
    let mut output = String::new();
    let task_map: HashMap<u32, &Task> = tasks.iter().map(|t| (t.id, t)).collect();

    output.push_str("Task Dependency Graph\n");
    output.push_str("═══════════════════════\n\n");

    for task in tasks {
        let status_symbol = match task.status {
            crate::task::TaskStatus::Pending => "○",
            crate::task::TaskStatus::InProgress => "◐",
            crate::task::TaskStatus::Completed => "●",
            crate::task::TaskStatus::Deleted => "✗",
        };

        output.push_str(&format!(
            "{} Task #{}: {}\n",
            status_symbol, task.id, task.subject
        ));

        if !task.blocked_by.is_empty() {
            output.push_str("  │\n");
            output.push_str("  ├─ Blocked by:\n");
            for &dep_id in &task.blocked_by {
                if let Some(dep_task) = task_map.get(&dep_id) {
                    output.push_str(&format!("  │  └─ #{}: {}\n", dep_id, dep_task.subject));
                } else {
                    output.push_str(&format!("  │  └─ #{} (missing)\n", dep_id));
                }
            }
        }

        if !task.blocks.is_empty() {
            output.push_str("  │\n");
            output.push_str("  └─ Blocks:\n");
            for &blocked_id in &task.blocks {
                if let Some(blocked_task) = task_map.get(&blocked_id) {
                    output.push_str(&format!(
                        "     └─ #{}: {}\n",
                        blocked_id, blocked_task.subject
                    ));
                } else {
                    output.push_str(&format!("     └─ #{} (missing)\n", blocked_id));
                }
            }
        }

        output.push('\n');
    }

    output
}

pub fn render_dot(tasks: &[Task]) -> String {
    let mut output = String::new();

    output.push_str("digraph tasks {\n");
    output.push_str("  rankdir=TB;\n");
    output.push_str("  node [shape=box, style=rounded];\n\n");

    for task in tasks {
        let color = match task.status {
            crate::task::TaskStatus::Pending => "lightblue",
            crate::task::TaskStatus::InProgress => "yellow",
            crate::task::TaskStatus::Completed => "lightgreen",
            crate::task::TaskStatus::Deleted => "lightgray",
        };

        let label = task
            .subject
            .chars()
            .take(40)
            .collect::<String>()
            .replace('"', "\\\"");

        output.push_str(&format!(
            "  task_{} [label=\"#{}: {}\", fillcolor={}, style=\"rounded,filled\"];\n",
            task.id, task.id, label, color
        ));
    }

    output.push('\n');

    for task in tasks {
        for &blocked_id in &task.blocks {
            output.push_str(&format!("  task_{} -> task_{};\n", task.id, blocked_id));
        }
    }

    output.push_str("}\n");

    output
}
