/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Typed scoped options that normalize resolved citation and bibliography specs.

use crate::options::bibliography::SubsequentAuthorSubstituteRule;
use crate::template::{NumberVariable, TemplateComponent, WrapConfig, WrapPunctuation};
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

/// Declarative presentation mode for processor-generated citation labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CitationLabelMode {
    /// Do not generate or render citation labels of either kind.
    None,
    /// Generate numeric citation labels from processor-owned citation numbers.
    Numeric,
    /// Generate alphabetic (trigraph) citation labels, as in biblatex `alpha`.
    Alphabetic,
}

impl CitationLabelMode {
    /// The number variable this label mode generates, if any.
    #[must_use]
    pub fn label_variable(self) -> Option<NumberVariable> {
        match self {
            CitationLabelMode::None => None,
            CitationLabelMode::Numeric => Some(NumberVariable::CitationNumber),
            CitationLabelMode::Alphabetic => Some(NumberVariable::CitationLabel),
        }
    }
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
    /// Alphabetic (trigraph) bibliography labels, as in biblatex `alpha`.
    Alphabetic,
    /// Author-date bibliography labels.
    AuthorDate,
}

impl BibliographyLabelMode {
    /// The number variable this label mode generates, if any.
    #[must_use]
    pub fn label_variable(self) -> Option<NumberVariable> {
        match self {
            BibliographyLabelMode::None | BibliographyLabelMode::AuthorDate => None,
            BibliographyLabelMode::Numeric => Some(NumberVariable::CitationNumber),
            BibliographyLabelMode::Alphabetic => Some(NumberVariable::CitationLabel),
        }
    }
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

    if let Some(options) = options
        && let Some(delimiter) = options.group_delimiter
    {
        citation.multi_cite_delimiter = Some(delimiter.as_str().into());
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

    let needs_template = options.date_position.is_some() || options.title_terminator.is_some();
    if needs_template && bibliography.template.is_none() && bibliography.template_ref.is_some() {
        bibliography.template = bibliography.resolve_template();
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
