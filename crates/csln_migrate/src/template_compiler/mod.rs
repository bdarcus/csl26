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
use indexmap::IndexMap;
use std::collections::HashMap;

/// Context for a conditional branch, distinguishing between type-specific
/// and default branches. This is critical for correct suppress semantics.
#[derive(Debug, Clone)]
enum BranchContext {
    /// Type-specific branch (THEN/ELSE_IF with type conditions).
    /// Components here should be shown ONLY for these types.
    TypeSpecific(Vec<ItemType>),
    /// Default branch (ELSE or no condition).
    /// Components here should be shown for ALL types except overridden.
    Default,
}

/// Records a component's occurrence in a specific branch context.
#[derive(Debug, Clone)]
struct ComponentOccurrence {
    component: TemplateComponent,
    context: BranchContext,
    source_order: Option<usize>,
}

/// Compiles CslnNode trees into TemplateComponents.
pub struct TemplateCompiler;

impl TemplateCompiler {
    /// Compile a list of CslnNodes into TemplateComponents.
    ///
    /// Uses occurrence-based compilation to properly handle mutually exclusive
    /// conditional branches. Components are collected with their branch context,
    /// then merged with correct suppress semantics.
    pub fn compile(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let no_wrap = (None, None, None);
        let mut occurrences = Vec::new();
        self.collect_occurrences(nodes, &no_wrap, &BranchContext::Default, &mut occurrences);
        self.merge_occurrences(occurrences)
    }
    /// Compile and sort for citation output (author first, then date).
    /// Uses simplified compile that skips else branches to avoid extra fields.
    pub fn compile_citation(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile_simple(nodes);
        self.sort_citation_components(&mut components);
        components
    }

    /// Collect component occurrences with their branch context.
    /// This replaces the old compile_with_wrap approach.
    fn collect_occurrences(
        &self,
        nodes: &[CslnNode],
        inherited_wrap: &(
            Option<csln_core::template::WrapPunctuation>,
            Option<String>,
            Option<String>,
        ),
        context: &BranchContext,
        occurrences: &mut Vec<ComponentOccurrence>,
    ) {
        let mut i = 0;

        while i < nodes.len() {
            let node = &nodes[i];

            // Lookahead merge for Text nodes
            if let CslnNode::Text { value } = node {
                if i + 1 < nodes.len() {
                    if let Some(mut next_comp) = self.compile_node(&nodes[i + 1]) {
                        // Merge text into prefix
                        let mut rendering = self.get_component_rendering(&next_comp);
                        let mut new_prefix = value.clone();
                        if let Some(p) = rendering.prefix {
                            new_prefix.push_str(&p);
                        }
                        rendering.prefix = Some(new_prefix);
                        self.set_component_rendering(&mut next_comp, rendering);

                        // Apply inherited wrap if applicable
                        if inherited_wrap.0.is_some()
                            && matches!(&next_comp, TemplateComponent::Date(_))
                        {
                            self.apply_wrap_to_component(&mut next_comp, inherited_wrap);
                        }

                        // Extract source_order from the next node
                        let source_order = self.extract_source_order(&nodes[i + 1]);
                        occurrences.push(ComponentOccurrence {
                            component: next_comp,
                            context: context.clone(),
                            source_order,
                        });
                        i += 2;
                        continue;
                    }
                }
            }

            if let Some(mut component) = self.compile_node(node) {
                // Apply inherited wrap to date components
                if inherited_wrap.0.is_some() && matches!(&component, TemplateComponent::Date(_)) {
                    self.apply_wrap_to_component(&mut component, inherited_wrap);
                }
                let source_order = self.extract_source_order(node);
                occurrences.push(ComponentOccurrence {
                    component,
                    context: context.clone(),
                    source_order,
                });
            } else {
                match node {
                    CslnNode::Group(g) => {
                        // Check if this group has its own wrap
                        let group_wrap = Self::infer_wrap_from_affixes(
                            &g.formatting.prefix,
                            &g.formatting.suffix,
                        );
                        let effective_wrap = if group_wrap.0.is_some() {
                            group_wrap.clone()
                        } else {
                            inherited_wrap.clone()
                        };

                        // Collect group components into a temporary list
                        let mut group_occurrences = Vec::new();
                        self.collect_occurrences(
                            &g.children,
                            &effective_wrap,
                            context,
                            &mut group_occurrences,
                        );

                        // Extract components from occurrences for grouping logic
                        let group_components: Vec<TemplateComponent> = group_occurrences
                            .iter()
                            .map(|o| o.component.clone())
                            .collect();

                        // Decide if this should be a List
                        let meaningful_delimiter = g
                            .delimiter
                            .as_ref()
                            .is_some_and(|d| matches!(d.as_str(), "" | "none" | ": " | " " | ", "));
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
                            let source_order = g.source_order;
                            occurrences.push(ComponentOccurrence {
                                component: list,
                                context: context.clone(),
                                source_order,
                            });
                        } else {
                            // Flatten - add all group occurrences directly
                            occurrences.extend(group_occurrences);
                        }
                    }
                    CslnNode::Condition(c) => {
                        // THEN branch: type-specific if types specified
                        let then_context = if c.if_item_type.is_empty() {
                            BranchContext::Default
                        } else {
                            BranchContext::TypeSpecific(c.if_item_type.clone())
                        };
                        self.collect_occurrences(
                            &c.then_branch,
                            inherited_wrap,
                            &then_context,
                            occurrences,
                        );

                        // ELSE_IF branches: each is type-specific
                        for else_if in &c.else_if_branches {
                            let else_if_context = if else_if.if_item_type.is_empty() {
                                BranchContext::Default
                            } else {
                                BranchContext::TypeSpecific(else_if.if_item_type.clone())
                            };
                            self.collect_occurrences(
                                &else_if.children,
                                inherited_wrap,
                                &else_if_context,
                                occurrences,
                            );
                        }

                        // ELSE branch: always default context
                        if let Some(ref else_nodes) = c.else_branch {
                            self.collect_occurrences(
                                else_nodes,
                                inherited_wrap,
                                &BranchContext::Default,
                                occurrences,
                            );
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
    }

    /// Merge component occurrences with smart suppress semantics.
    ///
    /// Key logic:
    /// - If component appears in DEFAULT branch → base suppress: false (visible by default)
    /// - If component ONLY in type-specific branches → base suppress: true + type overrides
    /// - Collect all type-specific occurrences as overrides with suppress: false
    fn merge_occurrences(&self, occurrences: Vec<ComponentOccurrence>) -> Vec<TemplateComponent> {
        let mut result: Vec<(TemplateComponent, Option<usize>)> = Vec::new();

        // Group occurrences by variable key (including Lists)
        let mut grouped: IndexMap<String, Vec<ComponentOccurrence>> = IndexMap::new();
        let mut list_counter = 0;

        for occurrence in occurrences {
            let key = if let Some(var_key) = self.get_variable_key(&occurrence.component) {
                var_key
            } else if let TemplateComponent::List(ref list) = occurrence.component {
                // Use consistent signature with deduplicate pass
                format!("list:{}", crate::passes::deduplicate::list_signature(list))
            } else {
                // Other non-variable components - give unique key
                list_counter += 1;
                format!("other:{}", list_counter)
            };

            grouped.entry(key).or_default().push(occurrence);
        }

        // Merge each group
        for (_key, mut group) in grouped {
            if group.is_empty() {
                continue;
            }

            // Sort by source_order to preserve macro call order from CSL 1.0.
            // Components without source_order (usize::MAX) sort last.
            // Stable sort preserves existing order for components with same source_order.
            group.sort_by_key(|occ| occ.source_order.unwrap_or(usize::MAX));

            // Check if any occurrence is in Default context
            let has_default = group
                .iter()
                .any(|occ| matches!(occ.context, BranchContext::Default));

            // Start with the first component as the base
            let mut merged = group[0].component.clone();

            // For Lists, propagate type overrides to each item from all branches
            if let TemplateComponent::List(ref mut list) = merged {
                for occurrence in &group {
                    if let BranchContext::TypeSpecific(types) = &occurrence.context {
                        self.add_type_overrides_to_list_items(&mut list.items, types);
                    }
                }
            }

            if has_default {
                // Component appears in default branch → visible by default
                let mut base_rendering = self.get_component_rendering(&merged);
                base_rendering.suppress = Some(false);
                self.set_component_rendering(&mut merged, base_rendering);

                // Add type-specific overrides for any TypeSpecific contexts
                for occurrence in &group {
                    if let BranchContext::TypeSpecific(types) = &occurrence.context {
                        for item_type in types {
                            let type_str = self.item_type_to_string(item_type);
                            let mut rendering = self.get_component_rendering(&occurrence.component);
                            rendering.suppress = Some(false); // Explicitly visible for this type
                            self.add_override_to_component(&mut merged, type_str, rendering);
                        }
                    }
                }
            } else {
                // Component ONLY in type-specific branches → hidden by default
                let mut base_rendering = self.get_component_rendering(&merged);
                base_rendering.suppress = Some(true);
                self.set_component_rendering(&mut merged, base_rendering.clone());

                // Add overrides for each type-specific occurrence
                for occurrence in &group {
                    if let BranchContext::TypeSpecific(types) = &occurrence.context {
                        for item_type in types {
                            let type_str = self.item_type_to_string(item_type);
                            let mut rendering = self.get_component_rendering(&occurrence.component);
                            rendering.suppress = Some(false); // Show for this type
                            self.add_override_to_component(&mut merged, type_str, rendering);
                        }
                    }
                }
            }

            // Track minimum source_order for this merged component
            let min_order = group.iter().filter_map(|occ| occ.source_order).min();
            result.push((merged, min_order));
        }

        // Debug: Print source orders before sorting
        eprintln!("=== Component source orders before sorting ===");
        for (comp, order) in &result {
            let comp_type = match comp {
                TemplateComponent::Contributor(c) => format!("Contributor({:?})", c.contributor),
                TemplateComponent::Date(d) => format!("Date({:?})", d.date),
                TemplateComponent::Title(t) => format!("Title({:?})", t.title),
                TemplateComponent::Number(n) => format!("Number({:?})", n.number),
                TemplateComponent::Variable(v) => format!("Variable({:?})", v.variable),
                TemplateComponent::List(_) => "List".to_string(),
                _ => "Other".to_string(),
            };
            eprintln!("  {} -> order: {:?}", comp_type, order);
        }

        // Sort result by source_order to preserve macro call order
        result.sort_by_key(|(_, order)| order.unwrap_or(usize::MAX));

        eprintln!("=== After sorting ===");
        for (comp, order) in &result {
            let comp_type = match comp {
                TemplateComponent::Contributor(c) => format!("Contributor({:?})", c.contributor),
                TemplateComponent::Date(d) => format!("Date({:?})", d.date),
                TemplateComponent::Title(t) => format!("Title({:?})", t.title),
                _ => "...".to_string(),
            };
            eprintln!("  {} -> order: {:?}", comp_type, order);
        }

        // Extract just the components (drop the ordering metadata)
        result.into_iter().map(|(comp, _)| comp).collect()
    }

    // Old compilation method kept for citation compilation (compile_simple)
    #[allow(dead_code)]
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
        let mut i = 0;

        while i < nodes.len() {
            let node = &nodes[i];

            // Lookahead merge for Text nodes
            if let CslnNode::Text { value } = node {
                if i + 1 < nodes.len() {
                    // Try to compile next node
                    if let Some(mut next_comp) = self.compile_node(&nodes[i + 1]) {
                        // Merge text into prefix
                        let mut rendering = self.get_component_rendering(&next_comp);
                        let mut new_prefix = value.clone();
                        if let Some(p) = rendering.prefix {
                            new_prefix.push_str(&p);
                        }
                        rendering.prefix = Some(new_prefix);
                        self.set_component_rendering(&mut next_comp, rendering);

                        // Apply inherited wrap if applicable
                        if inherited_wrap.0.is_some()
                            && matches!(&next_comp, TemplateComponent::Date(_))
                        {
                            self.apply_wrap_to_component(&mut next_comp, inherited_wrap);
                        }

                        self.add_or_upgrade_component(&mut components, next_comp, current_types);
                        i += 2;
                        continue;
                    }
                }
            }

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
            i += 1;
        }

        components
    }

    #[allow(dead_code)]
    fn add_or_upgrade_component(
        &self,
        components: &mut Vec<TemplateComponent>,
        new_component: TemplateComponent,
        current_types: &[ItemType],
    ) {
        // Recursive search for existing variable
        let mut existing_idx = None;

        for (i, c) in components.iter_mut().enumerate() {
            if self.same_variable(c, &new_component) {
                existing_idx = Some(i);
                break;
            }
            // Also check inside Lists
            if let TemplateComponent::List(ref mut list) = c {
                if self.has_variable_recursive(&list.items, &new_component) {
                    // Variable exists but is nested. We can't easily merge top-level into nested
                    // without knowing the structure. For now, mark as "found" so we can add overrides.
                    // Actually, let's just use a recursive mutation helper.
                    self.add_overrides_recursive(c, &new_component, current_types);
                    return;
                }
            }
        }

        if let Some(idx) = existing_idx {
            if current_types.is_empty() {
                // ... same global logic ...
                let mut rendering = self.get_component_rendering(&components[idx]);
                if rendering.suppress == Some(true) {
                    rendering.suppress = Some(false);
                    self.set_component_rendering(&mut components[idx], rendering);
                }

                if let (TemplateComponent::Date(existing), TemplateComponent::Date(new)) =
                    (&components[idx], &new_component)
                {
                    if existing.rendering.wrap.is_none() && new.rendering.wrap.is_some() {
                        components[idx] = new_component.clone();
                    }
                }
            } else {
                // Add overrides to existing top-level component
                self.add_overrides_to_existing(&mut components[idx], &new_component, current_types);
            }
        } else {
            // ... same NEW component logic ...
            let mut component_to_add = new_component;
            if !current_types.is_empty() {
                if let TemplateComponent::List(ref mut list) = component_to_add {
                    // For Lists, propagate type-specific overrides to each item
                    self.add_type_overrides_to_list_items(&mut list.items, current_types);
                } else {
                    let mut base = self.get_component_rendering(&component_to_add);
                    base.suppress = Some(true);
                    self.set_component_rendering(&mut component_to_add, base.clone());

                    for item_type in current_types {
                        let type_str = self.item_type_to_string(item_type);
                        let mut unsuppressed = base.clone();
                        unsuppressed.suppress = Some(false);
                        self.add_override_to_component(
                            &mut component_to_add,
                            type_str,
                            unsuppressed,
                        );
                    }
                }
            }
            components.push(component_to_add);
        }
    }

    fn has_variable_recursive(
        &self,
        items: &[TemplateComponent],
        target: &TemplateComponent,
    ) -> bool {
        for item in items {
            if self.same_variable(item, target) {
                return true;
            }
            if let TemplateComponent::List(list) = item {
                if self.has_variable_recursive(&list.items, target) {
                    return true;
                }
            }
        }
        false
    }

    /// Add type-specific overrides to all items within a List.
    /// This ensures that when a List is created inside a type-specific branch,
    /// all its items get the appropriate suppress=true with type-specific unsuppress.
    #[allow(dead_code)]
    fn add_type_overrides_to_list_items(
        &self,
        items: &mut [TemplateComponent],
        current_types: &[ItemType],
    ) {
        for item in items.iter_mut() {
            match item {
                TemplateComponent::List(ref mut nested_list) => {
                    // Recursively process nested lists
                    self.add_type_overrides_to_list_items(&mut nested_list.items, current_types);
                }
                _ => {
                    // Add suppress=true to base, with type-specific unsuppress overrides
                    let mut base = self.get_component_rendering(item);
                    base.suppress = Some(true);
                    self.set_component_rendering(item, base.clone());

                    for item_type in current_types {
                        let type_str = self.item_type_to_string(item_type);
                        let mut unsuppressed = base.clone();
                        unsuppressed.suppress = Some(false);
                        self.add_override_to_component(item, type_str, unsuppressed);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn add_overrides_recursive(
        &self,
        component: &mut TemplateComponent,
        new_comp: &TemplateComponent,
        current_types: &[ItemType],
    ) {
        if self.same_variable(component, new_comp) {
            if current_types.is_empty() {
                // Empty types means this is the default case - unsuppress the component
                let mut rendering = self.get_component_rendering(component);
                if rendering.suppress == Some(true) {
                    rendering.suppress = Some(false);
                    self.set_component_rendering(component, rendering);
                }
            } else {
                // Add type-specific overrides
                self.add_overrides_to_existing(component, new_comp, current_types);
            }
            return;
        }
        if let TemplateComponent::List(ref mut list) = component {
            for item in &mut list.items {
                self.add_overrides_recursive(item, new_comp, current_types);
            }
        }
    }

    /// Get a debug name for a component
    #[allow(dead_code)]
    fn get_component_name(&self, comp: &TemplateComponent) -> String {
        match comp {
            TemplateComponent::Contributor(c) => format!("contributor:{:?}", c.contributor),
            TemplateComponent::Date(d) => format!("date:{:?}", d.date),
            TemplateComponent::Title(t) => format!("title:{:?}", t.title),
            TemplateComponent::Number(n) => format!("number:{:?}", n.number),
            TemplateComponent::Variable(v) => format!("variable:{:?}", v.variable),
            TemplateComponent::List(_) => "List".to_string(),
            _ => "unknown".to_string(),
        }
    }

    #[allow(dead_code)]
    fn add_overrides_to_existing(
        &self,
        existing: &mut TemplateComponent,
        new_comp: &TemplateComponent,
        current_types: &[ItemType],
    ) {
        let base_rendering = self.get_component_rendering(new_comp);
        let new_overrides = self.get_component_overrides(new_comp);

        for item_type in current_types {
            let type_str = self.item_type_to_string(item_type);
            let mut rendering = new_overrides
                .as_ref()
                .and_then(|ovr| ovr.get(&type_str))
                .cloned()
                .unwrap_or_else(|| base_rendering.clone());

            if rendering.suppress.is_none() || rendering.suppress == Some(true) {
                rendering.suppress = Some(false);
            }

            self.add_override_to_component(existing, type_str, rendering);
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

    /// Compile bibliography, preserving CSL 1.0 layout order.
    pub fn compile_bibliography(
        &self,
        nodes: &[CslnNode],
        _is_numeric: bool,
    ) -> Vec<TemplateComponent> {
        // DISABLED: Sorting was needed to work around HashMap's random iteration order.
        // Now that we use IndexMap, we preserve the CSL 1.0 layout order naturally.
        self.compile(nodes)
    }

    /// Compile bibliography with type-specific templates.
    ///
    /// Uses the new occurrence-based compilation approach which correctly handles
    /// mutually exclusive conditional branches with proper suppress semantics.
    pub fn compile_bibliography_with_types(
        &self,
        nodes: &[CslnNode],
        _is_numeric: bool,
    ) -> (
        Vec<TemplateComponent>,
        HashMap<String, Vec<TemplateComponent>>,
    ) {
        // Compile using the new occurrence-based approach
        // This handles suppress semantics correctly without needing deduplication
        let mut default_template = self.compile(nodes);

        // DISABLED: Hardcoded sorting doesn't work for all styles (e.g., numeric styles have different order).
        // A general solution requires preserving macro call order from CSL 1.0 during parsing.
        // self.sort_bibliography_components(&mut default_template, _is_numeric);

        // Deduplicate number components (edition, volume, issue) in nested lists
        crate::passes::deduplicate::deduplicate_numbers_in_lists(&mut default_template);

        // Deduplicate date components (issued, accessed) in nested lists
        crate::passes::deduplicate::deduplicate_dates_in_lists(&mut default_template);

        // Fix duplicate variables (e.g., date appearing both in List and standalone)
        self.fix_duplicate_variables(&mut default_template);

        // Type-specific template generation is disabled for now.
        // Future work: properly merge common components with type-specific branches.
        let type_templates: HashMap<String, Vec<TemplateComponent>> = HashMap::new();

        (default_template, type_templates)
    }

    /// Fix duplicate variables that appear both in Lists and as standalone components.
    ///
    /// When a variable (like date:issued) appears:
    /// 1. Inside a List for specific types (e.g., article-journal)
    /// 2. As a standalone component for all types
    ///
    /// Both will render for those types, causing duplication. This method adds
    /// suppress overrides to standalone components for types where the variable
    /// already appears in a List.
    fn fix_duplicate_variables(&self, components: &mut [TemplateComponent]) {
        // Step 1: Collect which variables appear in Lists, and for which types
        let mut list_vars: HashMap<String, Vec<String>> = HashMap::new();
        let mut default_list_vars: Vec<String> = Vec::new();

        for component in components.iter() {
            if let TemplateComponent::List(list) = component {
                // Check if this List is visible by default
                let base_rendering = self.get_component_rendering(component);
                let is_default_visible = base_rendering.suppress != Some(true);

                // Extract all variables from this List
                let vars = self.extract_list_vars(list);

                if is_default_visible {
                    for var in &vars {
                        if !default_list_vars.contains(var) {
                            default_list_vars.push(var.clone());
                        }
                    }
                } else {
                    let visible_types = self.get_visible_types_for_component(component);
                    for var in vars {
                        list_vars
                            .entry(var)
                            .or_default()
                            .extend(visible_types.clone());
                    }
                }
            }
        }

        // Step 2: For each standalone component, add suppress overrides for types
        // where it already appears in a List
        for component in components.iter_mut() {
            // Skip Lists - we only care about standalone components
            if matches!(component, TemplateComponent::List(_)) {
                continue;
            }

            // Get the variable key for this component
            if let Some(var_key) = self.get_variable_key(component) {
                // If it appears in a default-visible List, suppress it by default
                if default_list_vars.contains(&var_key) {
                    let mut rendering = self.get_component_rendering(component);
                    rendering.suppress = Some(true);
                    self.set_component_rendering(component, rendering);
                } else if let Some(types_in_lists) = list_vars.get(&var_key) {
                    // Add suppress overrides for those types
                    for type_str in types_in_lists {
                        let mut suppressed = self.get_component_rendering(component);
                        suppressed.suppress = Some(true);
                        self.add_override_to_component(component, type_str.clone(), suppressed);
                    }
                }
            }
        }
    }

    /// Get the list of types for which a component is visible.
    ///
    /// Returns type names where suppress=false (either by default or via overrides).
    fn get_visible_types_for_component(&self, component: &TemplateComponent) -> Vec<String> {
        let base_rendering = self.get_component_rendering(component);
        let overrides = self.get_component_overrides(component);

        let mut visible_types = Vec::new();

        // If component has suppress=true by default, only count types with suppress=false overrides
        if base_rendering.suppress == Some(true) {
            if let Some(ovr) = overrides {
                for (type_str, rendering) in ovr {
                    if rendering.suppress != Some(true) {
                        visible_types.push(type_str);
                    }
                }
            }
        }
        // If component is visible by default (suppress=false or None),
        // we would need to list all types except those with suppress=true overrides.
        // For now, we skip this case as it would require enumerating all possible types.

        visible_types
    }

    /// Old deduplication method - no longer needed with occurrence-based compilation.
    /// Kept for reference but not used in new code path.
    #[allow(dead_code)]
    fn deduplicate_and_flatten(
        &self,
        components: Vec<TemplateComponent>,
    ) -> Vec<TemplateComponent> {
        let mut seen_vars: Vec<String> = Vec::new();
        let mut seen_list_signatures: Vec<String> = Vec::new();
        let mut result: Vec<TemplateComponent> = Vec::new();

        // First pass: add all non-List components and track their keys
        // When encountering duplicates, merge their overrides
        for component in &components {
            if !matches!(component, TemplateComponent::List(_)) {
                if let Some(key) = self.get_variable_key(component) {
                    if let Some(existing_idx) = seen_vars.iter().position(|k| k == &key) {
                        // Duplicate found - merge overrides into existing component
                        self.merge_overrides_into(&mut result[existing_idx], component);
                    } else {
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

    #[allow(dead_code)]
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
            delimiter: list.delimiter.clone(),
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

    #[allow(dead_code)]
    fn merge_overrides_into(&self, target: &mut TemplateComponent, source: &TemplateComponent) {
        if let Some(source_overrides) = self.get_component_overrides(source) {
            let target_overrides = match target {
                TemplateComponent::Contributor(c) => &mut c.overrides,
                TemplateComponent::Date(d) => &mut d.overrides,
                TemplateComponent::Number(n) => &mut n.overrides,
                TemplateComponent::Title(t) => &mut t.overrides,
                TemplateComponent::Variable(v) => &mut v.overrides,
                TemplateComponent::List(l) => &mut l.overrides,
                _ => return,
            };

            let overrides_map = target_overrides.get_or_insert_with(HashMap::new);
            for (k, v) in source_overrides {
                overrides_map.entry(k).or_insert(v);
            }
        }
    }

    /// Get a unique key for a component for deduplication purposes.
    fn get_variable_key(&self, component: &TemplateComponent) -> Option<String> {
        match component {
            TemplateComponent::Contributor(c) => Some(format!("contributor:{:?}", c.contributor)),
            TemplateComponent::Date(d) => Some(format!("date:{:?}", d.date)),
            TemplateComponent::Title(t) => Some(format!("title:{:?}", t.title)),
            TemplateComponent::Number(n) => Some(format!("number:{:?}", n.number)),
            TemplateComponent::Variable(v) => Some(format!("variable:{:?}", v.variable)),
            TemplateComponent::Term(t) => Some(format!("term:{:?}", t.term)),
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
    #[allow(dead_code)]
    fn sort_bibliography_components(&self, components: &mut [TemplateComponent], is_numeric: bool) {
        components.sort_by_key(|c| match c {
            // Citation number goes first for numeric bibliography styles
            TemplateComponent::Number(n) if n.number == NumberVariable::CitationNumber => 0,
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 1,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => {
                if is_numeric {
                    20
                } else {
                    2
                }
            }
            TemplateComponent::Title(t) if t.title == TitleType::Primary => 3,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial => 4,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph => 5,
            TemplateComponent::Number(_) => 6,
            TemplateComponent::Variable(_) => 7,
            TemplateComponent::Contributor(_) => 8,
            TemplateComponent::Date(_) => 9,
            TemplateComponent::Title(_) => 10,
            TemplateComponent::List(l) => {
                if self.has_variable_recursive(
                    &l.items,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                ) {
                    3
                } else if self.has_variable_recursive(
                    &l.items,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentSerial,
                        ..Default::default()
                    }),
                ) {
                    4
                } else if self.has_variable_recursive(
                    &l.items,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentMonograph,
                        ..Default::default()
                    }),
                ) {
                    5
                } else {
                    11
                }
            }
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
            (TemplateComponent::Term(t1), TemplateComponent::Term(t2)) => t1.term == t2.term,
            _ => false,
        }
    }

    /// Try to compile a single node into a TemplateComponent.
    fn compile_node(&self, node: &CslnNode) -> Option<TemplateComponent> {
        match node {
            CslnNode::Names(names) => self.compile_names(names),
            CslnNode::Date(date) => self.compile_date(date),
            CslnNode::Variable(var) => self.compile_variable(var),
            CslnNode::Term(term) => self.compile_term(term),
            _ => None,
        }
    }

    /// Compile a Names block into a Contributor component.
    fn compile_names(&self, names: &csln_core::NamesBlock) -> Option<TemplateComponent> {
        // Try to map the primary variable to a role
        let primary_role = self.map_variable_to_role(&names.variable);

        // Check if we should use a substitute instead of the primary
        // Rare contributor roles (composer, illustrator) often have author as first substitute
        let role = if let Some(role) = primary_role {
            // If primary is a rare role and we have substitutes, prefer the first common one
            let rare_roles = [
                ContributorRole::Composer,
                ContributorRole::Illustrator,
                ContributorRole::Interviewer,
                ContributorRole::Inventor,
                ContributorRole::Counsel,
                ContributorRole::CollectionEditor,
                ContributorRole::EditorialDirector,
                ContributorRole::OriginalAuthor,
                ContributorRole::ReviewedAuthor,
            ];

            if rare_roles.contains(&role) && !names.options.substitute.is_empty() {
                // Try to find a common role in the substitute list
                names
                    .options
                    .substitute
                    .iter()
                    .find_map(|var| self.map_variable_to_role(var))
                    .unwrap_or(role) // Fallback to primary if no valid substitute
            } else {
                role
            }
        } else {
            return None;
        };

        let form = match names.options.mode {
            Some(csln_core::NameMode::Short) => ContributorForm::Short,
            Some(csln_core::NameMode::Count) => ContributorForm::Short, // Map count to short
            _ => ContributorForm::Long,
        };

        let and = names.options.and.as_ref().map(|a| match a {
            csln_core::AndTerm::Text => csln_core::options::AndOptions::Text,
            csln_core::AndTerm::Symbol => csln_core::options::AndOptions::Symbol,
        });

        let shorten = names.options.et_al.as_ref().map(|et| {
            csln_core::options::ShortenListOptions {
                min: et.min,
                use_first: et.use_first,
                use_last: None, // Legacy CSL 1.0 et-al doesn't have use_last
                and_others: csln_core::options::AndOtherOptions::EtAl,
                delimiter_precedes_last: match names.options.delimiter_precedes_last {
                    Some(csln_core::DelimiterPrecedes::Always) => {
                        csln_core::options::DelimiterPrecedesLast::Always
                    }
                    Some(csln_core::DelimiterPrecedes::Never) => {
                        csln_core::options::DelimiterPrecedesLast::Never
                    }
                    Some(csln_core::DelimiterPrecedes::AfterInvertedName) => {
                        csln_core::options::DelimiterPrecedesLast::AfterInvertedName
                    }
                    _ => csln_core::options::DelimiterPrecedesLast::Contextual,
                },
            }
        });

        Some(TemplateComponent::Contributor(TemplateContributor {
            contributor: role,
            form,
            name_order: None, // Use global setting by default
            delimiter: names.options.delimiter.clone(),
            sort_separator: names.options.sort_separator.clone(),
            shorten,
            and,
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

    /// Compile a Term block into a Term component.
    fn compile_term(&self, term: &csln_core::TermBlock) -> Option<TemplateComponent> {
        Some(TemplateComponent::Term(csln_core::template::TemplateTerm {
            term: term.term,
            form: Some(term.form),
            rendering: self.convert_formatting(&term.formatting),
            overrides: None,
            ..Default::default()
        }))
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

            // Extract label form if present
            let label_form = var.label.as_ref().map(|l| self.map_label_form(&l.form));

            return Some(TemplateComponent::Number(TemplateNumber {
                number: num_var,
                form: None,
                label_form,
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

    /// Map LabelForm from legacy intermediate representation to template type.
    fn map_label_form(&self, form: &csln_core::LabelForm) -> csln_core::template::LabelForm {
        match form {
            csln_core::LabelForm::Long => csln_core::template::LabelForm::Long,
            csln_core::LabelForm::Short => csln_core::template::LabelForm::Short,
            csln_core::LabelForm::Symbol => csln_core::template::LabelForm::Symbol,
            // Verb and VerbShort don't exist in template::LabelForm, map to Long
            csln_core::LabelForm::Verb | csln_core::LabelForm::VerbShort => {
                csln_core::template::LabelForm::Long
            }
        }
    }

    /// Convert FormattingOptions to Rendering.
    fn convert_formatting(&self, fmt: &FormattingOptions) -> Rendering {
        // Infer wrap from prefix/suffix patterns
        let (mut wrap, remaining_prefix, remaining_suffix) =
            Self::infer_wrap_from_affixes(&fmt.prefix, &fmt.suffix);

        // quotes="true" in CSL maps to wrap: quotes in CSLN
        if fmt.quotes == Some(true) {
            wrap = Some(csln_core::template::WrapPunctuation::Quotes);
        }

        // If wrap is detected, remaining affixes are INNER.
        // If no wrap, affixes are OUTER (default prefix/suffix).
        let (prefix, suffix, inner_prefix, inner_suffix) = if wrap.is_some() {
            (None, None, remaining_prefix, remaining_suffix)
        } else {
            (remaining_prefix, remaining_suffix, None, None)
        };

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
            inner_prefix,
            inner_suffix,
            wrap,
            suppress: None,
            initialize_with: None,
            strip_periods: fmt.strip_periods,
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

        // Helper to apply rendering
        let apply = |rendering: &mut Rendering| {
            if rendering.wrap.is_none() && wrap.is_some() {
                rendering.wrap = wrap.clone();
            }

            // If wrap is being applied (or was already present and we are merging inner content),
            // then prefix/suffix should go to inner_prefix/inner_suffix.
            // If no wrap involved, they go to prefix/suffix.
            // Note: This logic assumes group_wrap comes from infer_wrap_from_affixes,
            // so if wrap is Some, prefix/suffix are "remaining" (inner).
            // If wrap is None, prefix/suffix are just outer.

            if wrap.is_some() {
                // Applying a wrap -> affixes are inner
                if rendering.inner_prefix.is_none() && prefix.is_some() {
                    rendering.inner_prefix = prefix.clone();
                }
                if rendering.inner_suffix.is_none() && suffix.is_some() {
                    rendering.inner_suffix = suffix.clone();
                }
            } else {
                // No wrap -> affixes are outer
                if rendering.prefix.is_none() && prefix.is_some() {
                    rendering.prefix = prefix.clone();
                }
                if rendering.suffix.is_none() && suffix.is_some() {
                    rendering.suffix = suffix.clone();
                }
            }
        };

        match component {
            TemplateComponent::Date(d) => apply(&mut d.rendering),
            TemplateComponent::Contributor(c) => apply(&mut c.rendering),
            TemplateComponent::Title(t) => apply(&mut t.rendering),
            TemplateComponent::Number(n) => apply(&mut n.rendering),
            TemplateComponent::Variable(v) => apply(&mut v.rendering),
            _ => {} // List and future variants - don't modify
        }
    }
    /// Map a String delimiter to DelimiterPunctuation.
    /// Preserves custom delimiters that don't match standard patterns.
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
            _ => Some(DelimiterPunctuation::Custom(d.clone())),
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
            TemplateComponent::Term(t) => t.rendering.clone(),
            _ => Rendering::default(),
        }
    }

    /// Set the rendering options for a component.
    fn set_component_rendering(&self, component: &mut TemplateComponent, rendering: Rendering) {
        match component {
            TemplateComponent::Contributor(c) => c.rendering = rendering,
            TemplateComponent::Date(d) => d.rendering = rendering,
            TemplateComponent::Number(n) => n.rendering = rendering,
            TemplateComponent::Title(t) => t.rendering = rendering,
            TemplateComponent::Variable(v) => v.rendering = rendering,
            TemplateComponent::List(l) => l.rendering = rendering,
            TemplateComponent::Term(t) => t.rendering = rendering,
            _ => {}
        }
    }

    #[allow(dead_code)]
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
            TemplateComponent::List(l) => l.overrides.clone(),
            TemplateComponent::Term(t) => t.overrides.clone(),
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
            TemplateComponent::Term(t) => {
                t.overrides
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

    /// Extracts the source_order from a CslnNode, if present.
    /// Returns the order value or usize::MAX if not set (sorts last).
    fn extract_source_order(&self, node: &CslnNode) -> Option<usize> {
        let order = match node {
            CslnNode::Variable(v) => v.source_order,
            CslnNode::Date(d) => d.source_order,
            CslnNode::Names(n) => n.source_order,
            CslnNode::Group(g) => g.source_order,
            CslnNode::Term(t) => t.source_order,
            _ => None,
        };
        eprintln!(
            "TemplateCompiler: extract_source_order({:?}) = {:?}",
            match node {
                CslnNode::Variable(v) => format!("Variable({:?})", v.variable),
                CslnNode::Date(d) => format!("Date({:?})", d.variable),
                CslnNode::Names(n) => format!("Names({:?})", n.variable),
                CslnNode::Group(_) => "Group".to_string(),
                CslnNode::Text { value } => format!("Text({})", value),
                CslnNode::Condition(_) => "Condition".to_string(),
                CslnNode::Term(t) => format!("Term({:?})", t.term),
            },
            order
        );
        order
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
            source_order: None,
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
            source_order: None,
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
            source_order: None,
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
            source_order: None,
        });

        let result = compiler.compile(&[var]);
        assert_eq!(result.len(), 1);

        if let TemplateComponent::Variable(v) = &result[0] {
            assert_eq!(v.variable, SimpleVariable::Doi);
        } else {
            panic!("Expected Variable component");
        }
    }

    #[test]
    fn test_compile_recursive_variable_discovery() {
        use csln_core::{ConditionBlock, ItemType, VariableBlock};
        let compiler = TemplateCompiler;

        // Branch 1: type="book" -> Group -> Publisher
        let pub_var = VariableBlock {
            variable: Variable::Publisher,
            label: None,
            formatting: FormattingOptions::default(),
            overrides: HashMap::new(),
            source_order: None,
        };
        let branch1 = CslnNode::Condition(ConditionBlock {
            if_item_type: vec![ItemType::Book],
            if_variables: Vec::new(),
            then_branch: vec![CslnNode::Group(csln_core::GroupBlock {
                children: vec![CslnNode::Variable(pub_var.clone())],
                delimiter: None,
                formatting: FormattingOptions::default(),
                source_order: None,
            })],
            else_if_branches: Vec::new(),
            else_branch: None,
        });

        // Branch 2: type="chapter" -> Group -> Publisher
        let branch2 = CslnNode::Condition(ConditionBlock {
            if_item_type: vec![ItemType::Chapter],
            if_variables: Vec::new(),
            then_branch: vec![CslnNode::Group(csln_core::GroupBlock {
                children: vec![CslnNode::Variable(pub_var.clone())],
                delimiter: None,
                formatting: FormattingOptions::default(),
                source_order: None,
            })],
            else_if_branches: Vec::new(),
            else_branch: None,
        });

        let result = compiler.compile(&[branch1, branch2]);

        // Find publisher (it might be inside a List or flattened)
        let has_pub = compiler.has_variable_recursive(
            &result,
            &TemplateComponent::Variable(csln_core::template::TemplateVariable {
                variable: SimpleVariable::Publisher,
                ..Default::default()
            }),
        );
        assert!(has_pub, "Publisher should be in the result");

        // Verify it has overrides for BOTH book and chapter
        fn check_overrides(items: &[TemplateComponent]) {
            for item in items {
                if let TemplateComponent::Variable(v) = item {
                    if v.variable == SimpleVariable::Publisher {
                        assert!(v.overrides.as_ref().unwrap().contains_key("book"));
                        assert!(v.overrides.as_ref().unwrap().contains_key("chapter"));
                    }
                }
                if let TemplateComponent::List(l) = item {
                    check_overrides(&l.items);
                }
            }
        }
        check_overrides(&result);
    }
}
