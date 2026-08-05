/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::{ItemType, Node, TemplateCompiler, TemplateComponent};

impl TemplateCompiler {
    pub(super) fn collect_types_with_branches(&self, nodes: &[Node]) -> Vec<ItemType> {
        let mut types = Vec::new();
        Self::collect_types_recursive(nodes, &mut types);
        types.sort_by_key(|t| self.item_type_to_string(t));
        types.dedup_by_key(|t| self.item_type_to_string(t));
        types
    }

    #[allow(dead_code, reason = "helper functions")]
    pub(super) fn collect_types_recursive(nodes: &[Node], types: &mut Vec<ItemType>) {
        for node in nodes {
            match node {
                Node::Group(g) => {
                    Self::collect_types_recursive(&g.children, types);
                }
                Node::Condition(c) => {
                    // Collect types from if branch
                    types.extend(c.if_item_type.clone());

                    // Collect types from else-if branches
                    for else_if in &c.else_if_branches {
                        types.extend(else_if.if_item_type.clone());
                    }

                    // Recurse into branches
                    Self::collect_types_recursive(&c.then_branch, types);
                    for else_if in &c.else_if_branches {
                        Self::collect_types_recursive(&else_if.children, types);
                    }
                    if let Some(ref else_nodes) = c.else_branch {
                        Self::collect_types_recursive(else_nodes, types);
                    }
                }
                _ => {}
            }
        }
    }

    /// Compile a complete template for a specific item type.
    ///
    /// When encountering type-based conditions, selects the matching branch
    /// for the given type, or falls back to else branch if no match.
    /// Currently unused - infrastructure for future `type_templates` generation.
    #[allow(dead_code, reason = "helper functions")]
    pub(super) fn compile_for_type(
        &self,
        nodes: &[Node],
        target_type: &ItemType,
    ) -> Vec<TemplateComponent> {
        self.compile_for_type_inner(nodes, target_type, false)
    }

    pub(super) fn compile_for_type_with_untyped_else_if_fallback(
        &self,
        nodes: &[Node],
        target_type: &ItemType,
    ) -> Vec<TemplateComponent> {
        self.compile_for_type_inner(nodes, target_type, true)
    }

    fn compile_for_type_inner(
        &self,
        nodes: &[Node],
        target_type: &ItemType,
        use_untyped_else_if_fallback: bool,
    ) -> Vec<TemplateComponent> {
        let mut components = Vec::new();

        for node in nodes {
            if let Some(component) = self.compile_node(node) {
                components.push(component);
            } else {
                match node {
                    Node::Group(g) => {
                        components.extend(self.compile_for_type_inner(
                            &g.children,
                            target_type,
                            use_untyped_else_if_fallback,
                        ));
                    }
                    Node::Condition(c) => {
                        // Check if this is a type-based condition
                        let has_type_condition = !c.if_item_type.is_empty()
                            || c.else_if_branches
                                .iter()
                                .any(|b| !b.if_item_type.is_empty());

                        if has_type_condition {
                            if c.if_item_type.contains(target_type) {
                                components.extend(self.compile_for_type_inner(
                                    &c.then_branch,
                                    target_type,
                                    use_untyped_else_if_fallback,
                                ));
                            } else {
                                let mut fallback_branch =
                                    if use_untyped_else_if_fallback && c.if_item_type.is_empty() {
                                        Some(&c.then_branch)
                                    } else {
                                        None
                                    };
                                let mut found = false;
                                for else_if in &c.else_if_branches {
                                    if else_if.if_item_type.contains(target_type) {
                                        components.extend(self.compile_for_type_inner(
                                            &else_if.children,
                                            target_type,
                                            use_untyped_else_if_fallback,
                                        ));
                                        found = true;
                                        break;
                                    }
                                    if use_untyped_else_if_fallback
                                        && else_if.if_item_type.is_empty()
                                        && fallback_branch.is_none()
                                    {
                                        fallback_branch = Some(&else_if.children);
                                    }
                                }
                                if !found {
                                    if let Some(fallback_branch) = fallback_branch {
                                        components.extend(self.compile_for_type_inner(
                                            fallback_branch,
                                            target_type,
                                            use_untyped_else_if_fallback,
                                        ));
                                    } else if let Some(ref else_nodes) = c.else_branch {
                                        components.extend(self.compile_for_type_inner(
                                            else_nodes,
                                            target_type,
                                            use_untyped_else_if_fallback,
                                        ));
                                    }
                                }
                            }
                        } else {
                            // Not a type condition, use default compile behavior
                            components.extend(self.compile_for_type_inner(
                                &c.then_branch,
                                target_type,
                                use_untyped_else_if_fallback,
                            ));
                            if let Some(ref else_nodes) = c.else_branch {
                                let else_components = self.compile_for_type_inner(
                                    else_nodes,
                                    target_type,
                                    use_untyped_else_if_fallback,
                                );
                                for ec in else_components {
                                    if !components.iter().any(|c| self.same_variable(c, &ec)) {
                                        components.push(ec);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        components
    }

    /// Convert `ItemType` to its string representation.
    #[allow(dead_code, reason = "helper functions")]
    pub(super) fn item_type_to_string(&self, item_type: &ItemType) -> String {
        match item_type {
            ItemType::Article => "article".to_string(),
            ItemType::ArticleJournal => "article-journal".to_string(),
            ItemType::ArticleMagazine => "article-magazine".to_string(),
            ItemType::ArticleNewspaper => "article-newspaper".to_string(),
            ItemType::Bill => "bill".to_string(),
            ItemType::Book => "book".to_string(),
            ItemType::Broadcast => "broadcast".to_string(),
            ItemType::Chapter => "chapter".to_string(),
            ItemType::Dataset => "dataset".to_string(),
            ItemType::Entry => "entry".to_string(),
            ItemType::EntryDictionary => "entry-dictionary".to_string(),
            ItemType::EntryEncyclopedia => "entry-encyclopedia".to_string(),
            ItemType::Figure => "figure".to_string(),
            ItemType::Graphic => "graphic".to_string(),
            ItemType::Interview => "interview".to_string(),
            ItemType::LegalCase => "legal-case".to_string(),
            ItemType::Legislation => "statute".to_string(),
            ItemType::Manuscript => "manuscript".to_string(),
            ItemType::Map => "map".to_string(),
            ItemType::MotionPicture => "motion-picture".to_string(),
            ItemType::MusicalScore => "musical-score".to_string(),
            ItemType::Pamphlet => "pamphlet".to_string(),
            ItemType::PaperConference => "paper-conference".to_string(),
            ItemType::Patent => "patent".to_string(),
            ItemType::PersonalCommunication => "personal-communication".to_string(),
            ItemType::Post => "post".to_string(),
            ItemType::PostWeblog => "post-weblog".to_string(),
            ItemType::Report => "report".to_string(),
            ItemType::Review => "review".to_string(),
            ItemType::ReviewBook => "review-book".to_string(),
            ItemType::Song => "song".to_string(),
            ItemType::Speech => "speech".to_string(),
            ItemType::Thesis => "thesis".to_string(),
            ItemType::Treaty => "treaty".to_string(),
            ItemType::Webpage => "webpage".to_string(),
            ItemType::Software => "software".to_string(),
            ItemType::Standard => "standard".to_string(),
        }
    }
}
