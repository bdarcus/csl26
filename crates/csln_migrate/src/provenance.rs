/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Provenance tracking for variable migration.
//!
//! Tracks the journey of a variable through the compilation pipeline:
//! CSL source → macro expansion → upsampling → compression → compilation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A location in the source CSL document
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub context: String, // e.g., "macro 'label-volume'" or "bibliography layout"
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {}, column {} ({})",
            self.line, self.column, self.context
        )
    }
}

/// A transformation event in the pipeline
#[derive(Debug, Clone)]
pub enum TransformationEvent {
    /// Found in source CSL element
    SourceElement {
        location: SourceLocation,
        element_type: String, // "text", "number", "date", etc.
        attributes: HashMap<String, String>,
    },
    /// Expanded from macro
    MacroExpansion {
        macro_name: String,
        source: SourceLocation,
    },
    /// Upsampled to CSLN representation
    Upsampled { from_type: String, to_type: String },
    /// Merged with another node
    Merged {
        with: Vec<SourceLocation>,
        reason: String,
    },
    /// Placed in final template
    TemplatePlacement {
        index: usize,
        position: String, // e.g., "bibliography.template[4]"
        component_type: String,
    },
    /// Applied type-specific override
    TypeOverride {
        item_type: String,
        override_reason: String,
    },
}

impl std::fmt::Display for TransformationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformationEvent::SourceElement {
                location,
                element_type,
                ..
            } => write!(f, "{} element at {}", element_type, location),
            TransformationEvent::MacroExpansion { macro_name, source } => {
                write!(f, "Expanded from macro '{}' at {}", macro_name, source)
            }
            TransformationEvent::Upsampled { from_type, to_type } => {
                write!(f, "Upsampled from {} to {}", from_type, to_type)
            }
            TransformationEvent::Merged { with, reason } => {
                write!(f, "Merged ({}) with {} location(s)", reason, with.len())
            }
            TransformationEvent::TemplatePlacement {
                index,
                position,
                component_type,
            } => write!(
                f,
                "{} component at index {} in {}",
                component_type, index, position
            ),
            TransformationEvent::TypeOverride {
                item_type,
                override_reason,
            } => write!(f, "Type override for {}: {}", item_type, override_reason),
        }
    }
}

/// Tracks the provenance of a single variable through the pipeline
#[derive(Debug, Clone)]
pub struct VariableProvenance {
    pub variable_name: String,
    pub events: Vec<TransformationEvent>,
}

/// Thread-local provenance tracker
#[derive(Clone)]
pub struct ProvenanceTracker {
    inner: Arc<ProvenanceTrackerInner>,
}

struct ProvenanceTrackerInner {
    variables: Mutex<HashMap<String, VariableProvenance>>,
    enabled: bool,
}

impl ProvenanceTracker {
    pub fn new(enabled: bool) -> Self {
        Self {
            inner: Arc::new(ProvenanceTrackerInner {
                variables: Mutex::new(HashMap::new()),
                enabled,
            }),
        }
    }

    pub fn record_source_element(
        &self,
        var_name: &str,
        location: SourceLocation,
        element_type: &str,
        attributes: HashMap<String, String>,
    ) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::SourceElement {
                    location,
                    element_type: element_type.to_string(),
                    attributes,
                });
        }
    }

    pub fn record_macro_expansion(&self, var_name: &str, macro_name: &str, source: SourceLocation) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::MacroExpansion {
                    macro_name: macro_name.to_string(),
                    source,
                });
        }
    }

    pub fn record_upsampling(&self, var_name: &str, from_type: &str, to_type: &str) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::Upsampled {
                    from_type: from_type.to_string(),
                    to_type: to_type.to_string(),
                });
        }
    }

    pub fn record_merge(&self, var_name: &str, with_locations: Vec<SourceLocation>, reason: &str) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::Merged {
                    with: with_locations,
                    reason: reason.to_string(),
                });
        }
    }

    pub fn record_template_placement(
        &self,
        var_name: &str,
        index: usize,
        position: &str,
        component_type: &str,
    ) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::TemplatePlacement {
                    index,
                    position: position.to_string(),
                    component_type: component_type.to_string(),
                });
        }
    }

    pub fn record_type_override(&self, var_name: &str, item_type: &str, reason: &str) {
        if !self.inner.enabled {
            return;
        }

        if let Ok(mut vars) = self.inner.variables.lock() {
            vars.entry(var_name.to_string())
                .or_insert_with(|| VariableProvenance {
                    variable_name: var_name.to_string(),
                    events: Vec::new(),
                })
                .events
                .push(TransformationEvent::TypeOverride {
                    item_type: item_type.to_string(),
                    override_reason: reason.to_string(),
                });
        }
    }

    pub fn get_provenance(&self, var_name: &str) -> Option<VariableProvenance> {
        if !self.inner.enabled {
            return None;
        }

        self.inner
            .variables
            .lock()
            .ok()
            .and_then(|vars| vars.get(var_name).cloned())
    }

    pub fn get_all_variables(&self) -> Vec<String> {
        self.inner
            .variables
            .lock()
            .ok()
            .map(|vars| vars.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_tracking() {
        let tracker = ProvenanceTracker::new(true);
        let loc = SourceLocation {
            line: 42,
            column: 10,
            context: "macro 'label-volume'".to_string(),
        };

        tracker.record_source_element(
            "volume",
            loc.clone(),
            "text",
            std::collections::HashMap::new(),
        );

        let prov = tracker.get_provenance("volume").unwrap();
        assert_eq!(prov.variable_name, "volume");
        assert_eq!(prov.events.len(), 1);
    }

    #[test]
    fn test_disabled_tracking() {
        let tracker = ProvenanceTracker::new(false);
        let loc = SourceLocation {
            line: 42,
            column: 10,
            context: "test".to_string(),
        };

        tracker.record_source_element("volume", loc, "text", std::collections::HashMap::new());

        assert!(tracker.get_provenance("volume").is_none());
    }
}
