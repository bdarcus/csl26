/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Options-level primary-contributor substitution policy.

use crate::locale::TermForm;
use crate::template::{Rendering, TemplateMessage};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

/// Scalar clear marker used by substitution fields.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SubstituteDisabled {
    /// Clear the inherited value.
    None,
}

/// An ordered candidate list or an explicit inherited-value clear.
#[derive(Debug, PartialEq, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum SubstituteCandidates {
    /// Clear inherited candidates.
    Disabled(SubstituteDisabled),
    /// A non-empty ordered candidate list.
    Candidates(Vec<SubstituteKey>),
}

impl<'de> Deserialize<'de> for SubstituteCandidates {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Disabled(SubstituteDisabled),
            Candidates(Vec<SubstituteKey>),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Disabled(disabled) => Ok(Self::Disabled(disabled)),
            Wire::Candidates(candidates) if candidates.is_empty() => Err(serde::de::Error::custom(
                "substitute candidate lists cannot be empty; use `none`",
            )),
            Wire::Candidates(candidates) => Ok(Self::Candidates(candidates)),
        }
    }
}

impl SubstituteCandidates {
    /// Return enabled candidates or an empty slice for `none`.
    #[must_use]
    pub fn as_slice(&self) -> &[SubstituteKey] {
        match self {
            Self::Disabled(_) => &[],
            Self::Candidates(candidates) => candidates,
        }
    }
}

/// A terminal locale message rendered after author candidates are exhausted.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SubstituteMessage {
    /// Locale message ID to render.
    pub message: String,
    /// Optional term form for a term-backed message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    /// Message-local rendering configuration.
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

impl SubstituteMessage {
    /// Convert this closed message policy for the shared component renderer.
    #[must_use]
    pub fn to_template_message(&self) -> TemplateMessage {
        TemplateMessage {
            message: self.message.clone(),
            form: self.form.clone(),
            rendering: self.rendering.clone(),
            ..TemplateMessage::default()
        }
    }
}

/// A terminal message or an explicit inherited-value clear.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum SubstituteOtherwise {
    /// Clear an inherited terminal message.
    Disabled(SubstituteDisabled),
    /// Render this message after candidates are exhausted.
    Message(SubstituteMessage),
}

impl SubstituteOtherwise {
    /// Return the enabled terminal message.
    #[must_use]
    pub fn message(&self) -> Option<&SubstituteMessage> {
        match self {
            Self::Disabled(_) => None,
            Self::Message(message) => Some(message),
        }
    }
}

/// Substitution rules for the primary contributor slot.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
#[allow(
    clippy::large_enum_variant,
    reason = "the unboxed explicit policy keeps the public config API direct"
)]
pub enum SubstituteConfig {
    /// A named preset, including the whole-policy `none` clear.
    Preset(crate::presets::SubstitutePreset),
    /// Explicit substitution configuration.
    Explicit(Substitute),
}

impl Default for SubstituteConfig {
    fn default() -> Self {
        Self::Explicit(Substitute::default())
    }
}

impl SubstituteConfig {
    /// Resolve this config to a concrete substitution policy.
    #[must_use]
    pub fn resolve(&self) -> Substitute {
        match self {
            Self::Preset(crate::presets::SubstitutePreset::None) => Substitute::default(),
            Self::Preset(preset) => preset.config(),
            Self::Explicit(config) => config.clone(),
        }
    }

    /// Resolve this config without cloning an explicit configuration.
    #[must_use]
    pub fn resolve_ref(&self) -> Cow<'_, Substitute> {
        match self {
            Self::Preset(crate::presets::SubstitutePreset::None) => {
                Cow::Owned(Substitute::default())
            }
            Self::Preset(preset) => Cow::Owned(preset.config()),
            Self::Explicit(config) => Cow::Borrowed(config),
        }
    }

    /// Resolve an optional transitional config for legacy engine consumers.
    #[must_use]
    pub fn resolve_or_default(config: Option<&Self>) -> Cow<'_, Substitute> {
        config.map_or_else(
            || Cow::Owned(Substitute::default()),
            SubstituteConfig::resolve_ref,
        )
    }

    /// Return whether this config explicitly disables the whole policy.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Preset(crate::presets::SubstitutePreset::None))
    }

    /// Merge an override config over a base config.
    #[must_use]
    pub fn merged(base: &Self, override_config: &Self) -> Self {
        if override_config.is_disabled() {
            return override_config.clone();
        }
        Self::Explicit(Substitute::merged(
            &base.resolve(),
            &override_config.resolve(),
        ))
    }
}

/// Explicit substitution configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Substitute {
    /// Form to use for contributor roles when substituting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributor_role_form: Option<String>,
    /// Optional `text-case` transform applied to the substitute role label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_role_case: Option<crate::options::titles::TextCase>,
    /// Ordered values tried after the semantic author is unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates: Option<SubstituteCandidates>,
    /// Transitional legacy candidate list retained until engine and styles migrate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template: Vec<SubstituteKey>,
    /// Type-specific primary-contributor candidate overrides.
    #[serde(
        default,
        deserialize_with = "deserialize_transitional_overrides",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub overrides: HashMap<String, SubstituteCandidates>,
    /// Per-role fallback chains for non-author contributor substitution.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub role_substitute: HashMap<String, Vec<String>>,
    /// Quoting policy for a title substituted into the author position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_quote: Option<SubstituteTitleQuoteMode>,
    /// Terminal locale message rendered after the primary chain is exhausted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otherwise: Option<SubstituteOtherwise>,
    /// Forward-compatible fields not understood by this schema version.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// Deserialize overrides while legacy styles still use an empty list as a clear.
fn deserialize_transitional_overrides<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, SubstituteCandidates>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wire {
        Disabled(SubstituteDisabled),
        Candidates(Vec<SubstituteKey>),
    }

    HashMap::<String, Wire>::deserialize(deserializer).map(|overrides| {
        overrides
            .into_iter()
            .map(|(selector, candidates)| {
                let candidates = match candidates {
                    Wire::Disabled(disabled) => SubstituteCandidates::Disabled(disabled),
                    Wire::Candidates(candidates) if candidates.is_empty() => {
                        SubstituteCandidates::Disabled(SubstituteDisabled::None)
                    }
                    Wire::Candidates(candidates) => SubstituteCandidates::Candidates(candidates),
                };
                (selector, candidates)
            })
            .collect()
    })
}

impl Default for Substitute {
    fn default() -> Self {
        Self {
            contributor_role_form: None,
            contributor_role_case: None,
            candidates: None,
            template: vec![
                SubstituteKey::Field(SubstituteField::Editor),
                SubstituteKey::Field(SubstituteField::Title),
                SubstituteKey::Field(SubstituteField::Translator),
            ],
            overrides: HashMap::new(),
            role_substitute: HashMap::new(),
            title_quote: None,
            otherwise: None,
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }
}

impl Substitute {
    /// Construct the conventional editor, title, translator policy.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            candidates: Some(SubstituteCandidates::Candidates(vec![
                SubstituteKey::Field(SubstituteField::Editor),
                SubstituteKey::Field(SubstituteField::Title),
                SubstituteKey::Field(SubstituteField::Translator),
            ])),
            ..Self::default()
        }
    }

    /// Return enabled default candidates.
    #[must_use]
    pub fn candidates(&self) -> &[SubstituteKey] {
        self.candidates
            .as_ref()
            .map_or(&[], SubstituteCandidates::as_slice)
    }

    /// Return enabled candidates for an exact type override.
    #[must_use]
    pub fn override_candidates(&self, key: &str) -> Option<&[SubstituteKey]> {
        self.overrides
            .get(key)
            .and_then(|candidates| match candidates {
                SubstituteCandidates::Disabled(_) => None,
                SubstituteCandidates::Candidates(candidates) => Some(candidates.as_slice()),
            })
    }

    /// Return the enabled terminal message.
    #[must_use]
    pub fn otherwise_message(&self) -> Option<&SubstituteMessage> {
        self.otherwise
            .as_ref()
            .and_then(SubstituteOtherwise::message)
    }

    /// Merge an override substitute config over this config.
    pub fn merge(&mut self, other: &Self) {
        if other.contributor_role_form.is_some() {
            self.contributor_role_form = other.contributor_role_form.clone();
        }
        if other.contributor_role_case.is_some() {
            self.contributor_role_case = other.contributor_role_case;
        }
        if other.candidates.is_some() {
            self.candidates = other.candidates.clone();
        }
        if !other.template.is_empty() {
            self.template = other.template.clone();
        }
        for (key, candidates) in &other.overrides {
            self.overrides.insert(key.clone(), candidates.clone());
        }
        self.role_substitute.extend(other.role_substitute.clone());
        if other.title_quote.is_some() {
            self.title_quote = other.title_quote;
        }
        if other.otherwise.is_some() {
            self.otherwise = other.otherwise.clone();
        }
        for (key, value) in &other.unknown_fields {
            self.unknown_fields.insert(key.clone(), value.clone());
        }
    }

    /// Create a merged substitute config from base and override.
    #[must_use]
    pub fn merged(base: &Self, override_config: &Self) -> Self {
        let mut result = base.clone();
        result.merge(override_config);
        result
    }
}

/// How a title used as an author substitute is quoted in citation context.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SubstituteTitleQuoteMode {
    /// Always quote the substituted title in citation context.
    Always,
    /// Resolve quoting via the normal title-category rendering machinery.
    ByCategory,
}

/// One candidate in an effective-primary substitution chain.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum SubstituteKey {
    /// A scalar field candidate such as `editor` or `title`.
    Field(SubstituteField),
    /// A scalar or merged contributor-role candidate.
    Contributor(SubstituteContributor),
}

#[allow(
    non_upper_case_globals,
    reason = "preserve the SubstituteKey constant API"
)]
impl SubstituteKey {
    /// Collection editor candidate.
    pub const CollectionEditor: Self = Self::Field(SubstituteField::CollectionEditor);
    /// Editor candidate.
    pub const Editor: Self = Self::Field(SubstituteField::Editor);
    /// Parent serial title candidate.
    pub const ParentSerial: Self = Self::Field(SubstituteField::ParentSerial);
    /// Primary title candidate.
    pub const Title: Self = Self::Field(SubstituteField::Title);
    /// Translator candidate.
    pub const Translator: Self = Self::Field(SubstituteField::Translator);
}

/// A contributor-role candidate in an effective-primary substitution chain.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SubstituteContributor {
    /// One contributor role or an ordered list of roles to promote.
    pub contributor: crate::template::ContributorRoles,
}

/// Scalar fields accepted in substitution chains.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SubstituteField {
    /// The collection editor contributor role.
    #[serde(rename = "collection-editor")]
    CollectionEditor,
    /// The editor contributor role.
    Editor,
    /// The parent serial title.
    #[serde(rename = "parent-serial")]
    ParentSerial,
    /// The primary title.
    Title,
    /// The translator contributor role.
    Translator,
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions")]
mod tests {
    use super::*;

    #[test]
    fn none_resolves_to_an_empty_policy() {
        let config: SubstituteConfig = serde_yaml::from_str("none").expect("none parses");
        assert!(config.is_disabled());
        assert!(config.resolve().candidates().is_empty());
    }

    #[test]
    fn empty_candidates_are_rejected() {
        let error = serde_yaml::from_str::<Substitute>("candidates: []")
            .expect_err("empty candidate lists must fail");
        assert!(error.to_string().contains("use `none`"));
    }

    #[test]
    fn merged_configs_preserve_role_substitute_and_apply_clears() {
        let base = SubstituteConfig::Explicit(Substitute {
            candidates: Some(SubstituteCandidates::Candidates(vec![
                SubstituteKey::Editor,
            ])),
            role_substitute: HashMap::from([(
                "container-author".to_string(),
                vec!["editor".to_string()],
            )]),
            ..Default::default()
        });
        let overlay: SubstituteConfig = serde_yaml::from_str(
            "candidates: none\notherwise:\n  message: term.anonymous\n  form: short\n",
        )
        .expect("overlay parses");

        let merged = SubstituteConfig::merged(&base, &overlay).resolve();
        assert!(merged.candidates().is_empty());
        assert!(merged.role_substitute.contains_key("container-author"));
        assert_eq!(
            merged
                .otherwise_message()
                .map(|message| message.message.as_str()),
            Some("term.anonymous")
        );
    }

    #[test]
    fn candidates_round_trip_scalar_and_merged_roles() {
        let yaml = r#"candidates:
  - editor
  - contributor: director
overrides:
  episode:
    - contributor: [writer, director]
"#;

        let parsed: Substitute = serde_yaml::from_str(yaml).expect("valid substitution yaml");
        assert!(matches!(
            parsed.candidates(),
            [
                SubstituteKey::Field(SubstituteField::Editor),
                SubstituteKey::Contributor(_)
            ]
        ));
        let serialized = serde_yaml::to_string(&parsed).expect("serializable");
        assert_eq!(
            serde_yaml::from_str::<Substitute>(&serialized).expect("round-trippable"),
            parsed
        );
    }
}
