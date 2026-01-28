use crate::{CslnNode, Variable, ItemType, VariableBlock, DateBlock, NamesBlock, GroupBlock, ConditionBlock};
use std::collections::HashMap;

/// A mock citation item with metadata.
pub struct CitationItem {
    pub item_type: ItemType,
    pub variables: HashMap<Variable, String>,
    // In a real engine, dates and names would be structured.
    // Here we use strings for simplicity of the vertical slice.
}

pub struct Renderer;

impl Renderer {
    pub fn render_citation(&self, nodes: &[CslnNode], item: &CitationItem) -> String {
        let mut output = String::new();
        for node in nodes {
            output.push_str(&self.render_node(node, item));
        }
        output
    }

    fn render_node(&self, node: &CslnNode, item: &CitationItem) -> String {
        match node {
            CslnNode::Text { value } => value.clone(),
            CslnNode::Variable(var_block) => self.render_variable(var_block, item),
            CslnNode::Date(date_block) => self.render_date(date_block, item),
            CslnNode::Names(names_block) => self.render_names(names_block, item),
            CslnNode::Group(group_block) => self.render_group(group_block, item),
            CslnNode::Condition(cond_block) => self.render_condition(cond_block, item),
        }
    }

    fn render_variable(&self, block: &VariableBlock, item: &CitationItem) -> String {
        if let Some(val) = item.variables.get(&block.variable) {
            let mut text = val.clone();
            
            // Apply Label
            if let Some(label_opts) = &block.label {
                // Simplistic label rendering
                let prefix = label_opts.formatting.prefix.as_deref().unwrap_or("");
                let suffix = label_opts.formatting.suffix.as_deref().unwrap_or("");
                // Mock label lookup
                let label_text = match block.variable {
                    Variable::Page => "p.",
                    _ => "",
                };
                text = format!("{}{}{}{}", prefix, label_text, suffix, text);
            }

            // Apply Formatting (Prefix/Suffix/Font)
            self.apply_formatting(&text, &block.formatting)
        } else {
            String::new()
        }
    }

    fn render_date(&self, block: &DateBlock, item: &CitationItem) -> String {
        if let Some(val) = item.variables.get(&block.variable) {
            // Mock date rendering
            self.apply_formatting(val, &block.formatting)
        } else {
            String::new()
        }
    }

    fn render_names(&self, block: &NamesBlock, item: &CitationItem) -> String {
        // Try primary variable
        if let Some(val) = item.variables.get(&block.variable) {
            return self.apply_formatting(val, &block.formatting);
        }
        
        // Try substitutes
        for sub_var in &block.options.substitute {
            if let Some(val) = item.variables.get(sub_var) {
                return self.apply_formatting(val, &block.formatting);
            }
        }

        String::new()
    }

    fn render_group(&self, block: &GroupBlock, item: &CitationItem) -> String {
        let mut parts = Vec::new();
        for child in &block.children {
            let rendered = self.render_node(child, item);
            if !rendered.is_empty() {
                parts.push(rendered);
            }
        }

        if parts.is_empty() {
            return String::new();
        }

        let delimiter = block.delimiter.as_deref().unwrap_or("");
        let content = parts.join(delimiter);
        
        self.apply_formatting(&content, &block.formatting)
    }

    fn render_condition(&self, block: &ConditionBlock, item: &CitationItem) -> String {
        // Evaluate IF (OR logic usually in CSL, but let's assume AND for different attributes? 
        // CSL 1.0 <if type="book" variable="author"> is AND.
        // But <if type="book article"> is OR.
        
        let type_match = block.if_item_type.is_empty() || block.if_item_type.contains(&item.item_type);
        
        let var_match = block.if_variables.is_empty() || block.if_variables.iter().any(|v| item.variables.contains_key(v));
        
        // If both lists are empty, it's a "True" block (shouldn't happen in valid CSL but handled here)
        // If one is non-empty, it must match.
        // Wait, if if_item_type is empty, it means "No type constraint".
        
        let match_found = if block.if_item_type.is_empty() && block.if_variables.is_empty() {
            false // Empty condition matches nothing (or maybe it was "is-numeric" which we ignore)
        } else {
            type_match && var_match
        };
        
        if match_found {
            let mut output = String::new();
            for child in &block.then_branch {
                output.push_str(&self.render_node(child, item));
            }
            output
        } else if let Some(else_branch) = &block.else_branch {
            let mut output = String::new();
            for child in else_branch {
                output.push_str(&self.render_node(child, item));
            }
            output
        } else {
            String::new()
        }
    }

    fn apply_formatting(&self, text: &str, fmt: &crate::FormattingOptions) -> String {
        let prefix = fmt.prefix.as_deref().unwrap_or("");
        let suffix = fmt.suffix.as_deref().unwrap_or("");
        
        // Mock font styles with markdown or HTML? Let's use simple indicators for console
        let mut res = text.to_string();
        if fmt.font_style == Some(crate::FontStyle::Italic) {
            res = format!("_{}_", res);
        }
        if fmt.font_weight == Some(crate::FontWeight::Bold) {
            res = format!("*{}*", res);
        }

        format!("{}{}{}", prefix, res, suffix)
    }
}
