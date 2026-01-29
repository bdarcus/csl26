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
    CslnNode, FormattingOptions, Variable,
};

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
                        // For now, take the then_branch (most common case)
                        // TODO: Use overrides for type-specific formatting
                        components.extend(self.compile(&c.then_branch));
                        // Also process else_branch to not lose components
                        if let Some(ref else_nodes) = c.else_branch {
                            // Only add else components if they're different from then
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
    pub fn compile_citation(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile(nodes);
        self.sort_citation_components(&mut components);
        components
    }

    /// Compile and sort for bibliography output.
    pub fn compile_bibliography(&self, nodes: &[CslnNode]) -> Vec<TemplateComponent> {
        let mut components = self.compile(nodes);
        self.sort_bibliography_components(&mut components);
        components
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

    /// Sort components for bibliography: author, date, title, then rest.
    fn sort_bibliography_components(&self, components: &mut [TemplateComponent]) {
        components.sort_by_key(|c| match c {
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 0,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => 1,
            TemplateComponent::Title(t) if t.title == TitleType::Primary => 2,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial => 3,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph => 4,
            TemplateComponent::Number(_) => 5,
            TemplateComponent::Variable(_) => 6,
            TemplateComponent::Contributor(_) => 7,
            TemplateComponent::Date(_) => 8,
            TemplateComponent::Title(_) => 9,
            TemplateComponent::List(_) => 10,
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
            }));
        }

        // Check if it's a title
        if let Some(title_type) = self.map_variable_to_title(&var.variable) {
            return Some(TemplateComponent::Title(TemplateTitle {
                title: title_type,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
            }));
        }

        // Check if it's a number
        if let Some(num_var) = self.map_variable_to_number(&var.variable) {
            return Some(TemplateComponent::Number(TemplateNumber {
                number: num_var,
                form: None,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
            }));
        }

        // Check if it's a simple variable
        if let Some(simple_var) = self.map_variable_to_simple(&var.variable) {
            return Some(TemplateComponent::Variable(TemplateVariable {
                variable: simple_var,
                rendering: self.convert_formatting(&var.formatting),
                overrides: None,
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
