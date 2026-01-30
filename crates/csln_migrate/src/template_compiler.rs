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
        ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
        SimpleVariable, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
        TemplateTitle, TemplateVariable, TitleType,
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
    pub fn compile(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = Vec::new();

        for node in nodes {
            if let Some(component) = self.compile_node(node) {
                components.push(component);
            } else {
                match node {
                    CslnNode::Group(g) => {
                        // Flatten groups - recurse into children
                        components.extend(self.compile(&g.children));
                    }
                    CslnNode::Condition(c) => {
                        // Process then_branch (most common case)
                        // TODO: Use overrides for type-specific formatting
                        components.extend(self.compile(&c.then_branch));

                        // Process all else-if branches to not lose type-specific components
                        for else_if in &c.else_if_branches {
                            let branch_components = self.compile(&else_if.children);
                            for bc in branch_components {
                                if !components.iter().any(|c| self.same_variable(c, &bc)) {
                                    components.push(bc);
                                }
                            }
                        }

                        // Also process else_branch
                        if let Some(ref else_nodes) = c.else_branch {
                            let else_components = self.compile(else_nodes);
                            for ec in else_components {
                                if !components.iter().any(|c| self.same_variable(c, &ec)) {
                                    components.push(ec);
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

    /// Compile and sort for citation output (author first, then date).
    /// Uses simplified compile that skips else branches to avoid extra fields.
    pub fn compile_citation(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile_simple(nodes);
        self.sort_citation_components(&mut components);
        components
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
                                    let branch_components =
                                        self.compile_simple(&else_if.children);
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
    ) -> (Vec<TemplateComponent>, HashMap<String, Vec<TemplateComponent>>) {
        // Compile the default template
        let mut default_template = self.compile(nodes);
        self.sort_bibliography_components(&mut default_template);

        // Type-specific template generation is disabled for now.
        // The compile_for_type approach produces malformed templates.
        // Future work: properly merge common components with type-specific branches.
        let type_templates: HashMap<String, Vec<TemplateComponent>> = HashMap::new();

        (default_template, type_templates)
    }

    /// Collect all ItemTypes that have specific branches in conditions.
    fn collect_types_with_branches(&self, nodes: &[CslnNode]) -> Vec<ItemType> {
        let mut types = Vec::new();
        self.collect_types_recursive(nodes, &mut types);
        types.sort_by_key(|t| self.item_type_to_string(t));
        types.dedup_by_key(|t| self.item_type_to_string(t));
        types
    }

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
    fn compile_for_type(&self, nodes: &[CslnNode], target_type: &ItemType) -> Vec<TemplateComponent> {
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
                            || c.else_if_branches.iter().any(|b| !b.if_item_type.is_empty());

                        if has_type_condition {
                            // Select the matching branch for target_type
                            if c.if_item_type.contains(target_type) {
                                components.extend(self.compile_for_type(&c.then_branch, target_type));
                            } else {
                                // Check else-if branches
                                let mut found = false;
                                for else_if in &c.else_if_branches {
                                    if else_if.if_item_type.contains(target_type) {
                                        components.extend(self.compile_for_type(&else_if.children, target_type));
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    // Fall back to else branch
                                    if let Some(ref else_nodes) = c.else_branch {
                                        components.extend(self.compile_for_type(else_nodes, target_type));
                                    }
                                }
                            }
                        } else {
                            // Not a type condition, use default compile behavior
                            components.extend(self.compile_for_type(&c.then_branch, target_type));
                            if let Some(ref else_nodes) = c.else_branch {
                                let else_components = self.compile_for_type(else_nodes, target_type);
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
            return Some(TemplateComponent::Title(TemplateTitle {
                title: title_type,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
                ..Default::default()
            }));
        }

        // Check if it's a number
        if let Some(num_var) = self.map_variable_to_number(&var.variable) {
            return Some(TemplateComponent::Number(TemplateNumber {
                number: num_var,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
                ..Default::default()
            }));
        }

        // Check if it's a simple variable
        if let Some(simple_var) = self.map_variable_to_simple(&var.variable) {
            return Some(TemplateComponent::Variable(TemplateVariable {
                variable: simple_var,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
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
            prefix: fmt.prefix.clone(),
            suffix: fmt.suffix.clone(),
            wrap: None, // Would need to infer from prefix/suffix patterns like "(" and ")"
            suppress: None,
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
