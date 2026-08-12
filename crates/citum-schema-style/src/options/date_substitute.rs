/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Options-level fallback policies for the identity date slot.

use crate::locale::TermForm;
use crate::template::{
    DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate, TemplateMessage,
    TypeSelector,
};
use indexmap::IndexMap;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An options-level date fallback candidate.
///
/// The closed candidate set deliberately permits only dates and locale messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateSubstituteCandidate {
    /// Render another date variable.
    Date(DateSubstituteDate),
    /// Render a locale message, normally `term.no-date`.
    Message(DateSubstituteMessage),
}

impl DateSubstituteCandidate {
    /// Materialize this closed candidate as an ordinary template component.
    #[must_use]
    pub fn to_template_component(&self) -> TemplateComponent {
        match self {
            Self::Date(candidate) => TemplateComponent::Date(TemplateDate {
                date: candidate.date.clone(),
                form: candidate.form.clone(),
                suppress_note: candidate.suppress_note,
                rendering: candidate.rendering.clone(),
                ..TemplateDate::default()
            }),
            Self::Message(candidate) => TemplateComponent::Message(TemplateMessage {
                message: candidate.message.clone(),
                form: candidate.form.clone(),
                rendering: candidate.rendering.clone(),
                ..TemplateMessage::default()
            }),
        }
    }
}

/// A date candidate in an options-level substitution policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateSubstituteDate {
    /// Date variable to try.
    pub date: DateVariable,
    /// Rendering form for the candidate date.
    pub form: DateForm,
    /// Suppress an opaque calendar annotation for this candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_note: Option<bool>,
    /// Candidate-local rendering configuration.
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// A locale-message candidate in an options-level substitution policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateSubstituteMessage {
    /// Locale message ID to render.
    pub message: String,
    /// Optional term form for a term-backed message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    /// Candidate-local rendering configuration.
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// A flat, insertion-ordered date-substitution selector map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DateSubstitute(IndexMap<TypeSelector, Vec<DateSubstituteCandidate>>);

#[cfg(feature = "schema")]
impl JsonSchema for DateSubstitute {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DateSubstitute".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        crate::template::type_keyed_map_schema::<Vec<DateSubstituteCandidate>>(generator)
    }
}

impl DateSubstitute {
    /// Construct a policy from an insertion-ordered selector map.
    #[must_use]
    pub fn new(entries: IndexMap<TypeSelector, Vec<DateSubstituteCandidate>>) -> Self {
        Self(entries)
    }

    /// Return the policy's insertion-ordered selector map.
    #[must_use]
    pub fn entries(&self) -> &IndexMap<TypeSelector, Vec<DateSubstituteCandidate>> {
        &self.0
    }

    /// Resolve candidates for a reference type.
    ///
    /// The first matching non-default selector wins, followed by the exact
    /// `default` selector. `None` preserves inline or implicit behavior; an
    /// empty matched slice intentionally blanks the identity date slot.
    #[must_use]
    pub fn candidates_for(&self, ref_type: &str) -> Option<&[DateSubstituteCandidate]> {
        self.0
            .iter()
            .find(|(selector, _)| !selector.is_default() && selector.matches(ref_type))
            .map(|(_, candidates)| candidates.as_slice())
            .or_else(|| {
                self.0
                    .iter()
                    .find(|(selector, _)| selector.is_default())
                    .map(|(_, candidates)| candidates.as_slice())
            })
    }

    /// Merge another selector map over this one per key.
    ///
    /// Each candidate list replaces as a whole. Replaced keys keep their
    /// existing insertion position and new keys append.
    pub fn merge(&mut self, other: &Self) {
        for (selector, candidates) in &other.0 {
            self.0.insert(selector.clone(), candidates.clone());
        }
    }
}

/// A named or explicit date-substitution policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateSubstituteEntry {
    /// A named built-in policy.
    Preset(DateSubstitutePreset),
    /// An explicit flat selector map.
    Explicit(DateSubstitute),
}

impl DateSubstituteEntry {
    /// Eagerly expand this entry to its concrete selector map.
    #[must_use]
    pub fn resolve(&self) -> DateSubstitute {
        match self {
            Self::Preset(preset) => preset.config(),
            Self::Explicit(config) => config.clone(),
        }
    }
}

/// Built-in identity-date substitution policies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateSubstitutePreset {
    /// Generic no-date baseline.
    #[default]
    Standard,
    /// Shared numeric and note-base policy for GB/T 7714-2025.
    #[serde(rename = "gb-t-7714-2025")]
    GbT7714_2025,
    /// Author-date policy for GB/T 7714-2025.
    #[serde(rename = "gb-t-7714-2025-author-date")]
    GbT7714_2025AuthorDate,
}

impl DateSubstitutePreset {
    /// Expand the preset to a concrete insertion-ordered selector map.
    #[must_use]
    pub fn config(self) -> DateSubstitute {
        match self {
            Self::Standard => standard_config(),
            Self::GbT7714_2025 => gb_t_config(),
            Self::GbT7714_2025AuthorDate => gb_t_author_date_config(),
        }
    }
}

fn selector(value: &str) -> TypeSelector {
    value.parse().unwrap_or_else(|never| match never {})
}

fn message_no_date() -> DateSubstituteCandidate {
    DateSubstituteCandidate::Message(DateSubstituteMessage {
        message: "term.no-date".to_string(),
        form: Some(TermForm::Short),
        rendering: Rendering::default(),
    })
}

fn date_candidate(
    date: DateVariable,
    prefix: Option<&str>,
    suffix: Option<&str>,
    brackets: bool,
) -> DateSubstituteCandidate {
    DateSubstituteCandidate::Date(DateSubstituteDate {
        date,
        form: DateForm::Year,
        suppress_note: None,
        rendering: Rendering {
            prefix: prefix.map(Into::into),
            suffix: suffix.map(Into::into),
            wrap: brackets.then_some(crate::template::WrapConfig {
                punctuation: crate::template::WrapPunctuation::Brackets,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Rendering::default()
        },
    })
}

fn publication_year_candidates(include_no_date: bool) -> Vec<DateSubstituteCandidate> {
    let mut candidates = vec![
        date_candidate(DateVariable::Copyright, Some("c"), None, false),
        date_candidate(DateVariable::Printing, None, Some("印刷"), false),
        date_candidate(DateVariable::Accessed, None, None, true),
    ];
    if include_no_date {
        candidates.push(message_no_date());
    }
    candidates
}

fn standard_config() -> DateSubstitute {
    DateSubstitute::new(IndexMap::from([(
        selector("default"),
        vec![message_no_date()],
    )]))
}

fn gb_t_config() -> DateSubstitute {
    DateSubstitute::new(IndexMap::from([
        (selector("default"), vec![]),
        (
            selector("chapter,entry-dictionary,entry-encyclopedia"),
            vec![date_candidate(DateVariable::Accessed, None, None, true)],
        ),
        (
            selector("book,thesis,map"),
            publication_year_candidates(false),
        ),
    ]))
}

fn gb_t_author_date_config() -> DateSubstitute {
    DateSubstitute::new(IndexMap::from([
        (selector("default"), vec![message_no_date()]),
        (selector("article-journal,article-magazine"), vec![]),
        (
            selector("webpage,post,post-weblog"),
            vec![
                date_candidate(DateVariable::Accessed, None, None, true),
                message_no_date(),
            ],
        ),
        (
            selector("book,thesis,map"),
            publication_year_candidates(true),
        ),
    ]))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions")]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("standard", DateSubstitutePreset::Standard)]
    #[case("gb-t-7714-2025", DateSubstitutePreset::GbT7714_2025)]
    #[case(
        "gb-t-7714-2025-author-date",
        DateSubstitutePreset::GbT7714_2025AuthorDate
    )]
    fn all_preset_names_deserialize(#[case] yaml: &str, #[case] expected: DateSubstitutePreset) {
        let actual: DateSubstitutePreset =
            serde_yaml::from_str(yaml).expect("preset name should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn preset_resolves_first_matching_selector_before_default() {
        let policy = DateSubstitutePreset::GbT7714_2025AuthorDate.config();

        let journal = policy
            .candidates_for("article-journal")
            .expect("journal selector should match");
        let report = policy
            .candidates_for("report")
            .expect("default selector should match");

        assert!(journal.is_empty());
        assert!(matches!(report, [DateSubstituteCandidate::Message(_)]));
    }

    #[test]
    fn selector_merge_replaces_lists_without_reordering_keys() {
        let mut base = DateSubstitute::new(IndexMap::from([
            (selector("default"), vec![message_no_date()]),
            (selector("book"), vec![]),
        ]));
        let override_policy = DateSubstitute::new(IndexMap::from([
            (
                selector("default"),
                vec![date_candidate(DateVariable::Accessed, None, None, true)],
            ),
            (selector("report"), vec![]),
        ]));

        base.merge(&override_policy);

        let keys: Vec<String> = base.entries().keys().map(ToString::to_string).collect();
        assert_eq!(keys, ["default", "book", "report"]);
        assert!(matches!(
            base.candidates_for("legal-case"),
            Some([DateSubstituteCandidate::Date(_)])
        ));
    }
}
