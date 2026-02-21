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
pub mod template_resolver;
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

    /// Expands macros starting from a specific order counter value.
    /// Used when layout macros have pre-assigned orders and nested macros
    /// should continue numbering from where layout assignment left off.
    fn expand_nodes_from_order(&self, nodes: &[CslNode], initial_order: usize) -> Vec<CslNode> {
        let mut order_counter = initial_order;
        self.expand_nodes_with_order(nodes, &mut order_counter)
    }

    /// Expand macros without incrementing order counter (for nested macros).
    /// Nested macros inherit their parent's order.
    fn expand_macros_no_increment(&self, nodes: &[CslNode]) -> Vec<CslNode> {
        let mut expanded = Vec::new();
        for node in nodes {
            match node {
                CslNode::Text(text) if text.macro_name.is_some() => {
                    let name = text.macro_name.as_ref().unwrap();
                    if let Some(macro_children) = self.macros.get(name) {
                        // Recursively expand nested macros without incrementing
                        let expanded_children = self.expand_macros_no_increment(macro_children);
                        expanded.extend(expanded_children);
                    } else {
                        expanded.push(node.clone());
                    }
                }
                CslNode::Group(group) => {
                    let mut new_group = group.clone();
                    new_group.children = self.expand_macros_no_increment(&group.children);
                    expanded.push(CslNode::Group(new_group));
                }
                CslNode::Names(names) => {
                    let mut new_names = names.clone();
                    new_names.children = self.expand_macros_no_increment(&names.children);
                    expanded.push(CslNode::Names(new_names));
                }
                CslNode::Choose(choose) => {
                    let mut new_choose = choose.clone();
                    new_choose.if_branch.children =
                        self.expand_macros_no_increment(&choose.if_branch.children);
                    for else_if in &mut new_choose.else_if_branches {
                        else_if.children = self.expand_macros_no_increment(&else_if.children);
                    }
                    if let Some(ref else_branch) = new_choose.else_branch {
                        new_choose.else_branch = Some(self.expand_macros_no_increment(else_branch));
                    }
                    expanded.push(CslNode::Choose(new_choose));
                }
                CslNode::Substitute(sub) => {
                    let mut new_sub = sub.clone();
                    new_sub.children = self.expand_macros_no_increment(&sub.children);
                    expanded.push(CslNode::Substitute(new_sub));
                }
                _ => {
                    expanded.push(node.clone());
                }
            }
        }
        expanded
    }

    /// Internal method that tracks macro call order during expansion.
    /// The order_counter is incremented each time a TOP-LEVEL macro is expanded,
    /// and all nodes within that macro inherit the same order value.
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
                        // Check if this macro call has a pre-assigned order from the layout
                        let is_layout_macro = text.macro_call_order.is_some();

                        let current_order = if is_layout_macro {
                            // This is a layout-level macro call - use its pre-assigned order
                            text.macro_call_order.unwrap()
                        } else {
                            // This is a nested macro - assign it the current counter value
                            let order = *order_counter;
                            *order_counter += 1;
                            order
                        };

                        // If this is a layout-level macro, expand its children WITH order tracking
                        // so that nested macro calls get their own order numbers.
                        // Otherwise, expand without tracking to inherit the parent's order.
                        let expanded_children = if is_layout_macro {
                            self.expand_nodes_with_order(macro_children, order_counter)
                        } else {
                            let children = self.expand_macros_no_increment(macro_children);
                            // For non-layout macros, assign the parent's order only to nodes that don't already have one
                            // This preserves orders from nested macros while filling in orders for non-macro nodes
                            children
                                .into_iter()
                                .map(|mut child| {
                                    self.assign_macro_order_if_none(&mut child, current_order);
                                    child
                                })
                                .collect()
                        };

                        // For layout macros, children already have their orders from nested expansion
                        // For non-layout macros, children have been assigned the parent's order above
                        expanded.extend(expanded_children);
                    } else {
                        // If macro not found, keep the original node (might be an error in the style)
                        expanded.push(node.clone());
                    }
                }
                // For other nodes that have children, we must recurse into them
                CslNode::Group(group) => {
                    let mut new_group = group.clone();
                    new_group.children =
                        self.expand_nodes_with_order(&group.children, order_counter);
                    expanded.push(CslNode::Group(new_group));
                }
                CslNode::Names(names) => {
                    let mut new_names = names.clone();
                    new_names.children =
                        self.expand_nodes_with_order(&names.children, order_counter);
                    expanded.push(CslNode::Names(new_names));
                }
                CslNode::Choose(choose) => {
                    let mut new_choose = choose.clone();
                    // For macro order tracking, we want sequential orders across ALL branches.
                    // This tracks the SOURCE order of macro calls, not runtime execution order.
                    // At runtime only one branch executes, but we need to track where each
                    // macro appeared in the CSL 1.0 source.

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
                // Nodes with no children or that don't call macros directly
                _ => expanded.push(node.clone()),
            }
        }
        expanded
    }

    /// Assigns macro_call_order only if not already set.
    /// This ensures nested macro nodes keep their own order while non-macro nodes get the parent order.
    fn assign_macro_order_if_none(&self, node: &mut CslNode, order: usize) {
        match node {
            CslNode::Text(text) if text.macro_call_order.is_none() => {
                text.macro_call_order = Some(order);
            }
            CslNode::Date(date) if date.macro_call_order.is_none() => {
                date.macro_call_order = Some(order);
            }
            CslNode::Label(label) if label.macro_call_order.is_none() => {
                label.macro_call_order = Some(order);
            }
            CslNode::Names(names) => {
                if names.macro_call_order.is_none() {
                    names.macro_call_order = Some(order);
                }
                // Recursively assign to children
                for child in &mut names.children {
                    self.assign_macro_order_if_none(child, order);
                }
            }
            CslNode::Group(group) => {
                if group.macro_call_order.is_none() {
                    group.macro_call_order = Some(order);
                }
                // Recursively assign to children
                for child in &mut group.children {
                    self.assign_macro_order_if_none(child, order);
                }
            }
            CslNode::Number(number) if number.macro_call_order.is_none() => {
                number.macro_call_order = Some(order);
            }
            CslNode::Choose(choose) => {
                // Recursively assign to all branches
                for child in &mut choose.if_branch.children {
                    self.assign_macro_order_if_none(child, order);
                }
                for branch in &mut choose.else_if_branches {
                    for child in &mut branch.children {
                        self.assign_macro_order_if_none(child, order);
                    }
                }
                if let Some(ref mut else_children) = choose.else_branch {
                    for child in else_children {
                        self.assign_macro_order_if_none(child, order);
                    }
                }
            }
            CslNode::Substitute(sub) => {
                // Recursively assign to children
                for child in &mut sub.children {
                    self.assign_macro_order_if_none(child, order);
                }
            }
            _ => {}
        }
    }

    /// Assigns macro_call_order to a node and all its descendants.
    /// This ensures all nodes within an expanded macro inherit the macro's order.
    #[allow(dead_code)]
    fn assign_macro_order(&self, node: &mut CslNode, order: usize) {
        match node {
            CslNode::Text(text) => {
                text.macro_call_order = Some(order);
            }
            CslNode::Date(date) => {
                date.macro_call_order = Some(order);
            }
            CslNode::Label(label) => {
                label.macro_call_order = Some(order);
            }
            CslNode::Names(names) => {
                names.macro_call_order = Some(order);
                // Recursively assign to children
                for child in &mut names.children {
                    self.assign_macro_order(child, order);
                }
            }
            CslNode::Group(group) => {
                group.macro_call_order = Some(order);
                // Recursively assign to children
                for child in &mut group.children {
                    self.assign_macro_order(child, order);
                }
            }
            CslNode::Number(number) => {
                number.macro_call_order = Some(order);
            }
            CslNode::Choose(choose) => {
                // Recursively assign to all branches
                for child in &mut choose.if_branch.children {
                    self.assign_macro_order(child, order);
                }
                for branch in &mut choose.else_if_branches {
                    for child in &mut branch.children {
                        self.assign_macro_order(child, order);
                    }
                }
                if let Some(else_children) = &mut choose.else_branch {
                    for child in else_children {
                        self.assign_macro_order(child, order);
                    }
                }
            }
            CslNode::Substitute(sub) => {
                // Recursively assign to children
                for child in &mut sub.children {
                    self.assign_macro_order(child, order);
                }
            }
            _ => {}
        }
    }

    /// Assign layout order to macro call nodes before expansion.
    /// This assigns a sequential order to each `<text macro="..."/>` node based on its
    /// position in the layout, which will be preserved during macro expansion.
    fn assign_layout_order(&self, nodes: &mut [CslNode], order_counter: &mut usize) {
        for node in nodes {
            match node {
                CslNode::Text(text) if text.macro_name.is_some() => {
                    // This is a macro call - assign it the current order
                    text.macro_call_order = Some(*order_counter);
                    *order_counter += 1;
                }
                CslNode::Group(group) => {
                    // Recurse into groups to find macro calls
                    self.assign_layout_order(&mut group.children, order_counter);
                }
                CslNode::Choose(choose) => {
                    // For layout order assignment, we want ALL macro calls to get unique sequential orders
                    // even across Choose branches, because we're tracking SOURCE order, not runtime order.
                    // At runtime only one branch executes, but we need to track where each macro appeared
                    // in the CSL 1.0 source.
                    self.assign_layout_order(&mut choose.if_branch.children, order_counter);

                    for branch in &mut choose.else_if_branches {
                        self.assign_layout_order(&mut branch.children, order_counter);
                    }

                    if let Some(else_children) = &mut choose.else_branch {
                        self.assign_layout_order(else_children, order_counter);
                    }
                }
                CslNode::Names(names) => {
                    self.assign_layout_order(&mut names.children, order_counter);
                }
                CslNode::Substitute(sub) => {
                    self.assign_layout_order(&mut sub.children, order_counter);
                }
                _ => {}
            }
        }
    }

    /// Returns a version of the bibliography layout with all macros inlined.
    pub fn inline_bibliography(&self, style: &Style) -> Option<Vec<CslNode>> {
        style.bibliography.as_ref().map(|bib| {
            // Clone the layout children so we can mutate them
            let mut layout_nodes = bib.layout.children.clone();

            // Assign order to layout macro calls before expansion
            let mut order_counter = 0;
            self.assign_layout_order(&mut layout_nodes, &mut order_counter);

            // Expand macros, starting nested macro numbering from where layout assignment left off.
            // This prevents collisions between layout macro orders and nested macro orders.
            self.expand_nodes_from_order(&layout_nodes, order_counter)
        })
    }

    /// Returns a version of the citation layout with all macros inlined.
    pub fn inline_citation(&self, style: &Style) -> Vec<CslNode> {
        self.expand_nodes(&style.citation.layout.children)
    }
}
