/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::{
    ContributorForm, ContributorRole, DateForm, DateVariable, Node, NumberVariable, SimpleVariable,
    TemplateCompiler, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    TemplateTitle, TemplateVariable, TitleType, Variable,
    formatting::{convert_formatting, map_label_form},
};

impl TemplateCompiler {
    pub(super) fn compile_node(&self, node: &Node) -> Option<TemplateComponent> {
        match node {
            Node::Names(names) => self.compile_names(names),
            Node::Date(date) => self.compile_date(date),
            Node::Variable(var) => self.compile_variable(var),
            Node::Term(term) => self.compile_term(term),
            _ => None,
        }
    }

    /// Compile a Names block into a Contributor component.
    pub(super) fn compile_names(&self, names: &crate::ir::NamesBlock) -> Option<TemplateComponent> {
        let mut roles = self.resolve_names_roles(names)?;

        let form = match names.options.mode {
            Some(crate::ir::NameMode::Short) => ContributorForm::Short,
            Some(crate::ir::NameMode::Count) => ContributorForm::Short, // Map count to short
            _ => ContributorForm::Long,
        };

        let and = names.options.and.as_ref().map(|a| match a {
            crate::ir::AndTerm::Text => citum_schema::options::AndOptions::Text,
            crate::ir::AndTerm::Symbol => citum_schema::options::AndOptions::Symbol,
        });

        let shorten = names.options.et_al.as_ref().map(|et| {
            citum_schema::options::ShortenListOptions {
                min: et.min,
                use_first: et.use_first,
                use_last: None, // Legacy CSL 1.0 et-al doesn't have use_last
                and_others: citum_schema::options::AndOtherOptions::EtAl,
                delimiter_precedes_last: match names.options.delimiter_precedes_last {
                    Some(crate::ir::DelimiterPrecedes::Always) => {
                        citum_schema::options::DelimiterPrecedesLast::Always
                    }
                    Some(crate::ir::DelimiterPrecedes::Never) => {
                        citum_schema::options::DelimiterPrecedesLast::Never
                    }
                    Some(crate::ir::DelimiterPrecedes::AfterInvertedName) => {
                        citum_schema::options::DelimiterPrecedesLast::AfterInvertedName
                    }
                    _ => citum_schema::options::DelimiterPrecedesLast::Contextual,
                },
                subsequent_min: et.subsequent.as_ref().map(|s| s.min),
                subsequent_use_first: et.subsequent.as_ref().map(|s| s.use_first),
            }
        });

        let mut rendering = convert_formatting(&names.formatting);
        if let Some(label) = &names.options.label {
            rendering.strip_periods = label.formatting.strip_periods.or(rendering.strip_periods);
        }

        let name_order = match &names.options.name_as_sort_order {
            Some(crate::ir::NameAsSortOrder::First) => {
                Some(citum_schema::template::NameOrder::FamilyFirstOnly)
            }
            Some(crate::ir::NameAsSortOrder::All) => {
                Some(citum_schema::template::NameOrder::FamilyFirst)
            }
            None => None,
        };

        let merge = Self::compile_names_merge(names, &roles);
        let contributor = if roles.len() == 1 {
            roles.remove(0).into()
        } else {
            roles.into()
        };

        Some(TemplateComponent::Contributor(TemplateContributor {
            contributor,
            form,
            merge,
            name_order,
            delimiter: names
                .options
                .delimiter
                .as_deref()
                .map(citum_schema::template::DelimiterPunctuation::from_csl_string),
            sort_separator: names.options.sort_separator.clone(),
            shorten,
            and,
            rendering,
            ..Default::default()
        }))
    }

    fn resolve_names_roles(&self, names: &crate::ir::NamesBlock) -> Option<Vec<ContributorRole>> {
        let mut roles = names
            .variables
            .iter()
            .filter_map(|variable| self.map_variable_to_role(variable))
            .collect::<Vec<_>>();
        let primary_role = roles.first()?.clone();
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
        if roles.len() == 1
            && rare_roles.contains(&primary_role)
            && let Some(replacement) = names
                .options
                .substitute
                .iter()
                .find_map(|variable| self.map_variable_to_role(variable))
            && let Some(role) = roles.first_mut()
        {
            *role = replacement;
        }
        Some(roles)
    }

    fn compile_names_merge(
        names: &crate::ir::NamesBlock,
        roles: &[ContributorRole],
    ) -> Option<citum_schema::template::ContributorMerge> {
        if roles.len() < 2 {
            return None;
        }
        let mut role_overrides = std::collections::HashMap::new();
        if let Some(label) = &names.options.label {
            let (form, placement) = match label.form {
                crate::ir::LabelForm::Short | crate::ir::LabelForm::Symbol => (
                    citum_schema::template::RoleLabelForm::Short,
                    citum_schema::template::LabelPlacement::Suffix,
                ),
                crate::ir::LabelForm::Verb | crate::ir::LabelForm::VerbShort => (
                    citum_schema::template::RoleLabelForm::Long,
                    citum_schema::template::LabelPlacement::Prefix,
                ),
                crate::ir::LabelForm::Long => (
                    citum_schema::template::RoleLabelForm::Long,
                    citum_schema::template::LabelPlacement::Suffix,
                ),
            };
            // Label every declared role individually: the engine resolves
            // authored combined terms (e.g. `editor-translator`) from the
            // locale automatically for combined entries, and resolves
            // single-role labels by the entry's actual role.
            for role in roles {
                role_overrides.insert(
                    role.clone(),
                    citum_schema::template::ContributorMergeRole {
                        labels: Some(citum_schema::template::ContributorLabelMode::Collective),
                        label: Some(citum_schema::template::RoleLabel {
                            term: role.as_str().to_string(),
                            form: form.clone(),
                            placement: placement.clone(),
                            text_case: None,
                            wrap: None,
                            prefix: label
                                .formatting
                                .prefix
                                .as_deref()
                                .map(citum_schema::template::DelimiterPunctuation::from_csl_string),
                            suffix: label
                                .formatting
                                .suffix
                                .as_deref()
                                .map(citum_schema::template::DelimiterPunctuation::from_csl_string),
                        }),
                    },
                );
            }
        }
        Some(citum_schema::template::ContributorMerge {
            order: citum_schema::template::ContributorMergeOrder::Role,
            labels: if names.options.label.is_some() {
                citum_schema::template::ContributorLabelMode::Collective
            } else {
                citum_schema::template::ContributorLabelMode::None
            },
            roles: role_overrides,
            combine_same_person: true,
            role_conjunction: None,
        })
    }

    /// Map a Variable to `ContributorRole`.
    pub(super) fn map_variable_to_role(&self, var: &Variable) -> Option<ContributorRole> {
        match var {
            Variable::Author => Some(ContributorRole::Author),
            Variable::Editor => Some(ContributorRole::Editor),
            Variable::Translator => Some(ContributorRole::Translator),
            Variable::Director => Some(ContributorRole::Director),
            Variable::Writer => Some(ContributorRole::Writer),
            Variable::Producer => Some(ContributorRole::Producer),
            Variable::Performer => Some(ContributorRole::Performer),
            Variable::Guest => Some(ContributorRole::Guest),
            Variable::Host => Some(ContributorRole::Unknown("host".to_string())),
            Variable::Narrator => Some(ContributorRole::Unknown("narrator".to_string())),
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
    pub(super) fn compile_date(&self, date: &crate::ir::DateBlock) -> Option<TemplateComponent> {
        let date_var = self.map_variable_to_date(&date.variable)?;

        let form = match &date.options.parts {
            Some(crate::ir::DateParts::Year) => DateForm::Year,
            Some(crate::ir::DateParts::YearMonth) => DateForm::YearMonth,
            _ => match &date.options.form {
                Some(crate::ir::DateForm::Numeric) => DateForm::Full,
                Some(crate::ir::DateForm::Text) => DateForm::Full,
                None => DateForm::Year,
            },
        };
        let fallback = matches!(&date_var, DateVariable::Issued).then(Vec::new);

        Some(TemplateComponent::Date(TemplateDate {
            date: date_var,
            form,
            fallback,
            rendering: convert_formatting(&date.formatting),
            ..Default::default()
        }))
    }

    /// Map a Variable to `DateVariable`.
    pub(super) fn map_variable_to_date(&self, var: &Variable) -> Option<DateVariable> {
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
    pub(super) fn compile_term(&self, term: &crate::ir::TermBlock) -> Option<TemplateComponent> {
        Some(TemplateComponent::Term(
            citum_schema::template::TemplateTerm {
                term: term.term.clone(),
                form: Some(term.form.clone()),
                rendering: convert_formatting(&term.formatting),
                ..Default::default()
            },
        ))
    }

    /// Compile a Variable block into the appropriate component.
    pub(super) fn compile_variable(
        &self,
        var: &crate::ir::VariableBlock,
    ) -> Option<TemplateComponent> {
        if let Variable::Identifier(name) = &var.variable {
            let identifier = citum_schema::reference::IdentifierName::new(name.clone()).ok()?;
            return Some(TemplateComponent::Identifier(
                citum_schema::template::TemplateIdentifier {
                    identifier,
                    rendering: convert_formatting(&var.formatting),
                },
            ));
        }

        // First, check if it's a contributor role
        if let Some(role) = self.map_variable_to_role(&var.variable) {
            return Some(TemplateComponent::Contributor(TemplateContributor {
                contributor: role.into(),
                form: ContributorForm::Long,
                name_order: None, // Use global setting by default
                delimiter: None,
                rendering: convert_formatting(&var.formatting),
                ..Default::default()
            }));
        }

        // Check if it's a title
        if let Some(title_type) = self.map_variable_to_title(&var.variable) {
            return Some(TemplateComponent::Title(TemplateTitle {
                title: title_type,
                form: None,
                rendering: convert_formatting(&var.formatting),
                ..Default::default()
            }));
        }

        // Check if it's a number
        if let Some(num_var) = self.map_variable_to_number(&var.variable) {
            let mut rendering = convert_formatting(&var.formatting);
            if let Some(label) = &var.label {
                rendering.strip_periods =
                    label.formatting.strip_periods.or(rendering.strip_periods);
            }

            // Extract label form if present
            let mut label_form = var.label.as_ref().map(|l| map_label_form(&l.form));
            let when_numeric = (num_var == NumberVariable::Edition)
                .then(|| label_form.take())
                .flatten();

            return Some(TemplateComponent::Number(TemplateNumber {
                number: num_var,
                form: var.number_form.clone(),
                label_form,
                when_numeric,
                rendering,
                ..Default::default()
            }));
        }

        // Check if it's a simple variable
        if let Some(simple_var) = self.map_variable_to_simple(&var.variable) {
            let mut rendering = convert_formatting(&var.formatting);

            if let Some(label) = &var.label
                && !matches!(simple_var, SimpleVariable::Locator)
            {
                // Locator labels are handled by style-level locators config
                rendering.strip_periods =
                    label.formatting.strip_periods.or(rendering.strip_periods);
            }

            return Some(TemplateComponent::Variable(TemplateVariable {
                variable: simple_var,
                rendering,
                ..Default::default()
            }));
        }

        None
    }

    /// Map a Variable to `TitleType`.
    pub(super) fn map_variable_to_title(&self, var: &Variable) -> Option<TitleType> {
        match var {
            Variable::Title => Some(TitleType::Primary),
            Variable::ContainerTitle => Some(TitleType::ContainerTitle),
            Variable::CollectionTitle => Some(TitleType::CollectionTitle),
            _ => None,
        }
    }

    /// Map a Variable to `NumberVariable`.
    pub(super) fn map_variable_to_number(&self, var: &Variable) -> Option<NumberVariable> {
        match var {
            Variable::Volume => Some(NumberVariable::Volume),
            Variable::Issue => Some(NumberVariable::Issue),
            Variable::Page => Some(NumberVariable::Pages),
            Variable::Edition => Some(NumberVariable::Edition),
            Variable::ChapterNumber => Some(NumberVariable::ChapterNumber),
            Variable::CollectionNumber => Some(NumberVariable::CollectionNumber),
            Variable::NumberOfPages => Some(NumberVariable::NumberOfPages),
            // Reference markers are processor-owned and declared via
            // `label-mode`, not emitted as components.
            // See docs/specs/REFERENCE_MARKERS.md.
            Variable::CitationNumber | Variable::CitationLabel => None,
            Variable::Number => Some(NumberVariable::Number),
            _ => None,
        }
    }

    /// Map a Variable to `SimpleVariable`.
    pub(super) fn map_variable_to_simple(&self, var: &Variable) -> Option<SimpleVariable> {
        match var {
            Variable::DOI => Some(SimpleVariable::Doi),
            Variable::ISBN => Some(SimpleVariable::Isbn),
            Variable::ISSN => Some(SimpleVariable::Issn),
            Variable::URL => Some(SimpleVariable::Url),
            Variable::Publisher => Some(SimpleVariable::Publisher),
            Variable::PublisherPlace => Some(SimpleVariable::PublisherPlace),
            Variable::Genre => Some(SimpleVariable::Genre),
            Variable::Authority => Some(SimpleVariable::Authority),
            Variable::Section => Some(SimpleVariable::Section),
            Variable::Archive => Some(SimpleVariable::Archive),
            Variable::ArchiveLocation => Some(SimpleVariable::ArchiveLocation),
            Variable::Version => Some(SimpleVariable::Version),
            Variable::Medium => Some(SimpleVariable::Medium),
            Variable::Source => Some(SimpleVariable::Source),
            Variable::Status => Some(SimpleVariable::Status),
            Variable::Locator => Some(SimpleVariable::Locator),
            Variable::PMID => Some(SimpleVariable::Pmid),
            Variable::PMCID => Some(SimpleVariable::Pmcid),
            Variable::Note => Some(SimpleVariable::Note),
            Variable::Annote => Some(SimpleVariable::Annote),
            Variable::Abstract => Some(SimpleVariable::Abstract),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DateBlock, DateOptions, FormattingOptions};

    #[test]
    fn issued_dates_use_an_authoritative_empty_fallback() {
        let component = TemplateCompiler.compile_date(&DateBlock {
            variable: Variable::Issued,
            options: DateOptions::default(),
            formatting: FormattingOptions::default(),
            source_order: None,
        });

        assert!(
            matches!(&component, Some(TemplateComponent::Date(_))),
            "issued date compilation must produce a date template component"
        );
        if let Some(TemplateComponent::Date(date)) = component {
            assert_eq!(date.fallback, Some(Vec::new()));
        }
    }

    #[test]
    fn non_issued_dates_keep_their_normal_missing_value_behavior() {
        let component = TemplateCompiler.compile_date(&DateBlock {
            variable: Variable::Accessed,
            options: DateOptions::default(),
            formatting: FormattingOptions::default(),
            source_order: None,
        });

        assert!(
            matches!(&component, Some(TemplateComponent::Date(_))),
            "accessed date compilation must produce a date template component"
        );
        if let Some(TemplateComponent::Date(date)) = component {
            assert_eq!(date.fallback, None);
        }
    }
}
