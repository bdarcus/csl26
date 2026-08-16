/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Lint helpers for raw locales and styles.

use crate::citation::LocatorType;
use crate::locale::{GeneralTerm, Locale, RawLocale, TermForm, types::MessageSyntax};
use crate::options::Config;
use crate::template::{
    ContributorForm, ContributorRole, LabelForm as TemplateLabelForm, MessageArgSource,
    NumberVariable, RoleLabelForm, TemplateComponent, TemplateContributor, TemplateMessage,
};
use crate::{CitationSpec, Style, TemplateVariant};
use std::collections::BTreeSet;

/// A single lint finding produced by locale or style validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFinding {
    /// The severity of the finding.
    pub severity: LintSeverity,
    /// The dotted path that identifies the lint target.
    pub path: String,
    /// The human-readable diagnostic message.
    pub message: String,
}

/// The severity of a lint finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    /// A non-fatal issue that should be reviewed.
    Warning,
    /// A fatal issue that makes the locale invalid.
    Error,
}

/// A lint report containing all findings emitted for a locale or style.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LintReport {
    /// All findings collected during linting.
    pub findings: Vec<LintFinding>,
}

impl LintReport {
    fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.findings.push(LintFinding {
            severity: LintSeverity::Warning,
            path: path.into(),
            message: message.into(),
        });
    }

    fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.findings.push(LintFinding {
            severity: LintSeverity::Error,
            path: path.into(),
            message: message.into(),
        });
    }

    /// Return true when this report contains at least one error finding.
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == LintSeverity::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocaleRequirementKind {
    General {
        term: GeneralTerm,
        form: TermForm,
    },
    Role {
        role: ContributorRole,
        form: TermForm,
    },
    Locator {
        locator: LocatorType,
        form: TermForm,
    },
    Message {
        message_id: String,
        args: Vec<String>,
        form: Option<TermForm>,
        gender: Option<crate::locale::GrammaticalGender>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocaleRequirement {
    path: String,
    kind: LocaleRequirementKind,
}

/// Validate a raw locale's message syntax, alias targets, and completeness.
///
/// MF2 messages are checked for placeholder syntax, selector arity, and
/// wildcard coverage. Legacy aliases must point to an existing message key.
/// A v2 locale missing `grammar-options` or `date-formats` produces a
/// warning, since both silently fall back to English defaults otherwise.
pub fn lint_raw_locale(raw: &RawLocale) -> LintReport {
    let mut report = LintReport::default();
    let uses_mf2 = raw
        .evaluation
        .as_ref()
        .is_some_and(|config| config.message_syntax == MessageSyntax::Mf2);

    for (message_id, message) in &raw.messages {
        if uses_mf2
            && (message.contains('{') || message.contains(".match"))
            && let Err(err) = lint_mf2_message(message)
        {
            report.error(
                format!("messages.{message_id}"),
                format!("invalid MF2 message: {err}"),
            );
        }
    }

    for (legacy_key, message_id) in &raw.legacy_term_aliases {
        if !raw.messages.contains_key(message_id) {
            report.error(
                format!("legacy-term-aliases.{legacy_key}"),
                format!("target '{message_id}' does not exist in messages"),
            );
        }
    }

    let is_v2 = raw.locale_schema_version.as_deref() == Some("2");
    if is_v2 && raw.grammar_options.is_none() {
        report.warning(
            "grammar-options",
            "v2 locale has no grammar-options; typography (quotes, delimiters, \
             page-range dashes) silently falls back to the engine's English defaults",
        );
    }
    if is_v2 && raw.date_formats.is_empty() {
        report.warning(
            "date-formats",
            "v2 locale has no date-formats; date assembly silently falls back to \
             the engine's hardcoded English patterns",
        );
    }

    report
}

/// Validate a style's locale-dependent templates against a locale.
///
/// Missing term, role, or locator resolutions are reported as warnings.
pub fn lint_style_against_locale(style: &Style, locale: &Locale) -> LintReport {
    let mut report = LintReport::default();
    let requirements = collect_style_locale_requirements(style);

    for requirement in requirements {
        match requirement.kind {
            LocaleRequirementKind::General { term, form } => {
                if locale.resolved_general_term(&term, &form, None).is_none() {
                    report.warning(
                        requirement.path,
                        format!(
                            "locale does not resolve general term '{term:?}' in form '{form:?}'"
                        ),
                    );
                }
            }
            LocaleRequirementKind::Role { role, form } => {
                let singular = locale.resolved_role_term(&role, false, &form, None);
                let plural = locale.resolved_role_term(&role, true, &form, None);
                if singular.is_none() || plural.is_none() {
                    report.warning(
                        requirement.path,
                        format!(
                            "locale does not fully resolve role term '{role:?}' in form '{form:?}'"
                        ),
                    );
                }
            }
            LocaleRequirementKind::Locator { locator, form } => {
                let singular = lint_locator_term(locale, &locator, false, form.clone());
                let plural = lint_locator_term(locale, &locator, true, form.clone());
                if singular.is_none() || plural.is_none() {
                    report.warning(
                        requirement.path,
                        format!(
                            "locale does not fully resolve locator term '{locator:?}' in form '{form:?}'"
                        ),
                    );
                }
            }
            LocaleRequirementKind::Message {
                message_id,
                args,
                form,
                gender,
            } => {
                let Some(message) = locale.messages.get(&message_id) else {
                    if locale
                        .resolve_template_message(
                            &message_id,
                            &crate::locale::MessageArgs::default(),
                            form.as_ref(),
                            gender,
                        )
                        .is_none()
                    {
                        report.error(
                            requirement.path,
                            format!("locale message '{message_id}' does not exist"),
                        );
                    }
                    continue;
                };

                for variable in mf2_variables(message) {
                    if !args.iter().any(|arg| arg == &variable) {
                        report.error(
                            requirement.path.clone(),
                            format!(
                                "locale message '{message_id}' references '${variable}' but the template message does not declare that arg"
                            ),
                        );
                    }
                }
            }
        }
    }

    report
}

fn lint_locator_term(
    locale: &Locale,
    locator: &LocatorType,
    plural: bool,
    form: TermForm,
) -> Option<String> {
    match locator {
        LocatorType::Custom(_) => locale
            .locators
            .get(locator)
            .and_then(|term| match form {
                TermForm::Long => term.long.as_ref(),
                TermForm::Short => term.short.as_ref(),
                TermForm::Symbol => term.symbol.as_ref(),
                _ => term.short.as_ref(),
            })
            .or_else(|| {
                locale.locators.get(locator).and_then(|term| {
                    term.long
                        .as_ref()
                        .or(term.short.as_ref())
                        .or(term.symbol.as_ref())
                })
            })
            .map(|forms| {
                if plural {
                    forms.plural.as_str().to_string()
                } else {
                    forms.singular.as_str().to_string()
                }
            }),
        _ => locale.resolved_locator_term(locator, plural, &form, None),
    }
}

fn collect_style_locale_requirements(style: &Style) -> Vec<LocaleRequirement> {
    let mut requirements = Vec::new();
    let base_config = style.options.clone().unwrap_or_default();
    collect_option_locale_requirements(&base_config, "options", &mut requirements);

    if let Some(template) = &style.templates {
        for (name, components) in template {
            collect_template_requirements(
                components,
                &format!("templates.{name}"),
                &base_config,
                &mut requirements,
            );
        }
    }

    if let Some(citation) = &style.citation {
        collect_citation_spec_requirements(citation, "citation", &base_config, &mut requirements);
    }

    if let Some(bibliography) = &style.bibliography {
        collect_bibliography_spec_requirements(
            bibliography,
            "bibliography",
            &base_config,
            &mut requirements,
        );
    }

    requirements
}

fn collect_citation_spec_requirements(
    spec: &CitationSpec,
    path: &str,
    base_config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    let effective_config = spec.options.as_ref().map_or_else(
        || base_config.clone(),
        |options| options.merged_with(base_config),
    );
    collect_option_locale_requirements(&effective_config, &format!("{path}.options"), requirements);

    if let Some(template) = spec.resolve_template() {
        collect_template_requirements(
            &template,
            &format!("{path}.template"),
            &effective_config,
            requirements,
        );
    }
    if let Some(locales) = &spec.locales {
        for (index, localized) in locales.iter().enumerate() {
            collect_template_requirements(
                &localized.template,
                &format!("{path}.locales[{index}].template"),
                &effective_config,
                requirements,
            );
        }
    }
    if let Some(spec) = &spec.integral {
        collect_citation_spec_requirements(
            spec,
            &format!("{path}.integral"),
            &effective_config,
            requirements,
        );
    }
    if let Some(spec) = &spec.non_integral {
        collect_citation_spec_requirements(
            spec,
            &format!("{path}.non-integral"),
            &effective_config,
            requirements,
        );
    }
    if let Some(spec) = &spec.subsequent {
        collect_citation_spec_requirements(
            spec,
            &format!("{path}.subsequent"),
            &effective_config,
            requirements,
        );
    }
    if let Some(spec) = &spec.ibid {
        collect_citation_spec_requirements(
            spec,
            &format!("{path}.ibid"),
            &effective_config,
            requirements,
        );
    }
}

fn collect_bibliography_spec_requirements(
    spec: &crate::BibliographySpec,
    path: &str,
    base_config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    let effective_config = spec.options.as_ref().map_or_else(
        || base_config.clone(),
        |options| options.merged_with(base_config),
    );
    collect_option_locale_requirements(&effective_config, &format!("{path}.options"), requirements);

    if let Some(template) = spec.resolve_template() {
        collect_template_requirements(
            &template,
            &format!("{path}.template"),
            &effective_config,
            requirements,
        );
    }
    if let Some(locales) = &spec.locales {
        for (index, localized) in locales.iter().enumerate() {
            collect_template_requirements(
                &localized.template,
                &format!("{path}.locales[{index}].template"),
                &effective_config,
                requirements,
            );
        }
    }
    if let Some(type_variants) = &spec.type_variants {
        for (selector, variant) in type_variants {
            let variant_path = format!("{path}.type-variants[{selector:?}]");
            match variant {
                TemplateVariant::Full(template) => {
                    collect_template_requirements(
                        template,
                        &variant_path,
                        &effective_config,
                        requirements,
                    );
                }
                TemplateVariant::Diff(diff) => {
                    for (index, operation) in diff.add.iter().enumerate() {
                        collect_template_requirements(
                            std::slice::from_ref(&operation.component),
                            &format!("{variant_path}.add[{index}].component"),
                            &effective_config,
                            requirements,
                        );
                    }
                }
            }
        }
    }
    if let Some(groups) = &spec.groups {
        for (index, group) in groups.iter().enumerate() {
            if let Some(heading) = &group.heading
                && let crate::grouping::GroupHeading::Term { term, form } = heading
            {
                requirements.push(LocaleRequirement {
                    path: format!("{path}.groups[{index}].heading"),
                    kind: LocaleRequirementKind::General {
                        term: term.clone(),
                        form: form.clone().unwrap_or(TermForm::Long),
                    },
                });
            }
            if let Some(template) = &group.template {
                collect_template_requirements(
                    template,
                    &format!("{path}.groups[{index}].template"),
                    &effective_config,
                    requirements,
                );
            }
        }
    }
}

fn collect_template_requirements(
    template: &[TemplateComponent],
    path: &str,
    config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    for (index, component) in template.iter().enumerate() {
        let component_path = format!("{path}[{index}]");
        match component {
            TemplateComponent::Term(term) => {
                requirements.push(LocaleRequirement {
                    path: component_path,
                    kind: LocaleRequirementKind::General {
                        term: term.term.clone(),
                        form: term.form.clone().unwrap_or(TermForm::Long),
                    },
                });
            }
            TemplateComponent::Message(message) => {
                collect_message_requirements(message, &component_path, config, requirements);
            }
            TemplateComponent::Contributor(contributor) => {
                collect_contributor_requirements(
                    contributor,
                    &component_path,
                    config,
                    requirements,
                );
            }
            TemplateComponent::Number(number) => {
                if let Some(form) = number.label_form.clone()
                    && let Some(locator) = number_variable_to_locator(number.number.clone())
                {
                    let term_form = match form {
                        TemplateLabelForm::Short => TermForm::Short,
                        TemplateLabelForm::Long => TermForm::Long,
                        TemplateLabelForm::Symbol => TermForm::Symbol,
                    };
                    requirements.push(LocaleRequirement {
                        path: component_path,
                        kind: LocaleRequirementKind::Locator {
                            locator,
                            form: term_form,
                        },
                    });
                }
            }
            TemplateComponent::Date(_) => {}
            TemplateComponent::Group(list) => {
                collect_template_requirements(
                    &list.group,
                    &format!("{component_path}.items"),
                    config,
                    requirements,
                );
            }
            _ => {}
        }
    }
}

fn collect_option_locale_requirements(
    config: &Config,
    path: &str,
    requirements: &mut Vec<LocaleRequirement>,
) {
    if let Some(message) = config.effective_substitute().otherwise_message() {
        collect_message_requirements(
            &message.to_template_message(),
            &format!("{path}.substitute.otherwise"),
            config,
            requirements,
        );
    }

    let Some(crate::options::DateFallbackConfig::Policy(policy)) = &config.date_fallback else {
        return;
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
            for (index, candidate) in candidates.iter().enumerate() {
                let crate::options::DateFallbackCandidate::Message(message) = candidate else {
                    continue;
                };
                collect_message_requirements(
                    &TemplateMessage {
                        message: message.message.clone(),
                        form: message.form.clone(),
                        rendering: message.rendering.clone(),
                        ..TemplateMessage::default()
                    },
                    &format!("{path}.date-fallback.{lane_name}.{selector}[{index}]"),
                    config,
                    requirements,
                );
            }
        }
    }
}

fn collect_message_requirements(
    message: &TemplateMessage,
    path: &str,
    config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    requirements.push(LocaleRequirement {
        path: path.to_string(),
        kind: LocaleRequirementKind::Message {
            message_id: message.message.clone(),
            args: message.args.keys().cloned().collect(),
            form: message.form.clone(),
            gender: message.gender.clone(),
        },
    });

    for (arg_name, source) in &message.args {
        collect_message_arg_requirements(
            source,
            &format!("{path}.args.{arg_name}"),
            config,
            requirements,
        );
    }
}

fn collect_message_arg_requirements(
    source: &MessageArgSource,
    path: &str,
    config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    if let Some(component) = source.as_template_component() {
        collect_template_requirements(std::slice::from_ref(&component), path, config, requirements);
    }
}

fn collect_contributor_requirements(
    contributor: &TemplateContributor,
    path: &str,
    config: &Config,
    requirements: &mut Vec<LocaleRequirement>,
) {
    let roles = contributor.contributor.as_slice();
    let primary_role = roles.first().cloned().unwrap_or_default();

    if let Some(label) = &contributor.label {
        let role = role_label_term_to_role(&label.term).unwrap_or(primary_role);
        let form = match label.form {
            RoleLabelForm::Short => TermForm::Short,
            RoleLabelForm::Long => TermForm::Long,
        };
        requirements.push(LocaleRequirement {
            path: format!("{path}.label"),
            kind: LocaleRequirementKind::Role { role, form },
        });
        return;
    }

    let configured_preset = config
        .contributors
        .as_ref()
        .and_then(|contributors| contributors.effective_role_label_preset(&primary_role));
    if let Some(role_label_preset) = configured_preset {
        let form = match role_label_preset {
            crate::options::RoleLabelPreset::None => return,
            crate::options::RoleLabelPreset::VerbPrefix => TermForm::Verb,
            crate::options::RoleLabelPreset::VerbShortPrefix => TermForm::VerbShort,
            crate::options::RoleLabelPreset::ShortSuffix
            | crate::options::RoleLabelPreset::ShortSuffixComma => TermForm::Short,
            crate::options::RoleLabelPreset::LongSuffix => TermForm::Long,
        };
        for role in roles {
            requirements.push(LocaleRequirement {
                path: path.to_string(),
                kind: LocaleRequirementKind::Role {
                    role: role.clone(),
                    form: form.clone(),
                },
            });
        }
        return;
    }

    match contributor.form {
        ContributorForm::Verb => {
            collect_role_requirements(roles, path, TermForm::Verb, requirements);
        }
        ContributorForm::VerbShort => {
            collect_role_requirements(roles, path, TermForm::VerbShort, requirements);
        }
        ContributorForm::Long => {
            for role in roles.iter().filter(|role| {
                matches!(role, ContributorRole::Editor | ContributorRole::Translator)
            }) {
                requirements.push(LocaleRequirement {
                    path: path.to_string(),
                    kind: LocaleRequirementKind::Role {
                        role: role.clone(),
                        form: TermForm::Short,
                    },
                });
            }
        }
        _ => {}
    }
}

fn collect_role_requirements(
    roles: &[ContributorRole],
    path: &str,
    form: TermForm,
    requirements: &mut Vec<LocaleRequirement>,
) {
    for role in roles {
        requirements.push(LocaleRequirement {
            path: path.to_string(),
            kind: LocaleRequirementKind::Role {
                role: role.clone(),
                form: form.clone(),
            },
        });
    }
}

fn role_label_term_to_role(term: &str) -> Option<ContributorRole> {
    match term {
        "editor" => Some(ContributorRole::Editor),
        "translator" => Some(ContributorRole::Translator),
        "director" => Some(ContributorRole::Director),
        "recipient" => Some(ContributorRole::Recipient),
        "interviewer" => Some(ContributorRole::Interviewer),
        _ => None,
    }
}

fn number_variable_to_locator(number: NumberVariable) -> Option<LocatorType> {
    match number {
        NumberVariable::Volume | NumberVariable::NumberOfVolumes => Some(LocatorType::Volume),
        NumberVariable::Pages | NumberVariable::NumberOfPages => Some(LocatorType::Page),
        NumberVariable::ChapterNumber => Some(LocatorType::Chapter),
        NumberVariable::Issue => Some(LocatorType::Issue),
        NumberVariable::Number
        | NumberVariable::DocketNumber
        | NumberVariable::PatentNumber
        | NumberVariable::StandardNumber
        | NumberVariable::ReportNumber
        | NumberVariable::PrintingNumber => Some(LocatorType::Number),
        NumberVariable::PartNumber => Some(LocatorType::Part),
        NumberVariable::SupplementNumber => Some(LocatorType::Supplement),
        NumberVariable::Custom(kind) => Some(LocatorType::Custom(kind)),
        _ => None,
    }
}

/// Validate an MF2 message string for structural correctness.
///
/// Checks that:
/// - `{…}` placeholders use `$variable` syntax (not bare MF1 identifiers)
/// - `.match` blocks have one or more `$`-prefixed selectors and a full-arity
///   wildcard fallback arm whose key count equals the number of selectors
///   (e.g. `when *` for a single selector, `when * *` for two selectors)
///
/// Returns `Err(description)` on the first structural violation.
fn lint_mf2_message(message: &str) -> Result<(), String> {
    let trimmed = message.trim();
    if trimmed.starts_with(".match") {
        lint_mf2_match(trimmed)
    } else {
        lint_mf2_pattern(trimmed)
    }
}

fn mf2_variables(message: &str) -> Vec<String> {
    let mut variables = BTreeSet::new();
    collect_mf2_variables(message, &mut variables);
    variables.into_iter().collect()
}

fn collect_mf2_variables(input: &str, variables: &mut BTreeSet<String>) {
    let mut cursor = 0usize;
    while let Some(offset) = input.get(cursor..).and_then(|s| s.find('{')) {
        let open = cursor + offset;
        let Some(close) = find_matching_brace(input, open) else {
            break;
        };
        let Some(inner) = input.get(open + 1..close).map(str::trim) else {
            break;
        };

        if let Some(rest) = inner.strip_prefix('$')
            && let Some(name) = rest.split_whitespace().next()
            && !name.is_empty()
        {
            variables.insert(name.to_string());
        }

        collect_mf2_variables(inner, variables);
        cursor = close + 1;
    }
}

/// Validate variable placeholders in a plain MF2 pattern string.
fn lint_mf2_pattern(pattern: &str) -> Result<(), String> {
    let mut cursor = 0usize;
    while let Some(offset) = pattern.get(cursor..).and_then(|s| s.find('{')) {
        let open = cursor + offset;
        let close = find_matching_brace(pattern, open)
            .ok_or_else(|| format!("unmatched '{{' at byte offset {open}"))?;
        let inner = pattern
            .get(open + 1..close)
            .ok_or_else(|| "invalid brace range".to_string())?
            .trim();
        if inner.is_empty() {
            return Err("empty placeholder {}".to_string());
        }
        if !inner.starts_with('$') {
            return Err(format!(
                "placeholder '{{{inner}}}' must use $variable syntax \
                 (bare identifiers are MF1, not MF2)"
            ));
        }
        cursor = close + 1;
    }
    Ok(())
}

/// Validate an MF2 `.match` block.
fn lint_mf2_match(message: &str) -> Result<(), String> {
    let (selector_count, variants) = lint_mf2_selectors(message)?;
    lint_mf2_variants(variants, selector_count)?;
    Ok(())
}

fn lint_mf2_selectors(message: &str) -> Result<(usize, &str), String> {
    #[allow(clippy::string_slice, reason = "'.match' is 1-byte ASCII")]
    let mut rest = message[".match".len()..].trim_start();
    let mut selector_count = 0usize;

    while rest.starts_with('{') {
        let close = find_matching_brace(rest, 0).ok_or("unmatched '{' in .match selector")?;
        let selector = rest.get(1..close).ok_or("invalid selector range")?.trim();
        if !selector.starts_with('$') {
            return Err(format!(
                "selector '{selector}' must start with $ (e.g. {{$count :plural}})"
            ));
        }
        selector_count += 1;
        rest = rest
            .get(close + 1..)
            .ok_or("missing when blocks after .match selector")?
            .trim_start();
    }

    if selector_count == 0 {
        return Err("missing selector after .match".to_string());
    }

    Ok((selector_count, rest))
}

fn lint_mf2_variants(variants: &str, selector_count: usize) -> Result<(), String> {
    let mut rest = variants.trim();
    let mut saw_when = false;
    let mut saw_wildcard = false;

    while !rest.is_empty() {
        if !rest.starts_with("when") {
            return Err(format!("expected 'when' block, found '{rest}'"));
        }

        saw_when = true;
        #[allow(clippy::string_slice, reason = "'when' is 1-byte ASCII")]
        let after_when = rest["when".len()..].trim_start();
        let brace_pos = after_when
            .find('{')
            .ok_or("missing pattern body in when block")?;
        #[allow(clippy::string_slice, reason = "brace_pos is found via find('{')")]
        let key_text = after_when[..brace_pos].trim();
        let keys: Vec<&str> = key_text.split_whitespace().collect();
        if keys.len() != selector_count {
            return Err(format!(
                "when block has {} selector keys but .match has {selector_count} selectors",
                keys.len()
            ));
        }
        if keys.iter().all(|key| *key == "*") {
            saw_wildcard = true;
        }

        let open_brace_index = rest.len() - after_when.len() + brace_pos;
        let close_brace_index = find_matching_brace(rest, open_brace_index)
            .ok_or("unmatched '{' in when block pattern")?;
        rest = rest
            .get(close_brace_index + 1..)
            .ok_or("invalid when block range")?
            .trim_start();
    }

    if !saw_when {
        return Err("no 'when' blocks found in .match".to_string());
    }
    if !saw_wildcard {
        let wildcard = vec!["*"; selector_count].join(" ");
        return Err(format!(
            "missing wildcard 'when {wildcard}' fallback in .match"
        ));
    }

    Ok(())
}

fn find_matching_brace(input: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;

    for (index, ch) in input
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
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
mod tests {
    use super::*;
    use crate::locale::types::{EvaluationConfig, MessageSyntax};
    use crate::template::{TemplateNumber, TemplateTerm};
    use std::collections::HashMap;

    #[test]
    fn test_lint_raw_locale_reports_invalid_mf2_syntax() {
        let raw = RawLocale {
            locale: "en-US".into(),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            messages: HashMap::from([(
                "term.page-label".into(),
                // MF1-style bare identifier — invalid in MF2 (must use $count)
                "{count}".into(),
            )]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.path == "messages.term.page-label"
                && finding.message.contains("invalid MF2 message")
        }));
    }

    #[test]
    fn test_lint_raw_locale_accepts_multi_selector_mf2() {
        let raw = RawLocale {
            locale: "es-ES".into(),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            messages: HashMap::from([(
                "role.editor.label-long".into(),
                ".match {$gender :select} {$count :plural}\nwhen feminine one {editora}\nwhen feminine * {editoras}\nwhen * * {equipo editorial}".into(),
            )]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_lint_raw_locale_rejects_multi_selector_arity_mismatch() {
        let raw = RawLocale {
            locale: "es-ES".into(),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            messages: HashMap::from([(
                "role.editor.label-long".into(),
                ".match {$gender :select} {$count :plural}\nwhen feminine {editora}\nwhen * * {equipo editorial}".into(),
            )]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.path == "messages.role.editor.label-long"
                && finding.message.contains("selector keys")
        }));
    }

    #[test]
    fn test_lint_raw_locale_rejects_missing_multi_selector_wildcard() {
        let raw = RawLocale {
            locale: "es-ES".into(),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            messages: HashMap::from([(
                "role.editor.label-long".into(),
                ".match {$gender :select} {$count :plural}\nwhen feminine one {editora}\nwhen feminine * {editoras}".into(),
            )]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.path == "messages.role.editor.label-long"
                && finding.message.contains("when * *")
        }));
    }

    #[test]
    fn test_lint_raw_locale_reports_missing_alias_target() {
        let raw = RawLocale {
            locale: "en-US".into(),
            messages: HashMap::from([("term.page-label".into(), "p.".into())]),
            legacy_term_aliases: HashMap::from([("page".into(), "term.page-label-long".into())]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.path == "legacy-term-aliases.page" && finding.message.contains("does not exist")
        }));
    }

    #[test]
    fn test_lint_raw_locale_warns_for_v2_locale_missing_grammar_options_and_date_formats() {
        let raw = RawLocale {
            locale: "zz-ZZ".into(),
            locale_schema_version: Some("2".into()),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(
            !report.has_errors(),
            "completeness gaps are warnings, not errors"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.path == "grammar-options")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.path == "date-formats")
        );
    }

    #[test]
    fn test_lint_raw_locale_accepts_complete_v2_locale() {
        let raw = RawLocale {
            locale: "zz-ZZ".into(),
            locale_schema_version: Some("2".into()),
            evaluation: Some(EvaluationConfig {
                message_syntax: MessageSyntax::Mf2,
            }),
            grammar_options: Some(crate::locale::types::GrammarOptions::default()),
            date_formats: HashMap::from([("iso".into(), "yyyy-MM-dd".into())]),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(!report.has_errors());
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.path == "grammar-options" || finding.path == "date-formats")
        );
    }

    #[test]
    fn test_lint_raw_locale_skips_completeness_check_for_v1_locale() {
        let raw = RawLocale {
            locale: "zz-ZZ".into(),
            ..Default::default()
        };

        let report = lint_raw_locale(&raw);

        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_lint_style_against_locale_warns_for_missing_general_term() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Term(TemplateTerm {
                        term: GeneralTerm::NoDate,
                        form: Some(TermForm::Short),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::default();

        let report = lint_style_against_locale(&style, &locale);

        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.severity == LintSeverity::Warning
                && finding.path == "citation.template[0]"
                && finding.message.contains("general term")
        }));
    }

    #[test]
    fn test_lint_style_against_locale_errors_for_missing_message_id() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Message(TemplateMessage {
                        message: "pattern.missing".into(),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let report = lint_style_against_locale(&style, &locale);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.severity == LintSeverity::Error
                && finding.path == "citation.template[0]"
                && finding.message.contains("pattern.missing")
        }));
    }

    #[test]
    fn test_lint_style_checks_messages_referenced_by_fallback_options() {
        let options: Config = serde_yaml::from_str(
            r#"
substitute:
  otherwise:
    message: pattern.missing-author
date-fallback:
  first-issued:
    default:
    - message: pattern.missing-date
"#,
        )
        .expect("fallback options should parse");
        let style = Style {
            options: Some(options),
            ..Default::default()
        };

        let report = lint_style_against_locale(&style, &Locale::en_us());

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.path == "options.substitute.otherwise"
                && finding.message.contains("pattern.missing-author")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.path == "options.date-fallback.first-issued.default[0]"
                && finding.message.contains("pattern.missing-date")
        }));
    }

    #[test]
    fn test_lint_style_against_locale_accepts_term_backed_message_id() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Message(TemplateMessage {
                        message: "term.edition".into(),
                        form: Some(TermForm::Short),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(include_str!("../embedded/locales/en-US.yaml"))
            .expect("english locale should parse");

        let report = lint_style_against_locale(&style, &locale);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_lint_style_against_locale_errors_for_missing_message_arg() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Message(TemplateMessage {
                        message: "pattern.accessed-date".into(),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut locale = Locale::en_us();
        locale
            .messages
            .insert("pattern.accessed-date".into(), "accessed {$date}".into());

        let report = lint_style_against_locale(&style, &locale);

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.severity == LintSeverity::Error
                && finding.path == "citation.template[0]"
                && finding.message.contains("$date")
        }));
    }

    #[test]
    fn test_lint_style_against_locale_warns_for_missing_role_term() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Editor.into(),
                        form: ContributorForm::Verb,
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::default();

        let report = lint_style_against_locale(&style, &locale);

        assert!(!report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.severity == LintSeverity::Warning
                && finding.path == "citation.template[0]"
                && finding.message.contains("role term")
        }));
    }

    #[test]
    fn test_lint_style_against_locale_accepts_custom_locator_when_locale_defines_it() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Number(TemplateNumber {
                        number: NumberVariable::Custom("reel".to_string()),
                        label_form: Some(TemplateLabelForm::Short),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  reel:
    short:
      singular: "reel"
      plural: "reels"
"#,
        )
        .expect("custom locale should parse");

        let report = lint_style_against_locale(&style, &locale);

        assert!(
            report.findings.is_empty(),
            "unexpected findings: {:?}",
            report.findings
        );
    }

    #[test]
    fn test_lint_style_against_locale_warns_for_missing_custom_locator_term() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Number(TemplateNumber {
                        number: NumberVariable::Custom("reel".to_string()),
                        label_form: Some(TemplateLabelForm::Short),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::default();

        let report = lint_style_against_locale(&style, &locale);

        assert!(report.findings.iter().any(|finding| {
            finding.severity == LintSeverity::Warning
                && finding.path == "citation.template[0]"
                && finding.message.contains("locator term")
        }));
    }
}
