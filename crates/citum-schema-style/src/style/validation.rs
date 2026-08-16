/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style validation and resource-limit checks.

use crate::template::{
    LocalizedTemplateSpec, TemplateComponent, TemplateVariant, TemplateVariants,
};
use crate::version::{MAX_TEMPLATE_COMPONENTS, MAX_TEMPLATE_NESTING_DEPTH};
use crate::{BibliographySpec, CitationSpec, ResolutionError};

use super::Style;

#[cfg(test)]
use crate::template::TemplateGroup;

/// A non-fatal validation warning emitted by [`Style::validate`].
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaWarning {
    /// A `TypeSelector` references an unrecognized reference type name.
    ///
    /// This usually indicates a typo (e.g., `article_journal` instead of
    /// `article-journal`). The selector will silently match nothing at
    /// render time.
    UnknownTypeName {
        /// The unrecognized type name string.
        name: String,
        /// Human-readable location hint (e.g., `"bibliography.type-variants"`).
        location: String,
    },
}

impl std::fmt::Display for SchemaWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaWarning::UnknownTypeName { name, location } => {
                write!(
                    f,
                    "unknown reference type \"{name}\" in {location} \
                     (may not match a reference; check for typos)"
                )
            }
        }
    }
}

impl Style {
    /// Validate hard resource limits for style templates.
    ///
    /// # Errors
    ///
    /// Returns an error when authored template structure exceeds the maximum
    /// depth or component count accepted by the engine.
    pub fn validate_resource_limits(&self) -> Result<(), String> {
        let mut budget = TemplateResourceBudget::default();

        if let Some(substitute) = self
            .options
            .as_ref()
            .and_then(|options| options.substitute.as_ref())
        {
            validate_substitute_candidates(substitute, "options.substitute")?;
        }
        if let Some(date_substitute) = self
            .options
            .as_ref()
            .and_then(|options| options.date_substitute.as_ref())
        {
            budget.check_date_substitute(date_substitute, "options.date-substitute")?;
        }

        if let Some(templates) = &self.templates {
            for (name, template) in templates {
                budget.check_template(template, &format!("templates.{name}"), 0)?;
            }
        }
        if let Some(citation) = &self.citation {
            budget.check_citation_spec(citation, "citation", 0)?;
        }
        if let Some(bibliography) = &self.bibliography {
            budget.check_bibliography_spec(bibliography, "bibliography", 0)?;
        }

        Ok(())
    }

    /// Validate the style and return any non-fatal warnings.
    ///
    /// This method checks for issues that are syntactically valid but
    /// semantically suspect, such as unrecognized reference type names in
    /// selectors or title mappings.
    ///
    /// Warnings do not prevent rendering; they are informational only.
    pub fn validate(&self) -> Vec<SchemaWarning> {
        let mut warnings = Vec::new();
        self.collect_type_selector_warnings(&mut warnings);
        warnings
    }

    /// Collect warnings for all `TypeSelector` values in the style.
    fn collect_type_selector_warnings(&self, warnings: &mut Vec<SchemaWarning>) {
        if let Some(type_mapping) = self
            .options
            .as_ref()
            .and_then(|options| options.titles.as_ref())
            .and_then(|titles| titles.type_mapping.as_ref())
        {
            for reference_type in type_mapping.keys().filter(|name| !name.is_known()) {
                warnings.push(SchemaWarning::UnknownTypeName {
                    name: reference_type.to_string(),
                    location: "options.titles.type-mapping".to_string(),
                });
            }
        }
        if let Some(date_substitute) = self
            .options
            .as_ref()
            .and_then(|options| options.date_substitute.as_ref())
        {
            collect_date_substitute_warnings(date_substitute, "options.date-substitute", warnings);
        }
        if let Some(bib) = &self.bibliography
            && let Some(type_variants) = &bib.type_variants
        {
            for selector in type_variants.keys() {
                for name in selector.unknown_type_names() {
                    warnings.push(SchemaWarning::UnknownTypeName {
                        name: name.to_string(),
                        location: "bibliography.type-variants".to_string(),
                    });
                }
            }
        }
        if let Some(cit) = &self.citation {
            collect_citation_spec_warnings(cit, "citation", warnings);
        }
        if let Some(bib) = &self.bibliography
            && let Some(date_substitute) = bib
                .options
                .as_ref()
                .and_then(|options| options.date_substitute.as_ref())
        {
            collect_date_substitute_warnings(
                date_substitute,
                "bibliography.options.date-substitute",
                warnings,
            );
        }
    }

    pub(crate) fn validate_profile_shape(&self) -> Result<(), ResolutionError> {
        if self.templates.is_some() || yaml_path_present(self.raw_yaml.as_ref(), &["templates"]) {
            return Err(ResolutionError::InvalidProfileOverride {
                location: "templates".to_string(),
            });
        }

        if let Some(location) = forbidden_profile_template_path(self.raw_yaml.as_ref()) {
            return Err(ResolutionError::InvalidProfileOverride { location });
        }

        Ok(())
    }
}

fn validate_substitute_candidates(
    config: &crate::options::SubstituteConfig,
    location: &str,
) -> Result<(), String> {
    let resolved = config.resolve();
    validate_candidate_list(&resolved.template, &format!("{location}.template"))?;
    for (reference_type, candidates) in &resolved.overrides {
        validate_candidate_list(
            candidates.as_slice(),
            &format!("{location}.overrides.{reference_type}"),
        )?;
    }
    Ok(())
}

fn validate_candidate_list(
    candidates: &[crate::options::SubstituteKey],
    location: &str,
) -> Result<(), String> {
    for (index, candidate) in candidates.iter().enumerate() {
        let crate::options::SubstituteKey::Contributor(candidate) = candidate else {
            continue;
        };
        let crate::template::ContributorRoles::Multiple(roles) = &candidate.contributor else {
            continue;
        };
        if roles.len() < 2 {
            return Err(format!(
                "{location}[{index}].contributor must contain at least two roles in list form"
            ));
        }
        let distinct = roles.iter().collect::<std::collections::HashSet<_>>();
        if distinct.len() != roles.len() {
            return Err(format!(
                "{location}[{index}].contributor role list must not contain duplicates"
            ));
        }
    }
    Ok(())
}

fn forbidden_profile_template_path(raw_yaml: Option<&serde_yaml::Value>) -> Option<String> {
    let raw_yaml = raw_yaml?;
    for (section, recursive) in [("citation", true), ("bibliography", false)] {
        if let Some(section_value) = mapping_child(raw_yaml, section) {
            if recursive {
                if let Some(location) = forbidden_citation_template_path(section_value, section) {
                    return Some(location);
                }
            } else if let Some(location) = forbidden_section_template_path(section_value, section) {
                return Some(location);
            }
        }
    }
    None
}

fn forbidden_section_template_path(section: &serde_yaml::Value, location: &str) -> Option<String> {
    for key in ["template", "template-ref", "type-variants", "locales"] {
        if mapping_child(section, key).is_some() {
            return Some(format!("{location}.{key}"));
        }
    }
    None
}

fn forbidden_citation_template_path(section: &serde_yaml::Value, location: &str) -> Option<String> {
    if let Some(location) = forbidden_section_template_path(section, location) {
        return Some(location);
    }

    for sub_section in ["integral", "non-integral", "subsequent", "ibid"] {
        if let Some(child) = mapping_child(section, sub_section)
            && let Some(location) =
                forbidden_citation_template_path(child, &format!("{location}.{sub_section}"))
        {
            return Some(location);
        }
    }
    None
}

fn mapping_child<'a>(value: &'a serde_yaml::Value, segment: &str) -> Option<&'a serde_yaml::Value> {
    let serde_yaml::Value::Mapping(map) = value else {
        return None;
    };
    let key = serde_yaml::Value::String(segment.to_string());
    map.get(&key)
}

fn yaml_path_present(value: Option<&serde_yaml::Value>, path: &[&str]) -> bool {
    let Some(mut current) = value else {
        return false;
    };
    for segment in path {
        let Some(next) = mapping_child(current, segment) else {
            return false;
        };
        current = next;
    }
    true
}

/// Collect warnings from a `CitationSpec` and its sub-specs.
fn collect_citation_spec_warnings(
    spec: &CitationSpec,
    location: &str,
    warnings: &mut Vec<SchemaWarning>,
) {
    if let Some(date_substitute) = spec
        .options
        .as_ref()
        .and_then(|options| options.date_substitute.as_ref())
    {
        collect_date_substitute_warnings(
            date_substitute,
            &format!("{location}.options.date-substitute"),
            warnings,
        );
    }
    if let Some(type_variants) = &spec.type_variants {
        for selector in type_variants.keys() {
            for name in selector.unknown_type_names() {
                warnings.push(SchemaWarning::UnknownTypeName {
                    name: name.to_string(),
                    location: format!("{location}.type-variants"),
                });
            }
        }
    }
    // Recurse into sub-specs
    for (sub_name, sub_spec) in [
        ("integral", spec.integral.as_deref()),
        ("non-integral", spec.non_integral.as_deref()),
        ("subsequent", spec.subsequent.as_deref()),
        ("ibid", spec.ibid.as_deref()),
    ]
    .into_iter()
    .filter_map(|(n, s)| s.map(|s| (n, s)))
    {
        collect_citation_spec_warnings(sub_spec, &format!("{location}.{sub_name}"), warnings);
    }
}

fn collect_date_substitute_warnings(
    date_substitute: &crate::options::DateSubstitute,
    location: &str,
    warnings: &mut Vec<SchemaWarning>,
) {
    for selector in date_substitute.entries().keys() {
        for name in selector.unknown_type_names() {
            warnings.push(SchemaWarning::UnknownTypeName {
                name: name.to_string(),
                location: location.to_string(),
            });
        }
    }
}

#[derive(Default)]
struct TemplateResourceBudget {
    component_count: usize,
}

impl TemplateResourceBudget {
    fn check_date_substitute(
        &mut self,
        date_substitute: &crate::options::DateSubstitute,
        location: &str,
    ) -> Result<(), String> {
        for (selector, candidates) in date_substitute.entries() {
            for candidate in candidates {
                self.check_component(
                    &candidate.to_template_component(),
                    &format!("{location}.{selector:?}"),
                    0,
                )?;
            }
        }
        Ok(())
    }

    fn check_template(
        &mut self,
        template: &[TemplateComponent],
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_TEMPLATE_NESTING_DEPTH {
            return Err(format!(
                "{location} exceeds maximum template nesting depth of {MAX_TEMPLATE_NESTING_DEPTH}"
            ));
        }
        for component in template {
            self.check_component(component, location, depth)?;
        }
        Ok(())
    }

    fn check_component(
        &mut self,
        component: &TemplateComponent,
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        self.component_count = self.component_count.saturating_add(1);
        if self.component_count > MAX_TEMPLATE_COMPONENTS {
            return Err(format!(
                "style exceeds maximum template component count of {MAX_TEMPLATE_COMPONENTS}"
            ));
        }

        match component {
            TemplateComponent::Date(date) => {
                if let Some(fallback) = &date.fallback {
                    self.check_template(fallback, &format!("{location}.date.fallback"), depth + 1)?;
                }
            }
            TemplateComponent::Group(group) => {
                if let Some(cond) = &group.render_when {
                    match (&cond.field_present, &cond.field_absent) {
                        (None, None) => {
                            return Err(format!(
                                "{location}.group.render-when: must set field-present or field-absent"
                            ));
                        }
                        (Some(present), Some(absent)) if present == absent => {
                            return Err(format!(
                                "{location}.group.render-when: field-present and field-absent must not be the same field ({present:?})"
                            ));
                        }
                        _ => {}
                    }
                }
                self.check_template(&group.group, &format!("{location}.group"), depth + 1)?;
            }
            TemplateComponent::Message(message) => {
                for (name, source) in &message.args {
                    if let Some(component) = source.as_template_component() {
                        self.check_component(
                            &component,
                            &format!("{location}.message.args.{name}"),
                            depth + 1,
                        )?;
                    }
                }
            }
            TemplateComponent::Contributor(contributor) => match &contributor.contributor {
                crate::template::ContributorRoles::Single(_) => {
                    if contributor.merge.is_some() {
                        return Err(format!(
                            "{location}.merge is valid only for a contributor role list"
                        ));
                    }
                }
                crate::template::ContributorRoles::Multiple(roles) => {
                    if roles.len() < 2 {
                        return Err(format!(
                            "{location}.contributor must contain at least two roles in list form"
                        ));
                    }
                    let distinct = roles.iter().collect::<std::collections::HashSet<_>>();
                    if distinct.len() != roles.len() {
                        return Err(format!(
                            "{location}.contributor role list must not contain duplicates"
                        ));
                    }
                    if contributor.label.is_some() {
                        return Err(format!("{location}.label is valid only for a single role"));
                    }
                    if let Some(merge) = &contributor.merge
                        && let Some(role) = merge.roles.keys().find(|role| !roles.contains(role))
                    {
                        return Err(format!(
                            "{location}.merge.roles contains undeclared role {}",
                            role.as_str()
                        ));
                    }
                }
            },
            TemplateComponent::Title(_)
            | TemplateComponent::Number(_)
            | TemplateComponent::Identifier(_)
            | TemplateComponent::Variable(_)
            | TemplateComponent::Term(_)
            | TemplateComponent::TypeLabel(_) => {}
        }

        Ok(())
    }

    fn check_variant(
        &mut self,
        variant: &TemplateVariant,
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        match variant {
            TemplateVariant::Full(template) => self.check_template(template, location, depth),
            TemplateVariant::Diff(diff) => {
                for (index, add) in diff.add.iter().enumerate() {
                    self.check_component(
                        &add.component,
                        &format!("{location}.add[{index}].component"),
                        depth,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn check_variants(
        &mut self,
        variants: &TemplateVariants,
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        for (selector, variant) in variants {
            self.check_variant(variant, &format!("{location}.{selector:?}"), depth)?;
        }
        Ok(())
    }

    fn check_locales(
        &mut self,
        locales: &[LocalizedTemplateSpec],
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        for (index, locale) in locales.iter().enumerate() {
            self.check_template(
                &locale.template,
                &format!("{location}[{index}].template"),
                depth,
            )?;
            if let Some(variants) = &locale.type_variants {
                for (selector, template) in variants {
                    self.check_template(
                        template,
                        &format!("{location}[{index}].type-variants.{selector:?}"),
                        depth,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn check_citation_spec(
        &mut self,
        spec: &CitationSpec,
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        if let Some(substitute) = spec
            .options
            .as_ref()
            .and_then(|options| options.substitute.as_ref())
        {
            validate_substitute_candidates(substitute, &format!("{location}.options.substitute"))?;
        }
        if let Some(date_substitute) = spec
            .options
            .as_ref()
            .and_then(|options| options.date_substitute.as_ref())
        {
            self.check_date_substitute(
                date_substitute,
                &format!("{location}.options.date-substitute"),
            )?;
        }
        if let Some(template) = &spec.template {
            self.check_variant(template, &format!("{location}.template"), depth)?;
        }
        if let Some(locales) = &spec.locales {
            self.check_locales(locales, &format!("{location}.locales"), depth)?;
        }
        if let Some(variants) = &spec.type_variants {
            self.check_variants(variants, &format!("{location}.type-variants"), depth)?;
        }
        for (sub_name, sub_spec) in [
            ("integral", spec.integral.as_deref()),
            ("non-integral", spec.non_integral.as_deref()),
            ("subsequent", spec.subsequent.as_deref()),
            ("ibid", spec.ibid.as_deref()),
        ]
        .into_iter()
        .filter_map(|(n, s)| s.map(|s| (n, s)))
        {
            self.check_citation_spec(sub_spec, &format!("{location}.{sub_name}"), depth + 1)?;
        }
        Ok(())
    }

    fn check_bibliography_spec(
        &mut self,
        spec: &BibliographySpec,
        location: &str,
        depth: usize,
    ) -> Result<(), String> {
        if let Some(substitute) = spec
            .options
            .as_ref()
            .and_then(|options| options.substitute.as_ref())
        {
            validate_substitute_candidates(substitute, &format!("{location}.options.substitute"))?;
        }
        if let Some(date_substitute) = spec
            .options
            .as_ref()
            .and_then(|options| options.date_substitute.as_ref())
        {
            self.check_date_substitute(
                date_substitute,
                &format!("{location}.options.date-substitute"),
            )?;
        }
        if let Some(template) = &spec.template {
            self.check_variant(template, &format!("{location}.template"), depth)?;
        }
        if let Some(locales) = &spec.locales {
            self.check_locales(locales, &format!("{location}.locales"), depth)?;
        }
        if let Some(variants) = &spec.type_variants {
            self.check_variants(variants, &format!("{location}.type-variants"), depth)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod security_resource_tests {
    use super::*;
    use crate::locale::TermForm;
    use crate::options::{
        BibliographyOptions, CitationOptions, Config, DateSubstitute, DateSubstituteCandidate,
        DateSubstituteMessage,
    };
    use crate::template::{Rendering, TypeSelector};
    use indexmap::IndexMap;

    fn nested_group(depth: usize) -> TemplateComponent {
        if depth == 0 {
            TemplateComponent::default()
        } else {
            TemplateComponent::Group(TemplateGroup {
                group: vec![nested_group(depth - 1)],
                ..TemplateGroup::default()
            })
        }
    }

    fn date_substitute_with_candidates(count: usize) -> DateSubstitute {
        let candidate = DateSubstituteCandidate::Message(DateSubstituteMessage {
            message: "term.no-date".to_string(),
            form: Some(TermForm::Short),
            rendering: Rendering::default(),
        });
        DateSubstitute::new(IndexMap::from([(
            TypeSelector::Single("default".to_string()),
            vec![candidate; count],
        )]))
    }

    #[test]
    fn validate_resource_limits_rejects_deeply_nested_templates() {
        let style = Style {
            bibliography: Some(BibliographySpec {
                template: Some(vec![nested_group(MAX_TEMPLATE_NESTING_DEPTH + 1)].into()),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let err = style
            .validate_resource_limits()
            .expect_err("deep template must be rejected");

        assert!(err.contains("maximum template nesting depth"));
    }

    #[test]
    fn validate_resource_limits_rejects_too_many_components() {
        let style = Style {
            bibliography: Some(BibliographySpec {
                template: Some(
                    vec![TemplateComponent::default(); MAX_TEMPLATE_COMPONENTS + 1].into(),
                ),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let err = style
            .validate_resource_limits()
            .expect_err("oversized template must be rejected");

        assert!(err.contains("maximum template component count"));
    }

    #[test]
    fn validate_resource_limits_counts_date_substitute_candidates_across_scopes() {
        let global_count = MAX_TEMPLATE_COMPONENTS / 3;
        let citation_count = MAX_TEMPLATE_COMPONENTS / 3;
        let bibliography_count = MAX_TEMPLATE_COMPONENTS - global_count - citation_count + 1;
        let style = Style {
            options: Some(Config {
                date_substitute: Some(date_substitute_with_candidates(global_count)),
                ..Config::default()
            }),
            citation: Some(CitationSpec {
                options: Some(CitationOptions {
                    date_substitute: Some(date_substitute_with_candidates(citation_count)),
                    ..CitationOptions::default()
                }),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                options: Some(BibliographyOptions {
                    date_substitute: Some(date_substitute_with_candidates(bibliography_count)),
                    ..BibliographyOptions::default()
                }),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let err = style
            .validate_resource_limits()
            .expect_err("date-substitute candidates must share the template budget");

        assert!(err.contains("maximum template component count"));
    }
}
