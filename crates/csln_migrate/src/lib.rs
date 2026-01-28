use csl_legacy::model::{Style, CslNode, Macro};
use std::collections::HashMap;

pub mod upsampler;
pub mod compressor;
pub use upsampler::Upsampler;
pub use compressor::Compressor;
pub struct MacroInliner {
    macros: HashMap<String, Vec<CslNode>>,
}

impl MacroInliner {
    pub fn new(style: &Style) -> Self {
        let mut macros = HashMap::new();
        for m in &style.macros {
            macros.insert(m.name.clone(), m.children.clone());
        }
        Self { macros }
    }

    /// Recursively expands all macro calls in a list of nodes.
    pub fn expand_nodes(&self, nodes: &[CslNode]) -> Vec<CslNode> {
        let mut expanded = Vec::new();
        for node in nodes {
            match node {
                CslNode::Text(text) if text.macro_name.is_some() => {
                    let name = text.macro_name.as_ref().unwrap();
                    if let Some(macro_children) = self.macros.get(name) {
                        // Recursively expand the macro's children in case it calls other macros
                        expanded.extend(self.expand_nodes(macro_children));
                    } else {
                        // If macro not found, keep the original node (might be an error in the style)
                        expanded.push(node.clone());
                    }
                }
                // For other nodes that have children, we must recurse into them
                CslNode::Group(group) => {
                    let mut new_group = group.clone();
                    new_group.children = self.expand_nodes(&group.children);
                    expanded.push(CslNode::Group(new_group));
                }
                CslNode::Names(names) => {
                    let mut new_names = names.clone();
                    new_names.children = self.expand_nodes(&names.children);
                    expanded.push(CslNode::Names(new_names));
                }
                CslNode::Choose(choose) => {
                    let mut new_choose = choose.clone();
                    new_choose.if_branch.children = self.expand_nodes(&choose.if_branch.children);
                    for branch in &mut new_choose.else_if_branches {
                        branch.children = self.expand_nodes(&branch.children);
                    }
                    if let Some(ref else_children) = choose.else_branch {
                        new_choose.else_branch = Some(self.expand_nodes(else_children));
                    }
                    expanded.push(CslNode::Choose(new_choose));
                }
                CslNode::Substitute(sub) => {
                    let mut new_sub = sub.clone();
                    new_sub.children = self.expand_nodes(&sub.children);
                    expanded.push(CslNode::Substitute(new_sub));
                }
                // Nodes with no children or that don't call macros directly
                _ => expanded.push(node.clone()),
            }
        }
        expanded
    }

    /// Returns a version of the bibliography layout with all macros inlined.
    pub fn inline_bibliography(&self, style: &Style) -> Option<Vec<CslNode>> {
        style.bibliography.as_ref().map(|bib| self.expand_nodes(&bib.layout.children))
    }

    /// Returns a version of the citation layout with all macros inlined.
    pub fn inline_citation(&self, style: &Style) -> Vec<CslNode> {
        self.expand_nodes(&style.citation.layout.children)
    }
}
