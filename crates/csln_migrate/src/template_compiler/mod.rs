/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Compiles legacy CslnNode trees into CSLN TemplateComponents.

pub mod dates;
pub mod group;
pub mod names;
pub mod text;

#[cfg(test)]
mod tests;

use csln_core::{
    template::{Rendering, TemplateComponent, TemplateList},
    CslnNode, ItemType,
};
use std::collections::HashMap;

/// Compiles CslnNode trees into TemplateComponents.
pub struct TemplateCompiler;

impl TemplateCompiler {
    pub fn compile(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let no_wrap = (None, None, None);
        self.compile_with_wrap(nodes, &no_wrap, &[])
    }

    pub fn compile_citation(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        self.compile(nodes) // Simplified for now
    }

    pub fn compile_bibliography_with_types(
        &self,
        nodes: &[CslnNode],
        is_numeric: bool,
    ) -> (
        Vec<TemplateComponent>,
        HashMap<String, Vec<TemplateComponent>>,
    ) {
        // Compile the default template
        let mut default_template = self.compile(nodes);

        // Deduplicate and flatten to remove redundant nesting from branch processing
        default_template = self.deduplicate_and_flatten(default_template);

        self.sort_bibliography_components(&mut default_template, is_numeric);

        // Type-specific template generation is disabled for now.
        let type_templates: HashMap<String, Vec<TemplateComponent>> = HashMap::new();

        (default_template, type_templates)
    }

    fn compile_with_wrap(
        &self,
        nodes: &[CslnNode],
        _inherited_wrap: &(
            Option<csln_core::template::WrapPunctuation>,
            Option<String>,
            Option<String>,
        ),
        _current_types: &[ItemType],
    ) -> Vec<TemplateComponent> {
        let mut components = Vec::new();
        for node in nodes {
            if let Some(component) = self.compile_node(node) {
                components.push(component);
            }
        }
        components
    }

    fn compile_node(&self, node: &CslnNode) -> Option<TemplateComponent> {
        match node {
            CslnNode::Variable(v) => text::compile_variable(v),
            CslnNode::Names(n) => names::compile_names(n),
            CslnNode::Date(d) => dates::compile_date(d),
            // ... other node types
            _ => None,
        }
    }

    // Public helpers for tests
    pub fn compile_names(&self, names: &csln_core::NamesBlock) -> Option<TemplateComponent> {
        names::compile_names(names)
    }

    pub fn compile_date(&self, date: &csln_core::DateBlock) -> Option<TemplateComponent> {
        dates::compile_date(date)
    }

    fn deduplicate_and_flatten(
        &self,
        components: Vec<TemplateComponent>,
    ) -> Vec<TemplateComponent> {
        let mut seen_vars: Vec<String> = Vec::new();
        let mut seen_list_signatures: Vec<String> = Vec::new();
        let mut result: Vec<TemplateComponent> = Vec::new();

        // First pass: add all non-List components and track their keys
        for component in &components {
            if !matches!(component, TemplateComponent::List(_)) {
                if let Some(key) = self.get_variable_key(component) {
                    if !seen_vars.contains(&key) {
                        seen_vars.push(key);
                        result.push(component.clone());
                    }
                } else {
                    result.push(component.clone());
                }
            }
        }

        // Second pass: process Lists with recursive cleaning
        for component in components {
            if let TemplateComponent::List(list) = component {
                // Recursively clean the list
                if let Some(cleaned) = self.clean_list_recursive(&list, &seen_vars) {
                    // Check if it's a List or was unwrapped
                    if let TemplateComponent::List(cleaned_list) = &cleaned {
                        // Create signature for duplicate detection
                        let list_vars = self.extract_list_vars(cleaned_list);
                        let mut signature_parts = list_vars.clone();
                        signature_parts.sort();
                        let signature = signature_parts.join("|");

                        // Skip duplicate lists
                        if seen_list_signatures.contains(&signature) {
                            continue;
                        }
                        seen_list_signatures.push(signature);

                        // Track variables in this list
                        for var in list_vars {
                            if !seen_vars.contains(&var) {
                                seen_vars.push(var);
                            }
                        }
                    } else if let Some(key) = self.get_variable_key(&cleaned) {
                        // If it was unwrapped to a single component, check if already seen
                        if seen_vars.contains(&key) {
                            continue;
                        }
                        seen_vars.push(key);
                    }

                    result.push(cleaned);
                }
            }
        }

        result
    }

    fn clean_list_recursive(
        &self,
        list: &TemplateList,
        seen_vars: &[String],
    ) -> Option<TemplateComponent> {
        let mut cleaned_items: Vec<TemplateComponent> = Vec::new();

        for item in &list.items {
            if let TemplateComponent::List(nested) = item {
                // Recursively clean nested lists
                if let Some(cleaned) = self.clean_list_recursive(nested, seen_vars) {
                    cleaned_items.push(cleaned);
                }
            } else if let Some(key) = self.get_variable_key(item) {
                // Only keep if not already seen
                if !seen_vars.contains(&key) {
                    cleaned_items.push(item.clone());
                }
            } else {
                // Keep other items (shouldn't happen often)
                cleaned_items.push(item.clone());
            }
        }

        // Skip empty lists
        if cleaned_items.is_empty() {
            return None;
        }

        // If only one item remains and no special rendering, unwrap it
        if cleaned_items.len() == 1
            && list.delimiter.is_none()
            && list.rendering == Rendering::default()
        {
            return Some(cleaned_items.remove(0));
        }

        Some(TemplateComponent::List(TemplateList {
            items: cleaned_items,
            delimiter: list.delimiter,
            rendering: list.rendering.clone(),
            ..Default::default()
        }))
    }

    fn extract_list_vars(&self, list: &TemplateList) -> Vec<String> {
        let mut vars = Vec::new();
        for item in &list.items {
            if let Some(key) = self.get_variable_key(item) {
                vars.push(key);
            } else if let TemplateComponent::List(nested) = item {
                vars.extend(self.extract_list_vars(nested));
            }
        }
        vars
    }

    fn get_variable_key(&self, component: &TemplateComponent) -> Option<String> {
        // Reuse logic from processor or implement simplified version here
        match component {
            TemplateComponent::Contributor(c) => Some(format!("contributor:{:?}", c.contributor)),
            TemplateComponent::Date(d) => Some(format!("date:{:?}", d.date)),
            TemplateComponent::Title(t) => Some(format!("title:{:?}", t.title)),
            TemplateComponent::Number(n) => Some(format!("number:{:?}", n.number)),
            TemplateComponent::Variable(v) => Some(format!("variable:{:?}", v.variable)),
            _ => None,
        }
    }

    fn sort_bibliography_components(&self, components: &mut [TemplateComponent], is_numeric: bool) {
        // Basic sorting: ensure certain components are in logical order if they exist
        // For numeric styles, ensure citation-number is first
        if is_numeric {
            if let Some(idx) = components.iter().position(|c| {
                matches!(c, TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::CitationNumber)
            }) {
                if idx > 0 {
                    // Move to front
                    // Using rotate for simplicity on slice, though vec method would be rotate_left
                    components[0..=idx].rotate_right(1);
                }
            }
        }
    }
}
