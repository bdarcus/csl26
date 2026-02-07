use csl_legacy::model::{CslNode, Style};
use std::collections::HashMap;

pub mod analysis;
pub mod compressor;
pub mod debug_output;
pub mod options_extractor;
pub mod passes;
pub mod preset_detector;
pub mod provenance;
pub mod template_compiler;
pub mod upsampler;

pub use compressor::Compressor;
pub use debug_output::DebugOutputFormatter;
pub use options_extractor::OptionsExtractor;
pub use preset_detector::{detect_contributor_preset, detect_date_preset, detect_title_preset};
pub use provenance::{ProvenanceTracker, SourceLocation};
pub use template_compiler::TemplateCompiler;
pub use upsampler::Upsampler;
pub struct MacroInliner {
    macros: HashMap<String, Vec<CslNode>>,
    provenance: Option<ProvenanceTracker>,
}

impl MacroInliner {
    pub fn new(style: &Style) -> Self {
        let mut macros = HashMap::new();
        for m in &style.macros {
            macros.insert(m.name.clone(), m.children.clone());
        }
        Self {
            macros,
            provenance: None,
        }
    }

    pub fn with_provenance(style: &Style, provenance: ProvenanceTracker) -> Self {
        let mut macros = HashMap::new();
        for m in &style.macros {
            macros.insert(m.name.clone(), m.children.clone());
        }
        Self {
            macros,
            provenance: Some(provenance),
        }
    }

    pub fn provenance(&self) -> Option<&ProvenanceTracker> {
        self.provenance.as_ref()
    }

    /// Recursively expands all macro calls in a list of nodes.
    pub fn expand_nodes(&self, nodes: &[CslNode]) -> Vec<CslNode> {
        let mut order_counter = 0;
        self.expand_nodes_with_order(nodes, &mut order_counter)
    }

    /// Internal method that assigns sequential orders to nodes during expansion.
    /// The order_counter is incremented for EACH node, creating a depth-first
    /// traversal order that preserves the exact sequence of components as they
    /// appear in the flattened CSL 1.0 layout.
    fn expand_nodes_with_order(
        &self,
        nodes: &[CslNode],
        order_counter: &mut usize,
    ) -> Vec<CslNode> {
        let mut expanded = Vec::new();
        for node in nodes {
            match node {
                CslNode::Text(text) if text.macro_name.is_some() => {
                    let name = text.macro_name.as_ref().unwrap();
                    if let Some(macro_children) = self.macros.get(name) {
                        // Recursively expand and assign sequential orders to all children
                        let expanded_children =
                            self.expand_nodes_with_order(macro_children, order_counter);
                        expanded.extend(expanded_children);
                    } else {
                        // If macro not found, keep the original node (might be an error in the style)
                        expanded.push(node.clone());
                    }
                }
                // For other nodes that have children, recurse and assign orders
                CslNode::Group(group) => {
                    let mut new_group = group.clone();
                    new_group.children =
                        self.expand_nodes_with_order(&group.children, order_counter);
                    new_group.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Group(new_group));
                }
                CslNode::Names(names) => {
                    let mut new_names = names.clone();
                    new_names.children =
                        self.expand_nodes_with_order(&names.children, order_counter);
                    new_names.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Names(new_names));
                }
                CslNode::Choose(choose) => {
                    let mut new_choose = choose.clone();
                    // For sequential ordering, we process all branches in source order.
                    // At runtime only one branch executes, but we track the source order
                    // of all nodes as they appear in the CSL 1.0 file.

                    new_choose.if_branch.children =
                        self.expand_nodes_with_order(&choose.if_branch.children, order_counter);

                    for (idx, branch) in choose.else_if_branches.iter().enumerate() {
                        new_choose.else_if_branches[idx].children =
                            self.expand_nodes_with_order(&branch.children, order_counter);
                    }

                    if let Some(ref else_children) = choose.else_branch {
                        new_choose.else_branch =
                            Some(self.expand_nodes_with_order(else_children, order_counter));
                    }

                    expanded.push(CslNode::Choose(new_choose));
                }
                CslNode::Substitute(sub) => {
                    let mut new_sub = sub.clone();
                    new_sub.children = self.expand_nodes_with_order(&sub.children, order_counter);
                    expanded.push(CslNode::Substitute(new_sub));
                }
                // Leaf nodes - assign sequential order
                CslNode::Text(text) => {
                    let mut new_text = text.clone();
                    new_text.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Text(new_text));
                }
                CslNode::Date(date) => {
                    let mut new_date = date.clone();
                    new_date.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Date(new_date));
                }
                CslNode::Label(label) => {
                    let mut new_label = label.clone();
                    new_label.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Label(new_label));
                }
                CslNode::Number(number) => {
                    let mut new_number = number.clone();
                    new_number.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                    expanded.push(CslNode::Number(new_number));
                }
                _ => expanded.push(node.clone()),
            }
        }
        expanded
    }

    /// Assigns macro_call_order to a node and all its descendants.
    /// This ensures all nodes within an expanded macro inherit the macro's order.
    #[allow(dead_code)]
    fn assign_macro_order(&self, node: &mut CslNode, order: usize) {
        match node {
            CslNode::Text(ref mut text) => {
                text.macro_call_order = Some(order);
            }
            CslNode::Date(ref mut date) => {
                date.macro_call_order = Some(order);
            }
            CslNode::Label(ref mut label) => {
                label.macro_call_order = Some(order);
            }
            CslNode::Names(ref mut names) => {
                names.macro_call_order = Some(order);
                // Recursively assign to children
                for child in &mut names.children {
                    self.assign_macro_order(child, order);
                }
            }
            CslNode::Group(ref mut group) => {
                group.macro_call_order = Some(order);
                // Recursively assign to children
                for child in &mut group.children {
                    self.assign_macro_order(child, order);
                }
            }
            CslNode::Number(ref mut number) => {
                number.macro_call_order = Some(order);
            }
            CslNode::Choose(ref mut choose) => {
                // Recursively assign to all branches
                for child in &mut choose.if_branch.children {
                    self.assign_macro_order(child, order);
                }
                for branch in &mut choose.else_if_branches {
                    for child in &mut branch.children {
                        self.assign_macro_order(child, order);
                    }
                }
                if let Some(ref mut else_children) = choose.else_branch {
                    for child in else_children {
                        self.assign_macro_order(child, order);
                    }
                }
            }
            CslNode::Substitute(ref mut sub) => {
                // Recursively assign to children
                for child in &mut sub.children {
                    self.assign_macro_order(child, order);
                }
            }
            _ => {}
        }
    }

    /// Returns a version of the bibliography layout with all macros inlined.
    /// Each node is assigned a sequential order based on depth-first traversal.
    pub fn inline_bibliography(&self, style: &Style) -> Option<Vec<CslNode>> {
        style
            .bibliography
            .as_ref()
            .map(|bib| self.expand_nodes(&bib.layout.children))
    }

    /// Returns a version of the citation layout with all macros inlined.
    pub fn inline_citation(&self, style: &Style) -> Vec<CslNode> {
        self.expand_nodes(&style.citation.layout.children)
    }
}
