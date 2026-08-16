/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Options-level fallback policies for missing issued dates.

use crate::locale::TermForm;
use crate::template::{
    DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate, TemplateMessage,
    TypeSelector,
};
use indexmap::IndexMap;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A closed options-level date fallback candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateFallbackCandidate {
    /// Render another date variable.
    Date(DateFallbackDate),
    /// Render a locale message, normally `term.no-date`.
    Message(DateFallbackMessage),
}

impl DateFallbackCandidate {
    /// Convert this closed candidate into the shared component renderer's input.
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

/// A date candidate in an options-level fallback rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateFallbackDate {
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

/// A locale-message candidate in an options-level fallback rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateFallbackMessage {
    /// Locale message ID to render.
    pub message: String,
    /// Optional term form for a term-backed message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    /// Candidate-local rendering configuration.
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Scalar rules accepted in a type-selector map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateFallbackRulePreset {
    /// Render the locale's short no-date message.
    Standard,
    /// Render no fallback and stop selector resolution.
    None,
}

/// One complete rule selected for a reference type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateFallbackRule {
    /// A scalar standard or none rule.
    Preset(DateFallbackRulePreset),
    /// A non-empty ordered candidate list.
    Candidates(Vec<DateFallbackCandidate>),
}

impl<'de> Deserialize<'de> for DateFallbackRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Preset(DateFallbackRulePreset),
            Candidates(Vec<DateFallbackCandidate>),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Preset(preset) => Ok(Self::Preset(preset)),
            Wire::Candidates(candidates) if candidates.is_empty() => Err(serde::de::Error::custom(
                "date fallback candidate lists cannot be empty; use `none`",
            )),
            Wire::Candidates(candidates) => Ok(Self::Candidates(candidates)),
        }
    }
}

impl DateFallbackRule {
    /// Return the effective candidates, or `None` for an intentional blank rule.
    #[must_use]
    pub fn candidates(&self) -> Option<Cow<'_, [DateFallbackCandidate]>> {
        match self {
            Self::Preset(DateFallbackRulePreset::None) => None,
            Self::Preset(DateFallbackRulePreset::Standard) => {
                Some(Cow::Owned(vec![message_no_date()]))
            }
            Self::Candidates(candidates) => Some(Cow::Borrowed(candidates)),
        }
    }
}

/// An insertion-ordered type-selector map for one issued-date occurrence lane.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DateFallbackSelectorMap(IndexMap<TypeSelector, DateFallbackRule>);

#[cfg(feature = "schema")]
impl JsonSchema for DateFallbackSelectorMap {
    fn schema_name() -> Cow<'static, str> {
        "DateFallbackSelectorMap".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        crate::template::type_keyed_map_schema::<DateFallbackRule>(generator)
    }
}

impl DateFallbackSelectorMap {
    /// Construct a selector map from insertion-ordered entries.
    #[must_use]
    pub fn new(entries: IndexMap<TypeSelector, DateFallbackRule>) -> Self {
        Self(entries)
    }

    /// Return the insertion-ordered entries.
    #[must_use]
    pub fn entries(&self) -> &IndexMap<TypeSelector, DateFallbackRule> {
        &self.0
    }

    /// Resolve the first matching non-default selector, followed by `default`.
    #[must_use]
    pub fn rule_for(&self, ref_type: &str) -> Option<&DateFallbackRule> {
        self.0
            .iter()
            .find(|(selector, _)| !selector.is_default() && selector.matches(ref_type))
            .map(|(_, rule)| rule)
            .or_else(|| {
                self.0
                    .iter()
                    .find(|(selector, _)| selector.is_default())
                    .map(|(_, rule)| rule)
            })
    }

    /// Merge another selector map over this one per selector key.
    pub fn merge(&mut self, other: &Self) {
        for (selector, rule) in &other.0 {
            self.0.insert(selector.clone(), rule.clone());
        }
    }
}

/// Scalar clear accepted for a whole policy or one occurrence lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateFallbackDisabled {
    /// Clear inherited fallback behavior.
    None,
}

/// One issued-date occurrence lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateFallbackLane {
    /// Clear this lane.
    Disabled(DateFallbackDisabled),
    /// Resolve fallback by reference type.
    Selectors(DateFallbackSelectorMap),
}

impl DateFallbackLane {
    fn merge(&mut self, other: &Self) {
        match (self, other) {
            (Self::Selectors(base), Self::Selectors(overlay)) => base.merge(overlay),
            (base, overlay) => *base = overlay.clone(),
        }
    }

    fn rule_for(&self, ref_type: &str) -> Option<&DateFallbackRule> {
        match self {
            Self::Disabled(_) => None,
            Self::Selectors(selectors) => selectors.rule_for(ref_type),
        }
    }
}

/// Explicit first-issued and later-issued fallback lanes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateFallback {
    /// Policy for the first issued-date component in effective template order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_issued: Option<DateFallbackLane>,
    /// Policy for every later issued-date component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub later_issued: Option<DateFallbackLane>,
}

impl DateFallback {
    /// Resolve a rule for the requested issued-date occurrence lane and type.
    #[must_use]
    pub fn rule_for(&self, first_issued: bool, ref_type: &str) -> Option<&DateFallbackRule> {
        let lane = if first_issued {
            self.first_issued.as_ref()
        } else {
            self.later_issued.as_ref()
        };
        lane.and_then(|lane| lane.rule_for(ref_type))
    }

    /// Merge an explicit policy over this one lane by lane and selector by selector.
    pub fn merge(&mut self, other: &Self) {
        merge_lane(&mut self.first_issued, other.first_issued.as_ref());
        merge_lane(&mut self.later_issued, other.later_issued.as_ref());
    }
}

fn merge_lane(base: &mut Option<DateFallbackLane>, overlay: Option<&DateFallbackLane>) {
    let Some(overlay) = overlay else {
        return;
    };
    if let Some(base) = base {
        base.merge(overlay);
    } else {
        *base = Some(overlay.clone());
    }
}

/// Effective whole-policy state retained through inheritance and scope cascading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateFallbackConfig {
    /// Clear all inherited date fallback behavior.
    Disabled(DateFallbackDisabled),
    /// Explicit occurrence-lane policy.
    Policy(DateFallback),
}

impl DateFallbackConfig {
    /// Resolve a rule from an enabled policy.
    #[must_use]
    pub fn rule_for(&self, first_issued: bool, ref_type: &str) -> Option<&DateFallbackRule> {
        match self {
            Self::Disabled(_) => None,
            Self::Policy(policy) => policy.rule_for(first_issued, ref_type),
        }
    }

    /// Merge another whole policy over this one.
    #[must_use]
    pub fn merged(base: &Self, overlay: &Self) -> Self {
        match (base, overlay) {
            (_, Self::Disabled(_)) => overlay.clone(),
            (Self::Policy(base), Self::Policy(overlay)) => {
                let mut merged = base.clone();
                merged.merge(overlay);
                Self::Policy(merged)
            }
            (Self::Disabled(_), Self::Policy(overlay)) => Self::Policy(overlay.clone()),
        }
    }
}

/// A named or explicit whole date-fallback policy accepted from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateFallbackEntry {
    /// A named built-in policy or whole-policy clear.
    Preset(DateFallbackPreset),
    /// Explicit occurrence lanes.
    Explicit(DateFallback),
}

impl DateFallbackEntry {
    /// Expand this authored entry while retaining an explicit `none` state.
    #[must_use]
    pub fn resolve(&self) -> DateFallbackConfig {
        match self {
            Self::Preset(DateFallbackPreset::None) => {
                DateFallbackConfig::Disabled(DateFallbackDisabled::None)
            }
            Self::Preset(preset) => DateFallbackConfig::Policy(preset.config()),
            Self::Explicit(config) => DateFallbackConfig::Policy(config.clone()),
        }
    }
}

/// Built-in date-fallback policies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateFallbackPreset {
    /// Generic no-date fallback on the first issued occurrence.
    #[default]
    Standard,
    /// Shared numeric and note-base policy for GB/T 7714-2025.
    #[serde(rename = "gb-t-7714-2025")]
    GbT7714_2025,
    /// Author-date policy for GB/T 7714-2025.
    #[serde(rename = "gb-t-7714-2025-author-date")]
    GbT7714_2025AuthorDate,
    /// Clear the complete inherited date-fallback policy.
    None,
}

impl DateFallbackPreset {
    /// Expand an enabled preset to explicit occurrence lanes.
    #[must_use]
    pub fn config(self) -> DateFallback {
        match self {
            Self::Standard => standard_config(),
            Self::GbT7714_2025 => gb_t_config(),
            Self::GbT7714_2025AuthorDate => gb_t_author_date_config(),
            Self::None => DateFallback::default(),
        }
    }
}

fn selector(value: &str) -> TypeSelector {
    value.parse().unwrap_or_else(|never| match never {})
}

fn message_no_date() -> DateFallbackCandidate {
    DateFallbackCandidate::Message(DateFallbackMessage {
        message: "term.no-date".to_string(),
        form: Some(TermForm::Short),
        rendering: Rendering::default(),
    })
}

fn date_candidate(
    date: DateVariable,
    form: DateForm,
    prefix: Option<&str>,
    suffix: Option<&str>,
    brackets: bool,
) -> DateFallbackCandidate {
    DateFallbackCandidate::Date(DateFallbackDate {
        date,
        form,
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

fn publication_year_candidates(include_no_date: bool) -> Vec<DateFallbackCandidate> {
    let mut candidates = vec![
        date_candidate(
            DateVariable::Copyright,
            DateForm::Year,
            Some("c"),
            None,
            false,
        ),
        date_candidate(
            DateVariable::Printing,
            DateForm::Year,
            None,
            Some("印刷"),
            false,
        ),
        date_candidate(DateVariable::Accessed, DateForm::Year, None, None, true),
    ];
    if include_no_date {
        candidates.push(message_no_date());
    }
    candidates
}

fn selectors(entries: IndexMap<TypeSelector, DateFallbackRule>) -> DateFallbackLane {
    DateFallbackLane::Selectors(DateFallbackSelectorMap::new(entries))
}

fn standard_config() -> DateFallback {
    DateFallback {
        first_issued: Some(selectors(IndexMap::from([(
            selector("default"),
            DateFallbackRule::Preset(DateFallbackRulePreset::Standard),
        )]))),
        later_issued: None,
    }
}

fn gb_t_config() -> DateFallback {
    DateFallback {
        first_issued: Some(selectors(IndexMap::from([
            (
                selector("default"),
                DateFallbackRule::Preset(DateFallbackRulePreset::None),
            ),
            (
                selector("chapter,entry-dictionary,entry-encyclopedia"),
                DateFallbackRule::Candidates(vec![date_candidate(
                    DateVariable::Accessed,
                    DateForm::Year,
                    None,
                    None,
                    true,
                )]),
            ),
            (
                selector("book,thesis,map"),
                DateFallbackRule::Candidates(publication_year_candidates(false)),
            ),
        ]))),
        later_issued: None,
    }
}

fn gb_t_author_date_config() -> DateFallback {
    DateFallback {
        first_issued: Some(selectors(IndexMap::from([
            (
                selector("default"),
                DateFallbackRule::Preset(DateFallbackRulePreset::Standard),
            ),
            (
                selector("article-journal,article-magazine"),
                DateFallbackRule::Preset(DateFallbackRulePreset::None),
            ),
            (
                selector("webpage,post,post-weblog"),
                DateFallbackRule::Candidates(vec![
                    date_candidate(DateVariable::Accessed, DateForm::Year, None, None, true),
                    message_no_date(),
                ]),
            ),
            (
                selector("book,thesis,map"),
                DateFallbackRule::Candidates(publication_year_candidates(true)),
            ),
        ]))),
        later_issued: Some(selectors(IndexMap::from([(
            selector("manuscript,personal-communication,pamphlet"),
            DateFallbackRule::Candidates(vec![date_candidate(
                DateVariable::Accessed,
                DateForm::Full,
                None,
                None,
                true,
            )]),
        )]))),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions")]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("standard", DateFallbackPreset::Standard)]
    #[case("gb-t-7714-2025", DateFallbackPreset::GbT7714_2025)]
    #[case(
        "gb-t-7714-2025-author-date",
        DateFallbackPreset::GbT7714_2025AuthorDate
    )]
    #[case("none", DateFallbackPreset::None)]
    fn preset_names_deserialize(#[case] yaml: &str, #[case] expected: DateFallbackPreset) {
        let actual: DateFallbackPreset =
            serde_yaml::from_str(yaml).expect("preset name should parse");
        assert_eq!(actual, expected);
    }

    #[test]
    fn empty_candidate_list_is_rejected() {
        let error =
            serde_yaml::from_str::<DateFallbackRule>("[]").expect_err("empty lists must use none");
        assert!(error.to_string().contains("use `none`"));
    }

    #[test]
    fn author_date_preset_resolves_first_and_later_issued_independently() {
        let policy = DateFallbackPreset::GbT7714_2025AuthorDate.config();
        assert!(matches!(
            policy.rule_for(true, "report"),
            Some(DateFallbackRule::Preset(DateFallbackRulePreset::Standard))
        ));
        assert!(matches!(
            policy.rule_for(false, "manuscript"),
            Some(DateFallbackRule::Candidates(_))
        ));
        assert!(policy.rule_for(false, "report").is_none());
    }

    #[test]
    fn selector_merge_replaces_rules_without_reordering_keys() {
        let mut base = DateFallbackSelectorMap::new(IndexMap::from([
            (
                selector("default"),
                DateFallbackRule::Preset(DateFallbackRulePreset::Standard),
            ),
            (
                selector("book"),
                DateFallbackRule::Preset(DateFallbackRulePreset::None),
            ),
        ]));
        let overlay = DateFallbackSelectorMap::new(IndexMap::from([
            (
                selector("default"),
                DateFallbackRule::Candidates(vec![date_candidate(
                    DateVariable::Accessed,
                    DateForm::Year,
                    None,
                    None,
                    true,
                )]),
            ),
            (
                selector("report"),
                DateFallbackRule::Preset(DateFallbackRulePreset::None),
            ),
        ]));

        base.merge(&overlay);

        let keys: Vec<String> = base.entries().keys().map(ToString::to_string).collect();
        assert_eq!(keys, ["default", "book", "report"]);
        assert!(matches!(
            base.rule_for("legal-case"),
            Some(DateFallbackRule::Candidates(_))
        ));
    }
}
