use csln_core::{CslnNode, ItemType};

pub struct Compressor;

impl Compressor {
    pub fn compress_nodes(&self, nodes: Vec<CslnNode>) -> Vec<CslnNode> {
        let mut compressed = Vec::new();
        for node in nodes {
            match node {
                CslnNode::Condition(cond) => {
                    // Recurse first
                    let then_compressed = self.compress_nodes(cond.then_branch);
                    let else_if_compressed: Vec<csln_core::ElseIfBranch> = cond
                        .else_if_branches
                        .into_iter()
                        .map(|branch| csln_core::ElseIfBranch {
                            if_item_type: branch.if_item_type,
                            if_variables: branch.if_variables,
                            children: self.compress_nodes(branch.children),
                        })
                        .collect();
                    let else_compressed = cond.else_branch.map(|e| self.compress_nodes(e));

                    // Attempt Merge (only when no else-if branches)
                    if else_if_compressed.is_empty() {
                        if let Some(merged) = self.try_merge_branches(
                            &cond.if_item_type,
                            &then_compressed,
                            &else_compressed,
                        ) {
                            compressed.push(merged);
                            continue;
                        }
                    }

                    // Keep Condition if merge fails, but use compressed children
                    let new_cond = csln_core::ConditionBlock {
                        if_item_type: cond.if_item_type.clone(),
                        if_variables: cond.if_variables.clone(),
                        then_branch: then_compressed,
                        else_if_branches: else_if_compressed,
                        else_branch: else_compressed,
                    };
                    compressed.push(CslnNode::Condition(new_cond));
                }
                CslnNode::Group(mut group) => {
                    group.children = self.compress_nodes(group.children);
                    compressed.push(CslnNode::Group(group));
                }
                _ => compressed.push(node),
            }
        }
        compressed
    }

    fn try_merge_branches(
        &self,
        if_types: &[ItemType],
        then_nodes: &[CslnNode],
        else_nodes: &Option<Vec<CslnNode>>,
    ) -> Option<CslnNode> {
        // Simple Case: 1 node in THEN, 1 node in ELSE (or empty ELSE)
        // And they are the SAME VARIABLE.

        if then_nodes.len() != 1 {
            return None;
        }

        // Handle "Then vs Else"
        if let Some(else_nodes) = else_nodes {
            if else_nodes.len() != 1 {
                return None;
            }

            let then_node = &then_nodes[0];
            let else_node = &else_nodes[0];

            if let (CslnNode::Variable(v1), CslnNode::Variable(v2)) = (then_node, else_node) {
                if v1.variable == v2.variable {
                    // MERGE!
                    let mut merged = v2.clone(); // Start with base (else) defaults

                    // For each type in the IF branch, create an override that:
                    // 1. Applies the IF formatting
                    // 2. Explicitly negates any base formatting that's not in the override
                    for t in if_types {
                        let mut override_fmt = v1.formatting.clone();

                        // If base has quotes=true but override has quotes=None,
                        // explicitly set quotes=false
                        if v2.formatting.quotes == Some(true) && override_fmt.quotes.is_none() {
                            override_fmt.quotes = Some(false);
                        }

                        // If base has font_style but override doesn't, clear it
                        if v2.formatting.font_style.is_some() && override_fmt.font_style.is_none() {
                            override_fmt.font_style = Some(csln_core::FontStyle::Normal);
                        }

                        merged.overrides.insert(t.clone(), override_fmt);
                    }
                    return Some(CslnNode::Variable(merged));
                }
            }
        } else {
            // Handle "Then vs Nothing" (e.g. only print Volume if Book)
            // This maps to "overrides" logic ONLY if we assume the default is "Hidden".
            // But CSLN VariableBlock implies "Show".
            // So we can't easily merge "Show vs Hide" into "Show with options" unless we have a "Hidden" formatting state.
            // For now, ignore.
        }

        None
    }
}
