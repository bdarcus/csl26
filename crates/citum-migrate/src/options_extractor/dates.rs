/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::locale::TermForm;
use citum_schema::options::{
    DateConfig, DateFallback, DateFallbackCandidate, DateFallbackConfig, DateFallbackDate,
    DateFallbackLane, DateFallbackMessage, DateFallbackRule, DateFallbackSelectorMap, MonthFormat,
};
use citum_schema::template::{DateForm, DateVariable, Rendering, TypeSelector};
use csl_legacy::model::{CslNode, Style};
use indexmap::IndexMap;
use std::collections::HashSet;

/// Result of extracting options-level issued-date fallback behavior.
#[derive(Debug, Default)]
pub struct DateFallbackExtraction {
    /// A representable fallback policy, when the source authored one.
    pub policy: Option<DateFallbackConfig>,
    /// Whether an authored conditional fallback could not be represented safely.
    pub unsupported: bool,
}

/// Extract explicit issued-date fallback branches from CSL conditionals.
#[must_use]
pub fn extract_date_fallback(style: &Style) -> DateFallbackExtraction {
    let mut extraction = DateFallbackExtraction::default();
    let mut visiting = HashSet::new();
    if let Some(bibliography) = &style.bibliography {
        scan_date_fallbacks(
            &bibliography.layout.children,
            style,
            &mut visiting,
            &mut extraction,
            false,
        );
    }
    scan_date_fallbacks(
        &style.citation.layout.children,
        style,
        &mut visiting,
        &mut extraction,
        false,
    );
    if extraction.unsupported {
        extraction.policy = None;
    }
    extraction
}

fn scan_date_fallbacks(
    nodes: &[CslNode],
    style: &Style,
    visiting: &mut HashSet<String>,
    extraction: &mut DateFallbackExtraction,
    type_scoped: bool,
) {
    for node in nodes {
        match node {
            CslNode::Choose(choose) => {
                let choose_has_type_condition = choose.if_branch.type_.is_some()
                    || choose
                        .else_if_branches
                        .iter()
                        .any(|branch| branch.type_.is_some());
                let nested_type_scoped = type_scoped || choose_has_type_condition;
                if branch_tests_only_issued(&choose.if_branch)
                    && contains_date_variable(
                        &choose.if_branch.children,
                        "issued",
                        style,
                        &mut HashSet::new(),
                    )
                    && (!choose.else_if_branches.is_empty() || choose.else_branch.is_some())
                {
                    if nested_type_scoped {
                        extraction.unsupported = true;
                    } else {
                        match fallback_candidates(choose, style, visiting) {
                            Some(candidates) if !candidates.is_empty() => {
                                let policy = fallback_policy(candidates);
                                match &extraction.policy {
                                    None => extraction.policy = Some(policy),
                                    Some(existing) if existing == &policy => {}
                                    Some(_) => extraction.unsupported = true,
                                }
                            }
                            _ => extraction.unsupported = true,
                        }
                    }
                }
                scan_date_fallbacks(
                    &choose.if_branch.children,
                    style,
                    visiting,
                    extraction,
                    nested_type_scoped,
                );
                for branch in &choose.else_if_branches {
                    scan_date_fallbacks(
                        &branch.children,
                        style,
                        visiting,
                        extraction,
                        nested_type_scoped,
                    );
                }
                if let Some(branch) = &choose.else_branch {
                    scan_date_fallbacks(branch, style, visiting, extraction, nested_type_scoped);
                }
            }
            CslNode::Group(group) => {
                scan_date_fallbacks(&group.children, style, visiting, extraction, type_scoped);
            }
            CslNode::Names(names) => {
                scan_date_fallbacks(&names.children, style, visiting, extraction, type_scoped);
            }
            CslNode::Text(text) => {
                let Some(name) = &text.macro_name else {
                    continue;
                };
                if !visiting.insert(name.clone()) {
                    continue;
                }
                if let Some(style_macro) = style
                    .macros
                    .iter()
                    .find(|candidate| &candidate.name == name)
                {
                    scan_date_fallbacks(
                        &style_macro.children,
                        style,
                        visiting,
                        extraction,
                        type_scoped,
                    );
                }
                visiting.remove(name);
            }
            _ => {}
        }
    }
}

fn branch_tests_only_issued(branch: &csl_legacy::model::ChooseBranch) -> bool {
    branch.variable.as_deref().is_some_and(|variables| {
        let mut variables = variables.split_whitespace();
        variables.next() == Some("issued") && variables.next().is_none()
    }) && branch.type_.is_none()
        && branch.is_numeric.is_none()
        && branch.is_uncertain_date.is_none()
        && branch.locator.is_none()
        && branch.position.is_none()
        && branch
            .match_mode
            .as_deref()
            .is_none_or(|mode| mode == "all")
}

fn fallback_candidates(
    choose: &csl_legacy::model::Choose,
    style: &Style,
    visiting: &mut HashSet<String>,
) -> Option<Vec<DateFallbackCandidate>> {
    let mut candidates = Vec::new();
    for branch in &choose.else_if_branches {
        if branch.type_.is_some()
            || branch.is_numeric.is_some()
            || branch.is_uncertain_date.is_some()
            || branch.locator.is_some()
            || branch.position.is_some()
            || branch
                .match_mode
                .as_deref()
                .is_some_and(|mode| mode != "all")
        {
            return None;
        }
        let candidate = candidate_from_nodes(&branch.children, style, visiting)?;
        let tested = branch.variable.as_deref()?;
        let DateFallbackCandidate::Date(date) = &candidate else {
            return None;
        };
        let mut tested = tested.split_whitespace();
        if tested.next() != Some(date_variable_name(&date.date)) || tested.next().is_some() {
            return None;
        }
        candidates.push(candidate);
    }
    if let Some(branch) = &choose.else_branch {
        candidates.push(candidate_from_nodes(branch, style, visiting)?);
    }
    Some(candidates)
}

fn candidate_from_nodes(
    nodes: &[CslNode],
    style: &Style,
    visiting: &mut HashSet<String>,
) -> Option<DateFallbackCandidate> {
    let mut candidate = None;
    for node in nodes {
        if is_year_suffix_node(node) {
            continue;
        }
        let next = candidate_from_node(node, style, visiting)?;
        if candidate.replace(next).is_some() {
            return None;
        }
    }
    candidate
}

fn is_year_suffix_node(node: &CslNode) -> bool {
    matches!(node, CslNode::Text(text) if text.variable.as_deref() == Some("year-suffix"))
}

fn candidate_from_node(
    node: &CslNode,
    style: &Style,
    visiting: &mut HashSet<String>,
) -> Option<DateFallbackCandidate> {
    match node {
        CslNode::Date(date) => Some(DateFallbackCandidate::Date(DateFallbackDate {
            date: date_variable(&date.variable)?,
            form: date_form(date),
            suppress_note: None,
            rendering: rendering(
                date.prefix.as_deref(),
                date.suffix.as_deref(),
                None,
                None,
                None,
                &date.formatting,
            ),
        })),
        CslNode::Text(text) if text.term.as_deref() == Some("no date") => {
            Some(DateFallbackCandidate::Message(DateFallbackMessage {
                message: "term.no-date".to_string(),
                form: term_form(text.form.as_deref()),
                rendering: rendering(
                    text.prefix.as_deref(),
                    text.suffix.as_deref(),
                    text.quotes,
                    text.strip_periods,
                    text.text_case.as_deref(),
                    &text.formatting,
                ),
            }))
        }
        CslNode::Text(text) => {
            let name = text.macro_name.as_ref()?;
            if !visiting.insert(name.clone()) {
                return None;
            }
            let result = style
                .macros
                .iter()
                .find(|candidate| &candidate.name == name)
                .and_then(|style_macro| {
                    candidate_from_nodes(&style_macro.children, style, visiting)
                });
            visiting.remove(name);
            result
        }
        CslNode::Group(group) if group.children.len() == 1 => {
            if group_has_unrepresentable_rendering(group) {
                None
            } else {
                candidate_from_nodes(&group.children, style, visiting)
            }
        }
        CslNode::Group(group) => candidate_from_nodes(&group.children, style, visiting),
        _ => None,
    }
}

fn group_has_unrepresentable_rendering(group: &csl_legacy::model::Group) -> bool {
    group.prefix.is_some()
        || group.suffix.is_some()
        || group.formatting.font_style.is_some()
        || group.formatting.font_variant.is_some()
        || group.formatting.font_weight.is_some()
        || group.formatting.text_decoration.is_some()
        || group.formatting.vertical_align.is_some()
        || group.formatting.display.is_some()
}

fn fallback_policy(candidates: Vec<DateFallbackCandidate>) -> DateFallbackConfig {
    DateFallbackConfig::Policy(DateFallback {
        first_issued: Some(DateFallbackLane::Selectors(DateFallbackSelectorMap::new(
            IndexMap::from([(
                TypeSelector::Single("default".to_string()),
                DateFallbackRule::Candidates(candidates),
            )]),
        ))),
        later_issued: None,
    })
}

fn contains_date_variable(
    nodes: &[CslNode],
    variable: &str,
    style: &Style,
    visiting: &mut HashSet<String>,
) -> bool {
    nodes.iter().any(|node| match node {
        CslNode::Date(date) => date.variable == variable,
        CslNode::Group(group) => contains_date_variable(&group.children, variable, style, visiting),
        CslNode::Choose(choose) => {
            contains_date_variable(&choose.if_branch.children, variable, style, visiting)
                || choose.else_if_branches.iter().any(|branch| {
                    contains_date_variable(&branch.children, variable, style, visiting)
                })
                || choose
                    .else_branch
                    .as_ref()
                    .is_some_and(|branch| contains_date_variable(branch, variable, style, visiting))
        }
        CslNode::Text(text) => text.macro_name.as_ref().is_some_and(|name| {
            if !visiting.insert(name.clone()) {
                return false;
            }
            let result = style
                .macros
                .iter()
                .find(|candidate| &candidate.name == name)
                .is_some_and(|style_macro| {
                    contains_date_variable(&style_macro.children, variable, style, visiting)
                });
            visiting.remove(name);
            result
        }),
        _ => false,
    })
}

fn date_variable(value: &str) -> Option<DateVariable> {
    match value {
        "issued" => Some(DateVariable::Issued),
        "accessed" => Some(DateVariable::Accessed),
        "original-date" => Some(DateVariable::OriginalPublished),
        "event-date" => Some(DateVariable::EventDate),
        "copyright" => Some(DateVariable::Copyright),
        "printing" => Some(DateVariable::Printing),
        _ => None,
    }
}

fn date_variable_name(value: &DateVariable) -> &'static str {
    match value {
        DateVariable::Issued => "issued",
        DateVariable::Accessed => "accessed",
        DateVariable::OriginalPublished => "original-date",
        DateVariable::EventDate => "event-date",
        DateVariable::Copyright => "copyright",
        DateVariable::Printing => "printing",
        _ => "",
    }
}

fn date_form(date: &csl_legacy::model::Date) -> DateForm {
    match date.date_parts.as_deref() {
        Some("year") => DateForm::Year,
        Some("year-month") => DateForm::YearMonth,
        _ if date.parts.len() == 1
            && date.parts.first().is_some_and(|part| part.name == "year") =>
        {
            DateForm::Year
        }
        _ => DateForm::Full,
    }
}

fn term_form(value: Option<&str>) -> Option<TermForm> {
    match value {
        Some("short") => Some(TermForm::Short),
        Some("long") | None => Some(TermForm::Long),
        Some("verb") => Some(TermForm::Verb),
        Some("verb-short") => Some(TermForm::VerbShort),
        Some("symbol") => Some(TermForm::Symbol),
        Some(_) => None,
    }
}

fn rendering(
    prefix: Option<&str>,
    suffix: Option<&str>,
    quote: Option<bool>,
    strip_periods: Option<bool>,
    text_case: Option<&str>,
    formatting: &csl_legacy::model::Formatting,
) -> Rendering {
    Rendering {
        prefix: prefix.map(Into::into),
        suffix: suffix.map(Into::into),
        quote,
        strip_periods,
        text_case: match text_case {
            Some("capitalize-first") => {
                Some(citum_schema::options::titles::TextCase::CapitalizeFirst)
            }
            Some("lowercase") => Some(citum_schema::options::titles::TextCase::Lowercase),
            Some("uppercase") => Some(citum_schema::options::titles::TextCase::Uppercase),
            _ => None,
        },
        emph: formatting
            .font_style
            .as_deref()
            .is_some_and(|style| style == "italic" || style == "oblique")
            .then_some(true),
        small_caps: (formatting.font_variant.as_deref() == Some("small-caps")).then_some(true),
        strong: (formatting.font_weight.as_deref() == Some("bold")).then_some(true),
        ..Rendering::default()
    }
}

/// Extracts date configuration options from a CSL style.
///
/// Analyzes the style's date layouts to determine month format
/// and other date presentation options.
#[must_use]
pub fn extract_date_config(style: &Style) -> Option<DateConfig> {
    let mut config = DateConfig::default();
    let mut found_date = false;

    // Scan bibliography for month format
    if let Some(bib) = &style.bibliography {
        if let Some(format) = scan_for_month_format(&bib.layout.children, style) {
            config.month = format;
            found_date = true;
        } else if scan_for_any_date(&bib.layout.children, style) {
            found_date = true;
        }
    }

    // Fallback to citation if bibliography didn't have it
    if !found_date {
        if let Some(format) = scan_for_month_format(&style.citation.layout.children, style) {
            config.month = format;
            found_date = true;
        } else if scan_for_any_date(&style.citation.layout.children, style) {
            found_date = true;
        }
    }

    if found_date { Some(config) } else { None }
}

fn scan_for_any_date(nodes: &[CslNode], style: &Style) -> bool {
    for node in nodes {
        match node {
            CslNode::Date(_) => return true,
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && scan_for_any_date(&m.children, style)
                {
                    return true;
                }
            }
            CslNode::Group(g) if scan_for_any_date(&g.children, style) => {
                return true;
            }
            CslNode::Choose(c) => {
                if scan_for_any_date(&c.if_branch.children, style) {
                    return true;
                }
                for branch in &c.else_if_branches {
                    if scan_for_any_date(&branch.children, style) {
                        return true;
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && scan_for_any_date(else_branch, style)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn scan_for_month_format(nodes: &[CslNode], style: &Style) -> Option<MonthFormat> {
    for node in nodes {
        match node {
            CslNode::Date(d) => {
                if let Some(form) = &d.form {
                    return Some(match form.as_str() {
                        "short" => MonthFormat::Short,
                        "numeric" | "numeric-leading-zeros" => MonthFormat::Numeric,
                        _ => MonthFormat::Long,
                    });
                }
                // Check parts for month form
                for part in &d.parts {
                    if part.name == "month"
                        && let Some(form) = &part.form
                    {
                        return Some(match form.as_str() {
                            "short" => MonthFormat::Short,
                            "numeric" | "numeric-leading-zeros" => MonthFormat::Numeric,
                            _ => MonthFormat::Long,
                        });
                    }
                }
            }
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && let Some(format) = scan_for_month_format(&m.children, style)
                {
                    return Some(format);
                }
            }
            CslNode::Group(g) => {
                if let Some(format) = scan_for_month_format(&g.children, style) {
                    return Some(format);
                }
            }
            CslNode::Choose(c) => {
                if let Some(format) = scan_for_month_format(&c.if_branch.children, style) {
                    return Some(format);
                }
                for branch in &c.else_if_branches {
                    if let Some(format) = scan_for_month_format(&branch.children, style) {
                        return Some(format);
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && let Some(format) = scan_for_month_format(else_branch, style)
                {
                    return Some(format);
                }
            }
            _ => {}
        }
    }
    None
}
