/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Compiles legacy CslnNode trees into CSLN TemplateComponents.
//!
//! This is the final step in migration: converting the upsampled node tree
//! into the clean, declarative TemplateComponent format.

use csln_core::{
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, DelimiterPunctuation,
        NumberVariable, Rendering, SimpleVariable, TemplateComponent, TemplateContributor,
        TemplateDate, TemplateList, TemplateNumber, TemplateTitle, TemplateVariable, TitleType,
    },
    CslnNode, FormattingOptions, ItemType, Variable,
};
use std::collections::HashMap;

/// Compiles CslnNode trees into TemplateComponents.
pub struct TemplateCompiler;

impl TemplateCompiler {
    /// Compile a list of CslnNodes into TemplateComponents.
    ///
    /// Recursively processes Groups and Conditions to extract all components.
    /// In the future, Condition logic should be handled by Options/overrides.
    /// Compile a list of CslnNodes into TemplateComponents.
    pub fn compile(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        // Use compile_with_wrap with no initial wrap and no current types
        let no_wrap = (None, None, None);
        self.compile_with_wrap(nodes, &no_wrap, &[])
    }
    /// Compile and sort for citation output (author first, then date).
    /// Uses simplified compile that skips else branches to avoid extra fields.
    pub fn compile_citation(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile_simple(nodes);
        self.sort_citation_components(&mut components);
        components
    }

    fn compile_with_wrap(
        &self,
        nodes: &[CslnNode],
        inherited_wrap: &(
            Option<csln_core::template::WrapPunctuation>,
            Option<String>,
            Option<String>,
        ),
        current_types: &[ItemType],
    ) -> Vec<TemplateComponent> {
        let mut components = Vec::new();

        for node in nodes {
            if let Some(mut component) = self.compile_node(node) {
                // Apply inherited wrap to date components
                if inherited_wrap.0.is_some() && matches!(&component, TemplateComponent::Date(_)) {
                    self.apply_wrap_to_component(&mut component, inherited_wrap);
                }
                // Add or replace with better-formatted version
                self.add_or_upgrade_component(&mut components, component, current_types);
            } else {
                match node {
                    CslnNode::Group(g) => {
                        // Check if this group has its own wrap
                        let group_wrap = Self::infer_wrap_from_affixes(
                            &g.formatting.prefix,
                            &g.formatting.suffix,
                        );
                        // Use group's wrap if it has one, otherwise inherit from parent
                        let effective_wrap = if group_wrap.0.is_some() {
                            group_wrap.clone()
                        } else {
                            inherited_wrap.clone()
                        };
                        let group_components =
                            self.compile_with_wrap(&g.children, &effective_wrap, current_types);

                        // Only create a List for meaningful structural groups:
                        // - Groups with explicit non-default delimiters (not period/comma)
                        // - AND containing 2-3 components that form a logical unit
                        // Most groups should just be flattened.
                        let meaningful_delimiter = g.delimiter.as_ref().is_some_and(|d| {
                            // Keep lists for special delimiters like none (volume+issue)
                            // or colon (title: subtitle)
                            matches!(d.as_str(), "" | "none" | ": " | " " | ", ")
                        });
                        let is_small_structural_group =
                            group_components.len() >= 2 && group_components.len() <= 3;
                        let should_be_list = meaningful_delimiter
                            && is_small_structural_group
                            && group_wrap.0.is_none();

                        if should_be_list && !group_components.is_empty() {
                            let list = TemplateComponent::List(TemplateList {
                                items: group_components,
                                delimiter: self.map_delimiter(&g.delimiter),
                                rendering: self.convert_formatting(&g.formatting),
                                ..Default::default()
                            });
                            self.add_or_upgrade_component(&mut components, list, current_types);
                        } else {
                            for gc in group_components {
                                self.add_or_upgrade_component(&mut components, gc, current_types);
                            }
                        }
                    }
                    CslnNode::Condition(c) => {
                        // Concatenate current types with if_item_type
                        let mut then_types = current_types.to_vec();
                        then_types.extend(c.if_item_type.clone());

                        // Pass wrap through conditions
                        let then_components =
                            self.compile_with_wrap(&c.then_branch, inherited_wrap, &then_types);
                        for tc in then_components {
                            self.add_or_upgrade_component(&mut components, tc, &then_types);
                        }

                        for else_if in &c.else_if_branches {
                            let mut else_if_types = current_types.to_vec();
                            else_if_types.extend(else_if.if_item_type.clone());

                            let branch_components = self.compile_with_wrap(
                                &else_if.children,
                                inherited_wrap,
                                &else_if_types,
                            );
                            for bc in branch_components {
                                self.add_or_upgrade_component(&mut components, bc, &else_if_types);
                            }
                        }

                        if let Some(ref else_nodes) = c.else_branch {
                            let else_components =
                                self.compile_with_wrap(else_nodes, inherited_wrap, current_types);
                            for ec in else_components {
                                self.add_or_upgrade_component(&mut components, ec, current_types);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        components
    }

    /// Add a component to the list, or upgrade an existing one if the new one has better formatting.
    fn add_or_upgrade_component(
        &self,
        components: &mut Vec<TemplateComponent>,
        new_component: TemplateComponent,
        current_types: &[ItemType],
    ) {
        // Check if we already have this component
        if let Some(idx) = components
            .iter()
            .position(|c| self.same_variable(c, &new_component))
        {
            if current_types.is_empty() {
                // For dates, upgrade if the new one has wrap and the old one doesn't
                if let (TemplateComponent::Date(existing), TemplateComponent::Date(new)) =
                    (&components[idx], &new_component)
                {
                    if existing.rendering.wrap.is_none() && new.rendering.wrap.is_some() {
                        // Upgrade: replace with the wrapped version
                        components[idx] = new_component;
                    }
                }
            } else {
                // Found a variable in a type-specific branch. Add to overrides.
                // Use type-specific override from new component if it exists,
                // otherwise fall back to base rendering.
                let base_rendering = self.get_component_rendering(&new_component);
                let new_overrides = self.get_component_overrides(&new_component);

                for item_type in current_types {
                    let type_str = self.item_type_to_string(item_type);
                    // Check if new component has a specific override for this type
                    let rendering = new_overrides
                        .as_ref()
                        .and_then(|ovr| ovr.get(&type_str))
                        .cloned()
                        .unwrap_or_else(|| base_rendering.clone());
                    self.add_override_to_component(&mut components[idx], type_str, rendering);
                }
            }
        } else {
            if let TemplateComponent::Title(t) = &new_component {
                if let Some(ref ovr) = t.overrides {
                    for (k, v) in ovr {
                        eprintln!("  {} -> emph={:?} quote={:?}", k, v.emph, v.quote);
                    }
                }
            }
            components.push(new_component);
        }
    }

    /// Simplified compile that only takes then_branch (for citations).
    /// This avoids pulling in type-specific variations from else branches.
    fn compile_simple(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        use csln_core::ItemType;
        let mut components = Vec::new();

        for node in nodes {
            if let Some(component) = self.compile_node(node) {
                components.push(component);
            } else {
                match node {
                    CslnNode::Group(g) => {
                        components.extend(self.compile_simple(&g.children));
                    }
                    CslnNode::Condition(c) => {
                        // For citations, prefer else_branch for uncommon type conditions
                        let uncommon_types = [
                            ItemType::PersonalCommunication,
                            ItemType::Interview,
                            ItemType::LegalCase,
                            ItemType::Legislation,
                            ItemType::Bill,
                            ItemType::Treaty,
                        ];
                        let is_uncommon_type = !c.if_item_type.is_empty()
                            && c.if_item_type.iter().any(|t| uncommon_types.contains(t));

                        if is_uncommon_type {
                            // Prefer else_branch for common/default case
                            // Check else_if_branches first for common types
                            let mut found = false;
                            for else_if in &c.else_if_branches {
                                let has_common_types = else_if.if_item_type.is_empty()
                                    || else_if
                                        .if_item_type
                                        .iter()
                                        .any(|t| !uncommon_types.contains(t));
                                if has_common_types {
                                    components.extend(self.compile_simple(&else_if.children));
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                if let Some(ref else_nodes) = c.else_branch {
                                    components.extend(self.compile_simple(else_nodes));
                                } else {
                                    components.extend(self.compile_simple(&c.then_branch));
                                }
                            }
                        } else {
                            // Take then_branch, but fall back to else_if/else_branch if empty
                            let then_components = self.compile_simple(&c.then_branch);
                            if !then_components.is_empty() {
                                components.extend(then_components);
                            } else {
                                // Try else_if branches first
                                let mut found = false;
                                for else_if in &c.else_if_branches {
                                    let branch_components = self.compile_simple(&else_if.children);
                                    if !branch_components.is_empty() {
                                        components.extend(branch_components);
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    if let Some(ref else_nodes) = c.else_branch {
                                        components.extend(self.compile_simple(else_nodes));
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

    /// Compile and sort for bibliography output.
    pub fn compile_bibliography(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile(nodes);
        self.sort_bibliography_components(&mut components);
        components
    }

    /// Compile bibliography with type-specific templates.
    ///
    /// Returns the default template and a HashMap of type-specific templates.
    /// Each type-specific template is COMPLETE - it includes all components,
    /// not just the type-specific parts.
    ///
    /// Currently DISABLED - returns empty type_templates because the extraction
    /// produces malformed templates with duplicates and missing components.
    /// The infrastructure is in place for future enhancement.
    pub fn compile_bibliography_with_types(
        &self,
        nodes: &[CslnNode],
    ) -> (
        Vec<TemplateComponent>,
        HashMap<String, Vec<TemplateComponent>>,
    ) {
        // Compile the default template
        let mut default_template = self.compile(nodes);

        // Deduplicate and flatten to remove redundant nesting from branch processing
        default_template = self.deduplicate_and_flatten(default_template);

        // DEBUG: Check titles after deduplicate
        for c in &default_template {
            if let TemplateComponent::Title(t) = c {
                if t.title == csln_core::template::TitleType::Primary {
                    if let Some(ref ovr) = t.overrides {
                        for (k, v) in ovr {
                            eprintln!("  {} -> emph={:?} quote={:?}", k, v.emph, v.quote);
                        }
                    } else {
                        eprintln!("  (no overrides)");
                    }
                }
            }
        }

        self.sort_bibliography_components(&mut default_template);

        // Type-specific template generation is disabled for now.
        // The compile_for_type approach produces malformed templates.
        // Future work: properly merge common components with type-specific branches.
        let type_templates: HashMap<String, Vec<TemplateComponent>> = HashMap::new();

        (default_template, type_templates)
    }

    /// Remove duplicate components and flatten unnecessary nesting.
    ///
    /// The compile_with_wrap function processes ALL branches of conditions,
    /// which can result in duplicate components and deeply nested Lists.
    /// This function cleans up the result by:
    /// 1. First pass: add all non-List components (primary variables)
    /// 2. Second pass: recursively clean Lists by removing items that are at top-level
    /// 3. Skip Lists that become empty or only have one item after cleaning
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

    /// Recursively clean a List by removing items that duplicate seen variables.
    /// Returns None if the list becomes empty, unwraps single-item lists.
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

    /// Extract all variable keys from a List (recursively).
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

    /// Get a unique key for a component for deduplication purposes.
    fn get_variable_key(&self, component: &TemplateComponent) -> Option<String> {
        match component {
            TemplateComponent::Contributor(c) => Some(format!("contributor:{:?}", c.contributor)),
            TemplateComponent::Date(d) => Some(format!("date:{:?}", d.date)),
            TemplateComponent::Title(t) => Some(format!("title:{:?}", t.title)),
            TemplateComponent::Number(n) => Some(format!("number:{:?}", n.number)),
            TemplateComponent::Variable(v) => Some(format!("variable:{:?}", v.variable)),
            // Lists don't have a single key - they contain multiple variables
            TemplateComponent::List(_) => None,
            _ => None,
        }
    }

    /// Collect all ItemTypes that have specific branches in conditions.
    /// Currently unused - infrastructure for future type_templates generation.
    #[allow(dead_code)]
    fn collect_types_with_branches(&self, nodes: &[CslnNode]) -> Vec<ItemType> {
        let mut types = Vec::new();
        self.collect_types_recursive(nodes, &mut types);
        types.sort_by_key(|t| self.item_type_to_string(t));
        types.dedup_by_key(|t| self.item_type_to_string(t));
        types
    }

    #[allow(dead_code)]
    #[allow(clippy::only_used_in_recursion)]
    fn collect_types_recursive(&self, nodes: &[CslnNode], types: &mut Vec<ItemType>) {
        for node in nodes {
            match node {
                CslnNode::Group(g) => {
                    self.collect_types_recursive(&g.children, types);
                }
                CslnNode::Condition(c) => {
                    // Collect types from if branch
                    types.extend(c.if_item_type.clone());

                    // Collect types from else-if branches
                    for else_if in &c.else_if_branches {
                        types.extend(else_if.if_item_type.clone());
                    }

                    // Recurse into branches
                    self.collect_types_recursive(&c.then_branch, types);
                    for else_if in &c.else_if_branches {
                        self.collect_types_recursive(&else_if.children, types);
                    }
                    if let Some(ref else_nodes) = c.else_branch {
                        self.collect_types_recursive(else_nodes, types);
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
    /// Currently unused - infrastructure for future type_templates generation.
    #[allow(dead_code)]
    fn compile_for_type(
        &self,
        nodes: &[CslnNode],
        target_type: &ItemType,
    ) -> Vec<TemplateComponent> {
        let mut components = Vec::new();

        for node in nodes {
            if let Some(component) = self.compile_node(node) {
                components.push(component);
            } else {
                match node {
                    CslnNode::Group(g) => {
                        components.extend(self.compile_for_type(&g.children, target_type));
                    }
                    CslnNode::Condition(c) => {
                        // Check if this is a type-based condition
                        let has_type_condition = !c.if_item_type.is_empty()
                            || c.else_if_branches
                                .iter()
                                .any(|b| !b.if_item_type.is_empty());

                        if has_type_condition {
                            // Select the matching branch for target_type
                            if c.if_item_type.contains(target_type) {
                                components
                                    .extend(self.compile_for_type(&c.then_branch, target_type));
                            } else {
                                // Check else-if branches
                                let mut found = false;
                                for else_if in &c.else_if_branches {
                                    if else_if.if_item_type.contains(target_type) {
                                        components.extend(
                                            self.compile_for_type(&else_if.children, target_type),
                                        );
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    // Fall back to else branch
                                    if let Some(ref else_nodes) = c.else_branch {
                                        components
                                            .extend(self.compile_for_type(else_nodes, target_type));
                                    }
                                }
                            }
                        } else {
                            // Not a type condition, use default compile behavior
                            components.extend(self.compile_for_type(&c.then_branch, target_type));
                            if let Some(ref else_nodes) = c.else_branch {
                                let else_components =
                                    self.compile_for_type(else_nodes, target_type);
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

    /// Convert ItemType to its string representation.
    #[allow(dead_code)]
    fn item_type_to_string(&self, item_type: &ItemType) -> String {
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
            ItemType::LegalCase => "legal_case".to_string(),
            ItemType::Legislation => "legislation".to_string(),
            ItemType::Manuscript => "manuscript".to_string(),
            ItemType::Map => "map".to_string(),
            ItemType::MotionPicture => "motion_picture".to_string(),
            ItemType::MusicalScore => "musical_score".to_string(),
            ItemType::Pamphlet => "pamphlet".to_string(),
            ItemType::PaperConference => "paper-conference".to_string(),
            ItemType::Patent => "patent".to_string(),
            ItemType::PersonalCommunication => "personal_communication".to_string(),
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

    /// Sort components for citation: author/date first.
    fn sort_citation_components(&self, components: &mut [TemplateComponent]) {
        components.sort_by_key(|c| match c {
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 0,
            TemplateComponent::Contributor(_) => 1,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => 2,
            TemplateComponent::Date(_) => 3,
            TemplateComponent::Title(_) => 4,
            _ => 5,
        });
    }

    /// Sort components for bibliography: citation-number first (for numeric styles),
    /// then author, date, title, then rest.
    fn sort_bibliography_components(&self, components: &mut [TemplateComponent]) {
        components.sort_by_key(|c| match c {
            // Citation number goes first for numeric bibliography styles
            TemplateComponent::Number(n) if n.number == NumberVariable::CitationNumber => 0,
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 1,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => 2,
            TemplateComponent::Title(t) if t.title == TitleType::Primary => 3,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial => 4,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph => 5,
            TemplateComponent::Number(_) => 6,
            TemplateComponent::Variable(_) => 7,
            TemplateComponent::Contributor(_) => 8,
            TemplateComponent::Date(_) => 9,
            TemplateComponent::Title(_) => 10,
            TemplateComponent::List(_) => 11,
            _ => 99,
        });
    }

    /// Check if two components refer to the same variable.
    fn same_variable(&self, a: &TemplateComponent, b: &TemplateComponent) -> bool {
        match (a, b) {
            (TemplateComponent::Contributor(c1), TemplateComponent::Contributor(c2)) => {
                c1.contributor == c2.contributor
            }
            (TemplateComponent::Date(d1), TemplateComponent::Date(d2)) => d1.date == d2.date,
            (TemplateComponent::Title(t1), TemplateComponent::Title(t2)) => t1.title == t2.title,
            (TemplateComponent::Number(n1), TemplateComponent::Number(n2)) => {
                n1.number == n2.number
            }
            (TemplateComponent::Variable(v1), TemplateComponent::Variable(v2)) => {
                v1.variable == v2.variable
            }
            _ => false,
        }
    }

    /// Try to compile a single node into a TemplateComponent.
    fn compile_node(&self, node: &CslnNode) -> Option<TemplateComponent> {
        match node {
            CslnNode::Names(names) => self.compile_names(names),
            CslnNode::Date(date) => self.compile_date(date),
            CslnNode::Variable(var) => self.compile_variable(var),
            _ => None,
        }
    }

    /// Compile a Names block into a Contributor component.
    fn compile_names(&self, names: &csln_core::NamesBlock) -> Option<TemplateComponent> {
        let role = self.map_variable_to_role(&names.variable)?;

        let form = match names.options.mode {
            Some(csln_core::NameMode::Short) => ContributorForm::Short,
            Some(csln_core::NameMode::Count) => ContributorForm::Short, // Map count to short
            _ => ContributorForm::Long,
        };

        Some(TemplateComponent::Contributor(TemplateContributor {
            contributor: role,
            form,
            name_order: None, // Use global setting by default
            delimiter: names.options.delimiter.clone(),
            rendering: self.convert_formatting(&names.formatting),
            ..Default::default()
        }))
    }

    /// Map a Variable to ContributorRole.
    fn map_variable_to_role(&self, var: &Variable) -> Option<ContributorRole> {
        match var {
            Variable::Author => Some(ContributorRole::Author),
            Variable::Editor => Some(ContributorRole::Editor),
            Variable::Translator => Some(ContributorRole::Translator),
            Variable::Director => Some(ContributorRole::Director),
            Variable::Composer => Some(ContributorRole::Composer),
            Variable::Illustrator => Some(ContributorRole::Illustrator),
            Variable::Interviewer => Some(ContributorRole::Interviewer),
            Variable::Recipient => Some(ContributorRole::Recipient),
            Variable::CollectionEditor => Some(ContributorRole::CollectionEditor),
            Variable::ContainerAuthor => Some(ContributorRole::ContainerAuthor),
            Variable::EditorialDirector => Some(ContributorRole::EditorialDirector),
            Variable::OriginalAuthor => Some(ContributorRole::OriginalAuthor),
            Variable::ReviewedAuthor => Some(ContributorRole::ReviewedAuthor),
            _ => None,
        }
    }

    /// Compile a Date block into a Date component.
    fn compile_date(&self, date: &csln_core::DateBlock) -> Option<TemplateComponent> {
        let date_var = self.map_variable_to_date(&date.variable)?;

        let form = match &date.options.parts {
            Some(csln_core::DateParts::Year) => DateForm::Year,
            Some(csln_core::DateParts::YearMonth) => DateForm::YearMonth,
            _ => match &date.options.form {
                Some(csln_core::DateForm::Numeric) => DateForm::Full,
                Some(csln_core::DateForm::Text) => DateForm::Full,
                None => DateForm::Year,
            },
        };

        Some(TemplateComponent::Date(TemplateDate {
            date: date_var,
            form,
            rendering: self.convert_formatting(&date.formatting),
            ..Default::default()
        }))
    }

    /// Map a Variable to DateVariable.
    fn map_variable_to_date(&self, var: &Variable) -> Option<DateVariable> {
        match var {
            Variable::Issued => Some(DateVariable::Issued),
            Variable::Accessed => Some(DateVariable::Accessed),
            Variable::OriginalDate => Some(DateVariable::OriginalPublished),
            Variable::Submitted => Some(DateVariable::Submitted),
            Variable::EventDate => Some(DateVariable::EventDate),
            _ => None,
        }
    }

    /// Compile a Variable block into the appropriate component.
    fn compile_variable(&self, var: &csln_core::VariableBlock) -> Option<TemplateComponent> {
        // First, check if it's a contributor role
        if let Some(role) = self.map_variable_to_role(&var.variable) {
            return Some(TemplateComponent::Contributor(TemplateContributor {
                contributor: role,
                form: ContributorForm::Long,
                name_order: None, // Use global setting by default
                delimiter: None,
                rendering: self.convert_formatting(&var.formatting),
                ..Default::default()
            }));
        }

        // Check if it's a title
        if let Some(title_type) = self.map_variable_to_title(&var.variable) {
            // Convert overrides from FormattingOptions to Rendering
            let overrides = if var.overrides.is_empty() {
                None
            } else {
                for (t, fmt) in &var.overrides {
                    eprintln!("  {:?} -> {:?}", t, fmt);
                }
                Some(
                    var.overrides
                        .iter()
                        .map(|(t, fmt)| (self.item_type_to_string(t), self.convert_formatting(fmt)))
                        .collect(),
                )
            };
            return Some(TemplateComponent::Title(TemplateTitle {
                title: title_type,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides,
                ..Default::default()
            }));
        }

        // Check if it's a number
        if let Some(num_var) = self.map_variable_to_number(&var.variable) {
            // Convert overrides from FormattingOptions to Rendering
            let overrides = if var.overrides.is_empty() {
                None
            } else {
                Some(
                    var.overrides
                        .iter()
                        .map(|(t, fmt)| (self.item_type_to_string(t), self.convert_formatting(fmt)))
                        .collect(),
                )
            };
            return Some(TemplateComponent::Number(TemplateNumber {
                number: num_var,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides,
                ..Default::default()
            }));
        }

        // Check if it's a simple variable
        if let Some(simple_var) = self.map_variable_to_simple(&var.variable) {
            // Convert overrides from FormattingOptions to Rendering
            let overrides = if var.overrides.is_empty() {
                None
            } else {
                Some(
                    var.overrides
                        .iter()
                        .map(|(t, fmt)| (self.item_type_to_string(t), self.convert_formatting(fmt)))
                        .collect(),
                )
            };
            return Some(TemplateComponent::Variable(TemplateVariable {
                variable: simple_var,
                rendering: self.convert_formatting(&var.formatting),
                overrides,
                ..Default::default()
            }));
        }

        None
    }

    /// Map a Variable to TitleType.
    fn map_variable_to_title(&self, var: &Variable) -> Option<TitleType> {
        match var {
            Variable::Title => Some(TitleType::Primary),
            Variable::ContainerTitle => Some(TitleType::ParentSerial),
            Variable::CollectionTitle => Some(TitleType::ParentMonograph),
            _ => None,
        }
    }

    /// Map a Variable to NumberVariable.
    fn map_variable_to_number(&self, var: &Variable) -> Option<NumberVariable> {
        match var {
            Variable::Volume => Some(NumberVariable::Volume),
            Variable::Issue => Some(NumberVariable::Issue),
            Variable::Page => Some(NumberVariable::Pages),
            Variable::Edition => Some(NumberVariable::Edition),
            Variable::ChapterNumber => Some(NumberVariable::ChapterNumber),
            Variable::NumberOfVolumes => Some(NumberVariable::NumberOfVolumes),
            Variable::CitationNumber => Some(NumberVariable::CitationNumber),
            _ => None,
        }
    }

    /// Map a Variable to SimpleVariable.
    fn map_variable_to_simple(&self, var: &Variable) -> Option<SimpleVariable> {
        match var {
            Variable::DOI => Some(SimpleVariable::Doi),
            Variable::ISBN => Some(SimpleVariable::Isbn),
            Variable::ISSN => Some(SimpleVariable::Issn),
            Variable::URL => Some(SimpleVariable::Url),
            Variable::Publisher => Some(SimpleVariable::Publisher),
            Variable::PublisherPlace => Some(SimpleVariable::PublisherPlace),
            Variable::Genre => Some(SimpleVariable::Genre),
            _ => None,
        }
    }

    /// Convert FormattingOptions to Rendering.
    fn convert_formatting(&self, fmt: &FormattingOptions) -> Rendering {
        // Infer wrap from prefix/suffix patterns
        let (mut wrap, prefix, suffix) = Self::infer_wrap_from_affixes(&fmt.prefix, &fmt.suffix);

        // quotes="true" in CSL maps to wrap: quotes in CSLN
        if fmt.quotes == Some(true) {
            wrap = Some(csln_core::template::WrapPunctuation::Quotes);
        }

        Rendering {
            emph: fmt
                .font_style
                .as_ref()
                .map(|s| matches!(s, csln_core::FontStyle::Italic)),
            strong: fmt
                .font_weight
                .as_ref()
                .map(|w| matches!(w, csln_core::FontWeight::Bold)),
            small_caps: fmt
                .font_variant
                .as_ref()
                .map(|v| matches!(v, csln_core::FontVariant::SmallCaps)),
            quote: fmt.quotes,
            prefix,
            suffix,
            wrap,
            suppress: None,
        }
    }

    /// Infer wrap type from prefix/suffix patterns.
    ///
    /// CSL 1.0 uses `prefix="("` and `suffix=")"` for parentheses wrapping.
    /// CSLN prefers explicit `wrap: parentheses` for cleaner representation.
    ///
    /// Returns (wrap, remaining_prefix, remaining_suffix) where the wrap chars
    /// have been extracted and remaining affixes are returned.
    fn infer_wrap_from_affixes(
        prefix: &Option<String>,
        suffix: &Option<String>,
    ) -> (
        Option<csln_core::template::WrapPunctuation>,
        Option<String>,
        Option<String>,
    ) {
        use csln_core::template::WrapPunctuation;

        match (prefix.as_deref(), suffix.as_deref()) {
            // Clean parentheses: prefix ends with "(", suffix starts with ")"
            (Some(p), Some(s)) if p.ends_with('(') && s.starts_with(')') => {
                let remaining_prefix = p
                    .strip_suffix('(')
                    .map(|r| r.to_string())
                    .filter(|s| !s.is_empty());
                let remaining_suffix = s
                    .strip_prefix(')')
                    .map(|r| r.to_string())
                    .filter(|s| !s.is_empty());
                (
                    Some(WrapPunctuation::Parentheses),
                    remaining_prefix,
                    remaining_suffix,
                )
            }
            // Clean brackets
            (Some(p), Some(s)) if p.ends_with('[') && s.starts_with(']') => {
                let remaining_prefix = p
                    .strip_suffix('[')
                    .map(|r| r.to_string())
                    .filter(|s| !s.is_empty());
                let remaining_suffix = s
                    .strip_prefix(']')
                    .map(|r| r.to_string())
                    .filter(|s| !s.is_empty());
                (
                    Some(WrapPunctuation::Brackets),
                    remaining_prefix,
                    remaining_suffix,
                )
            }
            // No wrap pattern found - keep original affixes
            _ => (None, prefix.clone(), suffix.clone()),
        }
    }

    /// Apply wrap formatting from a parent group to a component.
    ///
    /// When a group with `prefix="(" suffix=")"` wraps a date, the date
    /// should inherit the wrap property since groups are flattened.
    fn apply_wrap_to_component(
        &self,
        component: &mut TemplateComponent,
        group_wrap: &(
            Option<csln_core::template::WrapPunctuation>,
            Option<String>,
            Option<String>,
        ),
    ) {
        let (wrap, prefix, suffix) = group_wrap;

        // Only apply wrap if the component doesn't already have one
        match component {
            TemplateComponent::Date(d) => {
                if d.rendering.wrap.is_none() && wrap.is_some() {
                    d.rendering.wrap = wrap.clone();
                }
                // Also apply remaining prefix/suffix if not already set
                if d.rendering.prefix.is_none() && prefix.is_some() {
                    d.rendering.prefix = prefix.clone();
                }
                if d.rendering.suffix.is_none() && suffix.is_some() {
                    d.rendering.suffix = suffix.clone();
                }
            }
            TemplateComponent::Contributor(c) => {
                if c.rendering.wrap.is_none() && wrap.is_some() {
                    c.rendering.wrap = wrap.clone();
                }
                if c.rendering.prefix.is_none() && prefix.is_some() {
                    c.rendering.prefix = prefix.clone();
                }
                if c.rendering.suffix.is_none() && suffix.is_some() {
                    c.rendering.suffix = suffix.clone();
                }
            }
            TemplateComponent::Title(t) => {
                if t.rendering.wrap.is_none() && wrap.is_some() {
                    t.rendering.wrap = wrap.clone();
                }
                if t.rendering.prefix.is_none() && prefix.is_some() {
                    t.rendering.prefix = prefix.clone();
                }
                if t.rendering.suffix.is_none() && suffix.is_some() {
                    t.rendering.suffix = suffix.clone();
                }
            }
            TemplateComponent::Number(n) => {
                if n.rendering.wrap.is_none() && wrap.is_some() {
                    n.rendering.wrap = wrap.clone();
                }
                if n.rendering.prefix.is_none() && prefix.is_some() {
                    n.rendering.prefix = prefix.clone();
                }
                if n.rendering.suffix.is_none() && suffix.is_some() {
                    n.rendering.suffix = suffix.clone();
                }
            }
            TemplateComponent::Variable(v) => {
                if v.rendering.wrap.is_none() && wrap.is_some() {
                    v.rendering.wrap = wrap.clone();
                }
                if v.rendering.prefix.is_none() && prefix.is_some() {
                    v.rendering.prefix = prefix.clone();
                }
                if v.rendering.suffix.is_none() && suffix.is_some() {
                    v.rendering.suffix = suffix.clone();
                }
            }
            _ => {} // List and future variants - don't modify
        }
    }
    /// Map a String delimiter to DelimiterPunctuation.
    fn map_delimiter(&self, delimiter: &Option<String>) -> Option<DelimiterPunctuation> {
        let d = delimiter.as_ref()?;
        match d.as_str() {
            ", " | "," => Some(DelimiterPunctuation::Comma),
            "; " | ";" => Some(DelimiterPunctuation::Semicolon),
            ". " | "." => Some(DelimiterPunctuation::Period),
            ": " | ":" => Some(DelimiterPunctuation::Colon),
            " & " | "&" => Some(DelimiterPunctuation::Ampersand),
            " | " | "|" => Some(DelimiterPunctuation::VerticalLine),
            " / " | "/" => Some(DelimiterPunctuation::Slash),
            " - " | "-" => Some(DelimiterPunctuation::Hyphen),
            " " => Some(DelimiterPunctuation::Space),
            "" => Some(DelimiterPunctuation::None),
            _ => None,
        }
    }

    /// Get the rendering options from a component.
    fn get_component_rendering(&self, component: &TemplateComponent) -> Rendering {
        match component {
            TemplateComponent::Contributor(c) => c.rendering.clone(),
            TemplateComponent::Date(d) => d.rendering.clone(),
            TemplateComponent::Number(n) => n.rendering.clone(),
            TemplateComponent::Title(t) => t.rendering.clone(),
            TemplateComponent::Variable(v) => v.rendering.clone(),
            TemplateComponent::List(l) => l.rendering.clone(),
            _ => Rendering::default(),
        }
    }

    /// Get type-specific overrides from a component.
    fn get_component_overrides(
        &self,
        component: &TemplateComponent,
    ) -> Option<HashMap<String, Rendering>> {
        match component {
            TemplateComponent::Contributor(c) => c.overrides.clone(),
            TemplateComponent::Date(d) => d.overrides.clone(),
            TemplateComponent::Number(n) => n.overrides.clone(),
            TemplateComponent::Title(t) => t.overrides.clone(),
            TemplateComponent::Variable(v) => v.overrides.clone(),
            _ => None,
        }
    }

    /// Add a type-specific override to a component.
    fn add_override_to_component(
        &self,
        component: &mut TemplateComponent,
        type_str: String,
        rendering: Rendering,
    ) {
        // Skip if override is basically empty/default
        if rendering == Rendering::default() {
            return;
        }

        match component {
            TemplateComponent::Contributor(c) => {
                c.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            TemplateComponent::Date(d) => {
                d.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            TemplateComponent::Number(n) => {
                n.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            TemplateComponent::Title(t) => {
                t.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            TemplateComponent::Variable(v) => {
                v.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            TemplateComponent::List(l) => {
                l.overrides
                    .get_or_insert_with(HashMap::new)
                    .insert(type_str, rendering);
            }
            _ => {} // Future variants
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::{DateBlock, DateOptions, NamesBlock, NamesOptions, VariableBlock};
    use std::collections::HashMap;

    #[test]
    fn test_compile_names_to_contributor() {
        let compiler = TemplateCompiler;
        let names = CslnNode::Names(NamesBlock {
            variable: Variable::Author,
            options: NamesOptions::default(),
            formatting: FormattingOptions::default(),
        });

        let result = compiler.compile(&[names]);
        assert_eq!(result.len(), 1);

        if let TemplateComponent::Contributor(c) = &result[0] {
            assert_eq!(c.contributor, ContributorRole::Author);
            assert_eq!(c.form, ContributorForm::Long);
        } else {
            panic!("Expected Contributor component");
        }
    }

    #[test]
    fn test_compile_date() {
        let compiler = TemplateCompiler;
        let date = CslnNode::Date(DateBlock {
            variable: Variable::Issued,
            options: DateOptions {
                parts: Some(csln_core::DateParts::Year),
                ..Default::default()
            },
            formatting: FormattingOptions::default(),
        });

        let result = compiler.compile(&[date]);
        assert_eq!(result.len(), 1);

        if let TemplateComponent::Date(d) = &result[0] {
            assert_eq!(d.date, DateVariable::Issued);
            assert_eq!(d.form, DateForm::Year);
        } else {
            panic!("Expected Date component");
        }
    }

    #[test]
    fn test_compile_variable_to_title() {
        let compiler = TemplateCompiler;
        let var = CslnNode::Variable(VariableBlock {
            variable: Variable::Title,
            label: None,
            formatting: FormattingOptions {
                font_style: Some(csln_core::FontStyle::Italic),
                ..Default::default()
            },
            overrides: HashMap::new(),
        });

        let result = compiler.compile(&[var]);
        assert_eq!(result.len(), 1);

        if let TemplateComponent::Title(t) = &result[0] {
            assert_eq!(t.title, TitleType::Primary);
            assert_eq!(t.rendering.emph, Some(true));
        } else {
            panic!("Expected Title component");
        }
    }

    #[test]
    fn test_compile_variable_to_doi() {
        let compiler = TemplateCompiler;
        let var = CslnNode::Variable(VariableBlock {
            variable: Variable::DOI,
            label: None,
            formatting: FormattingOptions::default(),
            overrides: HashMap::new(),
        });

        let result = compiler.compile(&[var]);
        assert_eq!(result.len(), 1);

        if let TemplateComponent::Variable(v) = &result[0] {
            assert_eq!(v.variable, SimpleVariable::Doi);
        } else {
            panic!("Expected Variable component");
        }
    }
}
