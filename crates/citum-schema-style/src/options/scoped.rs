/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Typed scoped options that normalize resolved citation and bibliography specs.

use crate::options::bibliography::SubsequentAuthorSubstituteRule;
use crate::template::{TemplateComponent, WrapConfig, WrapPunctuation};
use crate::{BibliographySpec, CitationSpec, Style, Template};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wrapper punctuation supported by citation and bibliography label options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelWrap {
    /// No outer punctuation.
    None,
    /// Parentheses.
    Parentheses,
    /// Brackets.
    Brackets,
    /// Superscript-style numeric labels.
    Superscript,
}

impl LabelWrap {
    /// Convert a supported punctuation style into a concrete wrap config.
    #[must_use]
    pub fn as_wrap_config(self) -> Option<WrapConfig> {
        match self {
            LabelWrap::None => None,
            LabelWrap::Parentheses => Some(WrapConfig::from(WrapPunctuation::Parentheses)),
            LabelWrap::Brackets => Some(WrapConfig::from(WrapPunctuation::Brackets)),
            LabelWrap::Superscript => None,
        }
    }
}

/// Wrapper punctuation supported by bibliography label options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum BibliographyLabelWrap {
    /// No outer punctuation.
    None,
    /// Parentheses.
    Parentheses,
    /// Brackets.
    Brackets,
    /// A trailing period, flush against the following component (no outer wrap).
    Period,
}

impl BibliographyLabelWrap {
    /// Convert a supported punctuation style into a concrete wrap config.
    ///
    /// Returns `None` for [`BibliographyLabelWrap::Period`], since a period is
    /// expressed as a rendering suffix rather than an outer wrap; see
    /// [`BibliographyLabelWrap::as_suffix`].
    #[must_use]
    pub fn as_wrap_config(self) -> Option<WrapConfig> {
        match self {
            BibliographyLabelWrap::None | BibliographyLabelWrap::Period => None,
            BibliographyLabelWrap::Parentheses => {
                Some(WrapConfig::from(WrapPunctuation::Parentheses))
            }
            BibliographyLabelWrap::Brackets => Some(WrapConfig::from(WrapPunctuation::Brackets)),
        }
    }

    /// Return the literal suffix this wrap style appends to the label, if any.
    #[must_use]
    pub fn as_suffix(self) -> Option<&'static str> {
        match self {
            BibliographyLabelWrap::Period => Some("."),
            BibliographyLabelWrap::None
            | BibliographyLabelWrap::Parentheses
            | BibliographyLabelWrap::Brackets => None,
        }
    }
}

/// Delimiters between grouped citations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CitationGroupDelimiter {
    /// `, `
    Comma,
    /// `; `
    Semicolon,
    /// ` `
    Space,
}

impl CitationGroupDelimiter {
    fn as_str(self) -> &'static str {
        match self {
            CitationGroupDelimiter::Comma => ", ",
            CitationGroupDelimiter::Semicolon => "; ",
            CitationGroupDelimiter::Space => " ",
        }
    }
}

/// Bibliography label modes supported by scoped bibliography options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum BibliographyLabelMode {
    /// No explicit label component.
    None,
    /// Numeric bibliography labels.
    Numeric,
    /// Author-date bibliography labels.
    AuthorDate,
}

/// Placement of issued dates inside bibliography entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DatePosition {
    /// Immediately after the contributor component.
    AfterAuthor,
    /// Immediately after the title component.
    AfterTitle,
    /// At the end of the entry.
    Terminal,
}

/// Terminator punctuation for bibliography titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TitleTerminator {
    /// Period.
    Period,
    /// Comma.
    Comma,
    /// No terminator.
    None,
}

impl TitleTerminator {
    fn as_suffix(self) -> Option<&'static str> {
        match self {
            TitleTerminator::Period => Some("."),
            TitleTerminator::Comma => Some(","),
            TitleTerminator::None => None,
        }
    }
}

/// Repeated-author rendering policies for bibliographies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RepeatedAuthorRendering {
    /// Always render full contributor names.
    Full,
    /// Replace repeated authors with an em dash.
    Dash,
    /// Replace repeated authors with an em dash followed by a space.
    DashWithSpace,
}

/// Apply scoped citation and bibliography options to a resolved style.
pub(crate) fn apply_scoped_style_options(style: &mut Style) {
    if let Some(citation) = style.citation.as_mut() {
        apply_citation_options_recursive(citation);
    }
    if let Some(bibliography) = style.bibliography.as_mut() {
        apply_bibliography_options(bibliography);
    }
}

fn apply_citation_options_recursive(citation: &mut CitationSpec) {
    let options = citation.options.clone();

    if let Some(options) = options {
        if let Some(delimiter) = options.group_delimiter {
            citation.multi_cite_delimiter = Some(delimiter.as_str().into());
        }
        if let Some(wrap) = options.label_wrap {
            if citation.template.is_none() && citation.template_ref.is_some() {
                citation.template = citation.resolve_template();
            }
            set_citation_wrap(citation, wrap);
        }
    }

    for child in [
        citation.integral.as_deref_mut(),
        citation.non_integral.as_deref_mut(),
        citation.subsequent.as_deref_mut(),
        citation.ibid.as_deref_mut(),
    ]
    .into_iter()
    .flatten()
    {
        apply_citation_options_recursive(child);
    }
}

fn apply_bibliography_options(bibliography: &mut BibliographySpec) {
    let options = bibliography.options.clone();
    let Some(options) = options else {
        return;
    };

    let needs_template = options.label_mode.is_some()
        || options.label_wrap.is_some()
        || options.date_position.is_some()
        || options.title_terminator.is_some();
    if needs_template && bibliography.template.is_none() && bibliography.template_ref.is_some() {
        bibliography.template = bibliography.resolve_template();
    }

    if let Some(mode) = options.label_mode {
        apply_bibliography_label_mode(bibliography, mode);
    }
    if let Some(wrap) = options.label_wrap {
        apply_bibliography_label_wrap(bibliography, wrap);
    }
    if let Some(position) = options.date_position {
        apply_date_position(bibliography, position);
    }
    if let Some(terminator) = options.title_terminator {
        apply_title_terminator(bibliography, terminator);
    }
    if let Some(repeated) = options.repeated_author_rendering {
        apply_repeated_author_rendering(bibliography, repeated);
    }
}

fn set_citation_wrap(citation: &mut CitationSpec, wrap: LabelWrap) {
    if wrap == LabelWrap::Superscript {
        citation.wrap = None;
        apply_citation_wrap_recursive(citation, wrap);
        return;
    }
    citation.wrap = wrap.as_wrap_config();
    apply_citation_wrap_recursive(citation, wrap);
}

fn apply_bibliography_label_mode(bibliography: &mut BibliographySpec, mode: BibliographyLabelMode) {
    update_label_mode(bibliography.template.as_mut(), mode);
    if let Some(variants) = bibliography.type_variants.as_mut() {
        for variant in variants.values_mut() {
            update_label_mode(variant.as_template_mut(), mode);
        }
    }
}

fn apply_bibliography_label_wrap(bibliography: &mut BibliographySpec, wrap: BibliographyLabelWrap) {
    update_label_wrap(bibliography.template.as_mut(), wrap);
    if let Some(variants) = bibliography.type_variants.as_mut() {
        for variant in variants.values_mut() {
            update_label_wrap(variant.as_template_mut(), wrap);
        }
    }
}

fn apply_date_position(bibliography: &mut BibliographySpec, position: DatePosition) {
    reposition_date(bibliography.template.as_mut(), position);
    if let Some(variants) = bibliography.type_variants.as_mut() {
        for variant in variants.values_mut() {
            reposition_date(variant.as_template_mut(), position);
        }
    }
}

fn apply_title_terminator(bibliography: &mut BibliographySpec, terminator: TitleTerminator) {
    update_title_terminator(bibliography.template.as_mut(), terminator);
    if let Some(variants) = bibliography.type_variants.as_mut() {
        for variant in variants.values_mut() {
            update_title_terminator(variant.as_template_mut(), terminator);
        }
    }
}

fn apply_repeated_author_rendering(
    bibliography: &mut BibliographySpec,
    repeated: RepeatedAuthorRendering,
) {
    let options = bibliography.options.get_or_insert_with(Default::default);
    match repeated {
        RepeatedAuthorRendering::Full => {
            options.subsequent_author_substitute = None;
            options.subsequent_author_substitute_rule = None;
        }
        RepeatedAuthorRendering::Dash => {
            options.subsequent_author_substitute = Some("———".to_string());
            options.subsequent_author_substitute_rule =
                Some(SubsequentAuthorSubstituteRule::CompleteAll);
        }
        RepeatedAuthorRendering::DashWithSpace => {
            options.subsequent_author_substitute = Some("——— ".to_string());
            options.subsequent_author_substitute_rule =
                Some(SubsequentAuthorSubstituteRule::CompleteAll);
        }
    }
}

/// Whether `component` is the citation-number/citation-label variable used
/// as a bibliography entry label.
fn is_bibliography_label(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Number(number)
            if matches!(
                number.number,
                crate::template::NumberVariable::CitationNumber
                    | crate::template::NumberVariable::CitationLabel
            )
    )
}

/// Whether a label is present anywhere in `components`, including nested groups —
/// e.g. the `delimiter: ""` group the `Numeric` branch below wraps it in.
fn template_has_label(components: &[TemplateComponent]) -> bool {
    components.iter().any(|component| {
        is_bibliography_label(component)
            || matches!(
                component,
                TemplateComponent::Group(group) if template_has_label(&group.group)
            )
    })
}

/// Remove any bibliography label from `components`, recursing into groups and
/// collapsing groups left empty or with a single trivial child behind.
fn strip_bibliography_label(components: &mut Vec<TemplateComponent>) {
    components.retain_mut(|component| {
        if is_bibliography_label(component) {
            return false;
        }
        if let TemplateComponent::Group(group) = component {
            strip_bibliography_label(&mut group.group);
        }
        true
    });
    collapse_trivial_groups(components);
}

/// Drop empty groups and flatten groups left with exactly one child that carry
/// no other semantics (no render condition, rendering affixes, or custom fields).
fn collapse_trivial_groups(components: &mut Vec<TemplateComponent>) {
    let mut index = 0;
    while index < components.len() {
        let Some(TemplateComponent::Group(group)) = components.get(index) else {
            index += 1;
            continue;
        };
        if group.group.is_empty() {
            components.remove(index);
            continue;
        }
        if group.group.len() == 1
            && group.render_when.is_none()
            && group.rendering == crate::template::Rendering::default()
            && group.custom.is_none()
        {
            if let TemplateComponent::Group(mut group) = components.remove(index)
                && let Some(child) = group.group.pop()
            {
                components.insert(index, child);
            }
            continue;
        }
        index += 1;
    }
}

fn update_label_mode(template: Option<&mut Template>, mode: BibliographyLabelMode) {
    let Some(template) = template else {
        return;
    };
    match mode {
        BibliographyLabelMode::None | BibliographyLabelMode::AuthorDate => {
            strip_bibliography_label(template);
        }
        BibliographyLabelMode::Numeric => {
            if template_has_label(template) {
                return;
            }
            let label = TemplateComponent::Number(crate::TemplateNumber {
                number: crate::template::NumberVariable::CitationNumber,
                ..Default::default()
            });
            if template.is_empty() {
                template.push(label);
                return;
            }
            let following = template.remove(0);
            template.insert(
                0,
                TemplateComponent::Group(crate::template::TemplateGroup {
                    group: vec![label, following],
                    delimiter: Some(crate::template::DelimiterPunctuation::Custom(String::new())),
                    ..Default::default()
                }),
            );
        }
    }
}

trait LabelWrapConfig {
    fn wrap_config(self) -> Option<WrapConfig>;

    /// Literal suffix this wrap style appends to the label, if any.
    fn suffix(self) -> Option<&'static str>
    where
        Self: Sized,
    {
        None
    }
}

impl LabelWrapConfig for LabelWrap {
    fn wrap_config(self) -> Option<WrapConfig> {
        self.as_wrap_config()
    }
}

impl LabelWrapConfig for BibliographyLabelWrap {
    fn wrap_config(self) -> Option<WrapConfig> {
        self.as_wrap_config()
    }

    fn suffix(self) -> Option<&'static str> {
        self.as_suffix()
    }
}

fn update_label_wrap<W>(template: Option<&mut Template>, wrap: W)
where
    W: Copy + LabelWrapConfig,
{
    let Some(template) = template else {
        return;
    };
    update_label_wrap_components(template, wrap);
}

fn update_label_wrap_components<W>(components: &mut [TemplateComponent], wrap: W)
where
    W: Copy + LabelWrapConfig,
{
    for component in components.iter_mut() {
        match component {
            TemplateComponent::Number(number)
                if matches!(
                    number.number,
                    crate::template::NumberVariable::CitationNumber
                        | crate::template::NumberVariable::CitationLabel
                ) =>
            {
                number.rendering.wrap = wrap.wrap_config();
                number.rendering.suffix = wrap.suffix().map(ToString::to_string).map(Into::into);
            }
            TemplateComponent::Group(group) => {
                update_label_wrap_components(&mut group.group, wrap);
            }
            _ => {}
        }
    }
}

fn apply_citation_superscript(template: Option<&mut Template>) {
    let Some(template) = template else {
        return;
    };
    for component in template.iter_mut() {
        if let TemplateComponent::Number(number) = component
            && matches!(
                number.number,
                crate::template::NumberVariable::CitationNumber
                    | crate::template::NumberVariable::CitationLabel
            )
        {
            number.rendering.vertical_align = Some(crate::VerticalAlign::Superscript);
            number.rendering.wrap = None;
        }
    }
}

fn apply_citation_wrap_recursive(citation: &mut CitationSpec, wrap: LabelWrap) {
    if wrap == LabelWrap::Superscript && citation.template.is_none() {
        citation.template = citation.resolve_template();
    }

    if wrap == LabelWrap::Superscript {
        apply_citation_superscript(citation.template.as_mut());
        if let Some(variants) = citation.type_variants.as_mut() {
            for variant in variants.values_mut() {
                apply_citation_superscript(variant.as_template_mut());
            }
        }
    }

    for child in [
        citation.integral.as_deref_mut(),
        citation.non_integral.as_deref_mut(),
        citation.subsequent.as_deref_mut(),
        citation.ibid.as_deref_mut(),
    ]
    .into_iter()
    .flatten()
    {
        child.wrap = if wrap == LabelWrap::Superscript {
            None
        } else {
            wrap.as_wrap_config()
        };
        apply_citation_wrap_recursive(child, wrap);
    }
}

fn reposition_date(template: Option<&mut Template>, position: DatePosition) {
    let Some(template) = template else {
        return;
    };
    let Some(index) = template.iter().position(|component| {
        matches!(
            component,
            TemplateComponent::Date(date) if date.date == crate::template::DateVariable::Issued
        )
    }) else {
        return;
    };
    let date = template.remove(index);
    let target = match position {
        DatePosition::AfterAuthor => template
            .iter()
            .position(|component| matches!(component, TemplateComponent::Contributor(_)))
            .map(|idx| idx + 1)
            .unwrap_or(0),
        DatePosition::AfterTitle => template
            .iter()
            .position(|component| matches!(component, TemplateComponent::Title(_)))
            .map(|idx| idx + 1)
            .unwrap_or(template.len()),
        DatePosition::Terminal => template.len(),
    };
    template.insert(target, date);
}

fn update_title_terminator(template: Option<&mut Template>, terminator: TitleTerminator) {
    let Some(template) = template else {
        return;
    };
    for component in template.iter_mut() {
        if let TemplateComponent::Title(title) = component
            && title.title == crate::template::TitleType::Primary
        {
            title.rendering.suffix = terminator
                .as_suffix()
                .map(ToString::to_string)
                .map(Into::into);
        }
    }
}
