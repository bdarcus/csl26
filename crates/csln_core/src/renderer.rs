use crate::{
    ConditionBlock, CslnNode, DateBlock, GroupBlock, ItemType, NamesBlock, Variable, VariableBlock,
};
use std::collections::HashMap;

/// A mock citation item with metadata.
pub struct CitationItem {
    pub item_type: ItemType,
    pub variables: HashMap<Variable, String>,
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

            if let Some(label_opts) = &block.label {
                let prefix = label_opts.formatting.prefix.as_deref().unwrap_or("");
                let suffix = label_opts.formatting.suffix.as_deref().unwrap_or("");
                let label_text = match block.variable {
                    Variable::Page => "p.",
                    Variable::Volume => "vol.",
                    _ => "",
                };
                text = format!("{}{}{}{}", prefix, label_text, suffix, text);
            }

            self.apply_formatting(&text, &block.formatting)
        } else {
            String::new()
        }
    }

    fn render_date(&self, block: &DateBlock, item: &CitationItem) -> String {
        if let Some(val) = item.variables.get(&block.variable) {
            self.apply_formatting(val, &block.formatting)
        } else {
            String::new()
        }
    }

    fn render_names(&self, block: &NamesBlock, item: &CitationItem) -> String {
        let active_val = if let Some(val) = item.variables.get(&block.variable) {
            Some(val.clone())
        } else {
            block
                .options
                .substitute
                .iter()
                .find_map(|sub_var| item.variables.get(sub_var).cloned())
        };

        if let Some(mut formatted) = active_val {
            if let Some(init) = &block.options.initialize_with {
                if !formatted.contains(init) {
                    formatted = format!("{} [Init: {}]", formatted, init);
                }
            }

            if let Some(order) = &block.options.name_as_sort_order {
                formatted = format!("{} [Sort: {:?}]", formatted, order);
            }

            self.apply_formatting(&formatted, &block.formatting)
        } else {
            String::new()
        }
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
        let type_match =
            block.if_item_type.is_empty() || block.if_item_type.contains(&item.item_type);
        let var_match = block.if_variables.is_empty()
            || block
                .if_variables
                .iter()
                .any(|v| item.variables.contains_key(v));

        let match_found = if block.if_item_type.is_empty() && block.if_variables.is_empty() {
            false
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

        let mut res = text.to_string();
        if fmt.font_style == Some(crate::FontStyle::Italic) {
            res = format!("_{}_", res);
        }
        if fmt.font_weight == Some(crate::FontWeight::Bold) {
            res = format!("*{}*", res);
        }
        if fmt.text_decoration == Some(crate::TextDecoration::Underline) {
            res = format!("<u>{}</u>", res);
        }
        if fmt.vertical_align == Some(crate::VerticalAlign::Superscript) {
            res = format!("^{}^", res);
        }

        format!("{}{}{}", prefix, res, suffix)
    }
}
