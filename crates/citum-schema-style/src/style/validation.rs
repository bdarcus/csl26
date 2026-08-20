/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style validation and resource-limit checks.

use crate::options::processing::RegimeFamily;
use crate::template::{
    LocalizedTemplateSpec, TemplateComponent, TemplateVariant, TemplateVariants,
};
use crate::version::{MAX_TEMPLATE_COMPONENTS, MAX_TEMPLATE_NESTING_DEPTH};
use crate::{BibliographySpec, CitationCollapse, CitationSpec, ResolutionError};

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
        if let Some(date_fallback) = self
            .options
            .as_ref()
            .and_then(|options| options.date_fallback.as_ref())
        {
            budget.check_date_fallback(date_fallback, "options.date-fallback")?;
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

    /// Validate that any declared `citation.collapse` (including on
    /// `integral`/`non-integral`/`subsequent`/`ibid` sub-specs) is licensed
    /// for the style's resolved processing regime.
    ///
    /// `same-author` is legal on `AuthorDate`-family, `Note`, and `Custom`;
    /// `citation-number` is legal on `Numeric` and `Custom` — mirroring the
    /// engine's own existing gate, `should_collapse_citation_numbers`, which
    /// already requires `Processing::Numeric`. Absent `options.processing`
    /// resolves to `Processing::default()` (`AuthorDate`), matching the
    /// schema default. See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §6.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError::IncoherentCollapseRegime`] for the first
    /// sub-spec whose declared `collapse` value is not licensed.
    pub(crate) fn validate_collapse_regime(&self) -> Result<(), ResolutionError> {
        let regime = self
            .options
            .as_ref()
            .and_then(|options| options.processing.as_ref())
            .map_or(
                RegimeFamily::AuthorDate,
                crate::options::Processing::regime_family,
            );
        if let Some(citation) = &self.citation {
            check_collapse_regime(citation, "citation", regime)?;
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
        if let Some(date_fallback) = self
            .options
            .as_ref()
            .and_then(|options| options.date_fallback.as_ref())
        {
            collect_date_fallback_warnings(date_fallback, "options.date-fallback", warnings);
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
            && let Some(date_fallback) = bib
                .options
                .as_ref()
                .and_then(|options| options.date_fallback.as_ref())
        {
            collect_date_fallback_warnings(
                date_fallback,
                "bibliography.options.date-fallback",
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
    validate_candidate_list(resolved.candidates(), &format!("{location}.candidates"))?;
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
    if let Some(date_fallback) = spec
        .options
        .as_ref()
        .and_then(|options| options.date_fallback.as_ref())
    {
        collect_date_fallback_warnings(
            date_fallback,
            &format!("{location}.options.date-fallback"),
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

/// Human-readable name for a [`CitationCollapse`] value, for error messages.
fn collapse_label(collapse: &CitationCollapse) -> &'static str {
    match collapse {
        CitationCollapse::CitationNumber => "citation-number",
        CitationCollapse::SameAuthor(_) => "same-author",
    }
}

/// Human-readable name for a [`RegimeFamily`], for error messages.
fn regime_label(regime: RegimeFamily) -> &'static str {
    match regime {
        RegimeFamily::AuthorDate => "author-date",
        RegimeFamily::Numeric => "numeric",
        RegimeFamily::Note => "note",
        RegimeFamily::Label => "label",
        RegimeFamily::Custom => "custom",
    }
}

/// Recursively check `collapse` on a `CitationSpec` and its sub-specs against
/// the style's resolved processing regime.
fn check_collapse_regime(
    spec: &CitationSpec,
    location: &str,
    regime: RegimeFamily,
) -> Result<(), ResolutionError> {
    if let Some(collapse) = &spec.collapse {
        let licensed = match collapse {
            CitationCollapse::SameAuthor(_) => matches!(
                regime,
                RegimeFamily::AuthorDate | RegimeFamily::Note | RegimeFamily::Custom
            ),
            CitationCollapse::CitationNumber => {
                matches!(regime, RegimeFamily::Numeric | RegimeFamily::Custom)
            }
        };
        if !licensed {
            return Err(ResolutionError::IncoherentCollapseRegime {
                location: format!("{location}.collapse"),
                collapse: collapse_label(collapse).to_string(),
                processing: regime_label(regime).to_string(),
            });
        }
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
        check_collapse_regime(sub_spec, &format!("{location}.{sub_name}"), regime)?;
    }
    Ok(())
}

fn collect_date_fallback_warnings(
    date_fallback: &crate::options::DateFallbackConfig,
    location: &str,
    warnings: &mut Vec<SchemaWarning>,
) {
    let crate::options::DateFallbackConfig::Policy(policy) = date_fallback else {
        return;
    };
    for (lane_name, lane) in [
        ("first-issued", policy.first_issued.as_ref()),
        ("later-issued", policy.later_issued.as_ref()),
    ] {
        let Some(crate::options::DateFallbackLane::Selectors(selectors)) = lane else {
            continue;
        };
        for selector in selectors.entries().keys() {
            for name in selector.unknown_type_names() {
                warnings.push(SchemaWarning::UnknownTypeName {
                    name: name.to_string(),
                    location: format!("{location}.{lane_name}"),
                });
            }
        }
    }
}

#[derive(Default)]
struct TemplateResourceBudget {
    component_count: usize,
}

impl TemplateResourceBudget {
    fn check_date_fallback(
        &mut self,
        date_fallback: &crate::options::DateFallbackConfig,
        location: &str,
    ) -> Result<(), String> {
        let crate::options::DateFallbackConfig::Policy(policy) = date_fallback else {
            return Ok(());
        };
        for (lane_name, lane) in [
            ("first-issued", policy.first_issued.as_ref()),
            ("later-issued", policy.later_issued.as_ref()),
        ] {
            let Some(crate::options::DateFallbackLane::Selectors(selectors)) = lane else {
                continue;
            };
            for (selector, rule) in selectors.entries() {
                let Some(candidates) = rule.candidates() else {
                    continue;
                };
                for candidate in candidates.iter() {
                    if matches!(
                        candidate,
                        crate::options::DateFallbackCandidate::Date(candidate)
                            if matches!(candidate.date, crate::template::DateVariable::Issued)
                    ) {
                        return Err(format!(
                            "{location}.{lane_name}.{selector:?}: date-fallback candidates must not reference `issued`"
                        ));
                    }
                    self.check_component(
                        &candidate.to_template_component(),
                        &format!("{location}.{lane_name}.{selector:?}"),
                        0,
                    )?;
                }
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
            TemplateComponent::Date(_)
            | TemplateComponent::Title(_)
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
        if let Some(date_fallback) = spec
            .options
            .as_ref()
            .and_then(|options| options.date_fallback.as_ref())
        {
            self.check_date_fallback(date_fallback, &format!("{location}.options.date-fallback"))?;
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
        if let Some(date_fallback) = spec
            .options
            .as_ref()
            .and_then(|options| options.date_fallback.as_ref())
        {
            self.check_date_fallback(date_fallback, &format!("{location}.options.date-fallback"))?;
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
        BibliographyOptions, CitationOptions, Config, DateFallback, DateFallbackCandidate,
        DateFallbackConfig, DateFallbackDate, DateFallbackLane, DateFallbackMessage,
        DateFallbackRule, DateFallbackSelectorMap,
    };
    use crate::template::{DateForm, DateVariable, Rendering, TypeSelector};
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

    fn date_fallback_with_candidates(count: usize) -> DateFallbackConfig {
        let candidate = DateFallbackCandidate::Message(DateFallbackMessage {
            message: "term.no-date".to_string(),
            form: Some(TermForm::Short),
            rendering: Rendering::default(),
        });
        DateFallbackConfig::Policy(DateFallback {
            first_issued: Some(DateFallbackLane::Selectors(DateFallbackSelectorMap::new(
                IndexMap::from([(
                    TypeSelector::Single("default".to_string()),
                    DateFallbackRule::Candidates(vec![candidate; count]),
                )]),
            ))),
            later_issued: None,
        })
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
    fn validate_resource_limits_counts_date_fallback_candidates_across_scopes() {
        let global_count = MAX_TEMPLATE_COMPONENTS / 3;
        let citation_count = MAX_TEMPLATE_COMPONENTS / 3;
        let bibliography_count = MAX_TEMPLATE_COMPONENTS - global_count - citation_count + 1;
        let style = Style {
            options: Some(Config {
                date_fallback: Some(date_fallback_with_candidates(global_count)),
                ..Config::default()
            }),
            citation: Some(CitationSpec {
                options: Some(CitationOptions {
                    date_fallback: Some(date_fallback_with_candidates(citation_count)),
                    ..CitationOptions::default()
                }),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                options: Some(BibliographyOptions {
                    date_fallback: Some(date_fallback_with_candidates(bibliography_count)),
                    ..BibliographyOptions::default()
                }),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let err = style
            .validate_resource_limits()
            .expect_err("date-fallback candidates must share the template budget");

        assert!(err.contains("maximum template component count"));
    }

    #[test]
    fn validate_resource_limits_rejects_issued_date_fallback_candidates() {
        let candidate = DateFallbackCandidate::Date(DateFallbackDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            suppress_note: None,
            rendering: Rendering::default(),
        });
        let style = Style {
            options: Some(Config {
                date_fallback: Some(DateFallbackConfig::Policy(DateFallback {
                    first_issued: Some(DateFallbackLane::Selectors(DateFallbackSelectorMap::new(
                        IndexMap::from([(
                            TypeSelector::Single("default".to_string()),
                            DateFallbackRule::Candidates(vec![candidate]),
                        )]),
                    ))),
                    later_issued: None,
                })),
                ..Config::default()
            }),
            ..Style::default()
        };

        let error = style
            .validate_resource_limits()
            .expect_err("issued must not recursively fall back to itself");

        assert_eq!(
            error,
            "options.date-fallback.first-issued.Single(\"default\"): date-fallback candidates must not reference `issued`"
        );
    }
}
