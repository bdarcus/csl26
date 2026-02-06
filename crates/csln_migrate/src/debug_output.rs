/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Formats provenance debug output for display.

use crate::provenance::{ProvenanceTracker, TransformationEvent};

pub struct DebugOutputFormatter;

impl DebugOutputFormatter {
    /// Format debug output for a specific variable
    pub fn format_variable(tracker: &ProvenanceTracker, var_name: &str) -> String {
        match tracker.get_provenance(var_name) {
            Some(provenance) => {
                let mut output = String::new();
                output.push_str(&format!("Variable: {}\n", var_name));
                output.push('\n');

                // Group events by category
                let mut source_nodes = Vec::new();
                let mut transformations = Vec::new();
                let mut placements = Vec::new();

                for event in &provenance.events {
                    match event {
                        TransformationEvent::SourceElement { .. } => source_nodes.push(event),
                        TransformationEvent::TemplatePlacement { .. } => placements.push(event),
                        _ => transformations.push(event),
                    }
                }

                // Source CSL nodes
                if !source_nodes.is_empty() {
                    output.push_str("Source CSL nodes:\n");
                    for (i, event) in source_nodes.iter().enumerate() {
                        output.push_str(&format!("  {}. {}\n", i + 1, event));
                    }
                    output.push('\n');
                }

                // Transformations
                if !transformations.is_empty() {
                    output.push_str("Transformations:\n");
                    for event in transformations {
                        output.push_str(&format!("  - {}\n", event));
                    }
                    output.push('\n');
                }

                // Template placement
                if !placements.is_empty() {
                    let placements_count = placements.len();
                    output.push_str("Compiled to:\n");
                    for event in placements {
                        output.push_str(&format!("  - {}\n", event));
                    }
                    output.push_str("\nSummary:\n");
                    output.push_str(&format!(
                        "  Total transformations: {}\n",
                        provenance.events.len()
                    ));
                    output.push_str(&format!("  Source nodes found: {}\n", source_nodes.len()));
                    output.push_str(&format!("  Template placements: {}\n", placements_count));
                }

                output
            }
            None => {
                format!(
                    "Variable '{}' not found in provenance.\n\nAvailable variables:\n",
                    var_name
                ) + &Self::format_available_variables(tracker)
            }
        }
    }

    /// Format list of available variables
    pub fn format_available_variables(tracker: &ProvenanceTracker) -> String {
        let mut vars: Vec<_> = tracker.get_all_variables();
        vars.sort();

        if vars.is_empty() {
            "  (none tracked)\n".to_string()
        } else {
            vars.iter()
                .enumerate()
                .map(|(i, v)| format!("  {}. {}\n", i + 1, v))
                .collect::<String>()
        }
    }

    /// Format full debug report for all tracked variables
    pub fn format_all_variables(tracker: &ProvenanceTracker) -> String {
        let mut vars: Vec<_> = tracker.get_all_variables();
        vars.sort();

        if vars.is_empty() {
            return "No variables tracked.\n".to_string();
        }

        let mut output = format!("Tracked {} variables:\n\n", vars.len());

        for (i, var) in vars.iter().enumerate() {
            if i > 0 {
                output.push('\n');
                output.push_str("---\n\n");
            }
            output.push_str(&Self::format_variable(tracker, var));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::SourceLocation;
    use std::collections::HashMap;

    #[test]
    fn test_format_variable() {
        let tracker = ProvenanceTracker::new(true);
        let loc = SourceLocation {
            line: 42,
            column: 10,
            context: "macro 'label-volume'".to_string(),
        };

        tracker.record_source_element("volume", loc, "text", HashMap::new());

        tracker.record_upsampling("volume", "Text", "Variable");
        tracker.record_template_placement("volume", 4, "bibliography.template", "Number");

        let output = DebugOutputFormatter::format_variable(&tracker, "volume");
        assert!(output.contains("Variable: volume"));
        assert!(output.contains("Source CSL nodes"));
        assert!(output.contains("Transformations"));
        assert!(output.contains("Compiled to"));
    }

    #[test]
    fn test_format_unknown_variable() {
        let tracker = ProvenanceTracker::new(true);
        let output = DebugOutputFormatter::format_variable(&tracker, "unknown");
        assert!(output.contains("Variable 'unknown' not found"));
    }
}
