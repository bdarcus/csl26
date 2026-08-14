/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contributor config: either a preset name or explicit configuration.
///
/// Allows styles to write `contributors: apa` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum ContributorConfigEntry {
    /// A named preset (e.g., "apa", "chicago", "vancouver", "springer").
    Preset(crate::presets::ContributorPreset),
    /// Explicit contributor configuration.
    Explicit(Box<ContributorConfig>),
}

impl Default for ContributorConfigEntry {
    fn default() -> Self {
        ContributorConfigEntry::Explicit(Box::default())
    }
}

impl ContributorConfigEntry {
    /// Resolve this entry to a concrete `ContributorConfig`.
    pub fn resolve(&self) -> ContributorConfig {
        match self {
            ContributorConfigEntry::Preset(preset) => preset.config(),
            ContributorConfigEntry::Explicit(config) => *config.clone(),
        }
    }
}

/// Contributor formatting configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct ContributorConfig {
    /// When to display a contributor's name in sort order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_as_sort: Option<DisplayAsSort>,
    /// Separator appended between initials when given names are initialized
    /// (e.g., ". " for "J. Smith").
    ///
    /// This only controls the separator string — it does **not** activate
    /// initials on its own. Set [`Self::name_form`] to
    /// [`NameForm::Initials`] to actually render initials; otherwise this
    /// field is inert and full given names are rendered regardless of its
    /// value. Defaults to ". " when `name_form: initials` is active and this
    /// is unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    /// Whether to include a hyphen when initializing names (e.g., "J.-P. Sartre").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with_hyphen: Option<bool>,
    /// Shorten the list of contributors (et al. handling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorten: Option<ShortenListOptions>,
    /// The delimiter between contributors. Defaults to the semantic `comma` mark.
    /// `None` means "not configured at this level" and will not override an inherited value.
    #[serde(
        default = "default_contributor_delimiter",
        skip_serializing_if = "is_default_contributor_delimiter"
    )]
    pub delimiter: Option<crate::template::DelimiterPunctuation>,
    /// Conjunction between last two contributors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<AndOptions>,
    /// When to include delimiter before the last contributor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_last: Option<DelimiterPrecedesLast>,
    /// Policy for applying the final delimiter to exactly two contributors.
    /// Unset resolves to [`TwoNameDelimiterPolicy::FollowRule`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_name_delimiter_policy: Option<TwoNameDelimiterPolicy>,
    /// When to include delimiter before "et al.".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_et_al: Option<DelimiterPrecedesLast>,
    /// When and how to display contributor roles. Accepts a preset name
    /// (e.g., "short-suffix") or explicit configuration.
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_role_options",
        default
    )]
    #[cfg_attr(feature = "schema", schemars(with = "Option<RoleOptionsEntry>"))]
    pub role: Option<RoleOptions>,
    /// Style-wide defaults for merged cross-role contributor candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<crate::template::ContributorMerge>,
    /// Role pairs for which an identical secondary role renders empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suppress: Vec<ContributorSuppressionRule>,
    /// Handling of non-dropping particles (e.g., "van" in "van Gogh").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demote_non_dropping_particle: Option<DemoteNonDroppingParticle>,
    /// Delimiter between family and given name when inverted (e.g., `", "` → "Smith, John").
    /// Defaults to `", "` in the engine when not specified.
    /// `None` means "not configured at this level" and will not override an inherited value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    /// How to render given names. See `NameForm` for variants.
    /// Per-scope overrides (per-mode, per-position) are expressed by setting
    /// this field in the appropriate scope's contributor config block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_form: Option<NameForm>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

impl ContributorConfig {
    /// Merge another ContributorConfig into this one.
    pub fn merge(&mut self, other: &ContributorConfig) {
        if other.display_as_sort.is_some() {
            self.display_as_sort = other.display_as_sort;
        }
        if other.initialize_with.is_some() {
            self.initialize_with = other.initialize_with.clone();
        }
        if other.initialize_with_hyphen.is_some() {
            self.initialize_with_hyphen = other.initialize_with_hyphen;
        }
        if other.shorten.is_some() {
            self.shorten = other.shorten.clone();
        }
        if other.delimiter.is_some() {
            self.delimiter = other.delimiter.clone();
        }
        if other.and.is_some() {
            self.and = other.and.clone();
        }
        if other.delimiter_precedes_last.is_some() {
            self.delimiter_precedes_last = other.delimiter_precedes_last;
        }
        if other.two_name_delimiter_policy.is_some() {
            self.two_name_delimiter_policy = other.two_name_delimiter_policy;
        }
        if other.delimiter_precedes_et_al.is_some() {
            self.delimiter_precedes_et_al = other.delimiter_precedes_et_al;
        }
        if other.role.is_some() {
            self.role = other.role.clone();
        }
        if other.merge.is_some() {
            self.merge = other.merge.clone();
        }
        if !other.suppress.is_empty() {
            self.suppress = other.suppress.clone();
        }
        if other.demote_non_dropping_particle.is_some() {
            self.demote_non_dropping_particle = other.demote_non_dropping_particle;
        }
        if other.sort_separator.is_some() {
            self.sort_separator = other.sort_separator.clone();
        }
        if other.name_form.is_some() {
            self.name_form = other.name_form;
        }
    }

    /// Return the configured rendering override for a specific contributor role.
    pub fn role_rendering(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<&RoleRendering> {
        self.role.as_ref()?.role_rendering(role)
    }

    /// Resolve the effective label preset for a specific contributor role.
    pub fn effective_role_label_preset(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<RoleLabelPreset> {
        self.role
            .as_ref()
            .and_then(|role_options| role_options.effective_label_preset(role))
    }

    /// Resolve the bibliography-context default label preset for a role.
    ///
    /// Returns the preset the configured [`RoleLabelDefaults`] bundle
    /// assigns to `role`, or `None` when no bundle is set or the bundle
    /// does not label this role.
    pub fn default_role_label_preset(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<RoleLabelPreset> {
        self.role
            .as_ref()
            .and_then(|role_options| role_options.defaults)
            .and_then(|defaults| defaults.preset_for(role))
    }

    /// Return the configured name-order override for a specific contributor role.
    pub fn effective_role_name_order(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<&crate::template::NameOrder> {
        self.role_rendering(role)
            .and_then(|rendering| rendering.name_order.as_ref())
    }

    /// Return the structural label presentation configured for a role.
    pub fn role_label_presentation(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<&RoleLabelPresentation> {
        self.role_rendering(role)
            .and_then(|rendering| rendering.label.as_ref())
    }

    /// Resolve component merge configuration over style-wide defaults.
    ///
    /// Borrows the component-level or style-wide merge block when either is
    /// present, only allocating an owned default (`ContributorMerge` carries
    /// a `HashMap`) when neither is configured.
    #[must_use]
    pub fn effective_merge<'a>(
        &'a self,
        component: Option<&'a crate::template::ContributorMerge>,
    ) -> std::borrow::Cow<'a, crate::template::ContributorMerge> {
        match component.or(self.merge.as_ref()) {
            Some(merge) => std::borrow::Cow::Borrowed(merge),
            None => std::borrow::Cow::Owned(crate::template::ContributorMerge::default()),
        }
    }
}

/// Suppress one contributor role when its people exactly match another role.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ContributorSuppressionRule {
    /// Contributor role that renders empty when the rule matches.
    pub role: crate::template::ContributorRole,
    /// Role whose non-empty person set must exactly match `role`.
    pub when_identical_to: crate::template::ContributorRole,
}

/// Named role-label presets for secondary contributor rendering.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RoleLabelPreset {
    /// Suppress the configured role label.
    None,
    /// Render the localized verb label before the names.
    VerbPrefix,
    /// Render the localized short verb label before the names.
    VerbShortPrefix,
    /// Render the localized short label after the names.
    ShortSuffix,
    /// Render the localized short label after the names, comma-joined (`, eds.`).
    ShortSuffixComma,
    /// Render the localized long label after the names.
    LongSuffix,
}

impl RoleLabelPreset {
    /// Resolve a legacy string form to the corresponding role-label preset.
    pub fn from_form_str(form: &str) -> Option<Self> {
        match form {
            "none" => Some(Self::None),
            "verb" => Some(Self::VerbPrefix),
            "verb-short" => Some(Self::VerbShortPrefix),
            "short" => Some(Self::ShortSuffix),
            "short-comma" => Some(Self::ShortSuffixComma),
            "long" => Some(Self::LongSuffix),
            _ => None,
        }
    }
}

/// Named bundles of per-role default label presets, matching a style
/// guide's documented bibliography-context convention.
///
/// Applied only where a style has no explicit `label`, no configured
/// preset, and no `role.omit` for the role, and only when rendering in
/// bibliography context — no examined style guide (APA, MLA, Chicago,
/// Vancouver/NLM) carries a role label into the in-text citation. See
/// div-012 in `docs/adjudication/DIVERGENCE_REGISTER.md`.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RoleLabelDefaults {
    /// No automatic role labels (the engine default when unset).
    None,
    /// APA-style convention: parenthesized editor, writer, and director
    /// descriptions in the bibliography primary-contributor slot.
    Apa,
    /// MLA convention: word-form comma-joined suffix (`", editor"`) for
    /// its documented role set (editor, translator, director,
    /// illustrator, interviewer).
    Mla,
}

impl RoleLabelDefaults {
    /// Resolve this bundle to the default label preset for a role, if any.
    pub fn preset_for(&self, role: &crate::template::ContributorRole) -> Option<RoleLabelPreset> {
        use crate::template::ContributorRole;
        match self {
            Self::None => None,
            Self::Apa => match role {
                ContributorRole::Editor => Some(RoleLabelPreset::ShortSuffix),
                ContributorRole::Writer | ContributorRole::Director => {
                    Some(RoleLabelPreset::LongSuffix)
                }
                _ => None,
            },
            Self::Mla => matches!(
                role,
                ContributorRole::Editor
                    | ContributorRole::Translator
                    | ContributorRole::Director
                    | ContributorRole::Illustrator
                    | ContributorRole::Interviewer
            )
            .then_some(RoleLabelPreset::LongSuffix),
        }
    }

    /// Resolve a complete bibliography label presentation supplied by this bundle.
    #[must_use]
    pub fn presentation_for(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<RoleLabelPresentation> {
        use crate::template::{ContributorRole, LabelPlacement, RoleLabelForm, WrapPunctuation};
        let (form, text_case) = match (self, role) {
            (Self::Apa, ContributorRole::Writer | ContributorRole::Director) => (
                RoleLabelForm::Long,
                Some(crate::options::titles::TextCase::Title),
            ),
            _ => return None,
        };
        Some(RoleLabelPresentation {
            form,
            placement: LabelPlacement::Suffix,
            text_case,
            wrap: Some(Box::new(WrapPunctuation::Parentheses.into())),
            prefix: None,
            suffix: None,
        })
    }
}

/// Options for demoting non-dropping particles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DemoteNonDroppingParticle {
    Never,
    SortOnly,
    #[default]
    DisplayAndSort,
}

/// When to display names in sort order (family-first).
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DisplayAsSort {
    All,
    First,
    #[default]
    None,
}

/// How to render the given-name component of a contributor name.
///
/// Controls whether full given names, family name only, or initialized
/// given names are rendered. Used to express first/subsequent mention
/// differences (Chicago) and integral/non-integral differences.
///
/// Initialization formatting details (`initialize_with`, `initialize_with_hyphen`)
/// are separate fields and only take effect when `NameForm::Initials` is active.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NameForm {
    /// Render full given names: "John D. Smith".
    #[default]
    Full,
    /// Render family name only, suppressing given names: "Smith".
    /// Used for subsequent mentions in Chicago/Turabian note styles.
    FamilyOnly,
    /// Render initialized given names using `initialize_with` separator.
    /// If `initialize_with` is None, defaults to ". " (e.g., "J. Smith").
    /// Empty string gives compact initials: "JD Smith".
    Initials,
}

/// Conjunction options between contributors.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum AndOptions {
    /// Use the localized term for "and" (e.g., "and" in English).
    Text,
    /// Use the localized symbol for "and" (e.g., "&" in English).
    Symbol,
    /// No conjunction (e.g., "Smith, Jones").
    #[default]
    None,
}

/// Role options: either a preset name or explicit configuration.
///
/// Allows styles to write `role: short-suffix` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum RoleOptionsEntry {
    /// A named preset (e.g., "short-suffix", "long-suffix").
    Preset(RoleLabelPreset),
    /// Explicit role options.
    Explicit(Box<RoleOptions>),
}

impl RoleOptionsEntry {
    /// Resolve this entry to a concrete `RoleOptions`.
    pub fn resolve(self) -> RoleOptions {
        match self {
            RoleOptionsEntry::Preset(preset) => RoleOptions {
                preset: Some(preset),
                ..Default::default()
            },
            RoleOptionsEntry::Explicit(opts) => *opts,
        }
    }
}

/// Role display options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RoleOptions {
    /// Contributor roles for which to omit the role description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omit: Vec<String>,
    /// Global role-label preset applied before legacy compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<RoleLabelPreset>,
    /// Global role label form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// Global prefix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Global suffix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Formatting for specific roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<HashMap<String, RoleRendering>>,
    /// Named bundle of bibliography-context default label presets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<RoleLabelDefaults>,
}

impl RoleOptions {
    /// Return the configured rendering override for a specific contributor role.
    pub fn role_rendering(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<&RoleRendering> {
        self.roles.as_ref()?.get(role.as_str())
    }

    /// Resolve the effective label preset for a specific contributor role.
    pub fn effective_label_preset(
        &self,
        role: &crate::template::ContributorRole,
    ) -> Option<RoleLabelPreset> {
        self.role_rendering(role)
            .and_then(|rendering| rendering.preset)
            .or(self.preset)
            .or_else(|| {
                self.form
                    .as_deref()
                    .and_then(RoleLabelPreset::from_form_str)
            })
    }
}

/// Rendering options for contributor roles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RoleRendering {
    /// Per-role label preset override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<RoleLabelPreset>,
    /// Structural label presentation for this role without repeating its term.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<RoleLabelPresentation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<crate::template::NameOrder>,
}

/// Style-wide role-label presentation independent of a contributor template.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RoleLabelPresentation {
    /// Term form used for the role label.
    #[serde(default)]
    pub form: crate::template::RoleLabelForm,
    /// Placement relative to the contributor name or name run.
    #[serde(default)]
    pub placement: crate::template::LabelPlacement,
    /// Optional case transformation for the localized role term.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_case: Option<crate::options::titles::TextCase>,
    /// Optional punctuation wrapped around the localized role term.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<Box<crate::template::WrapConfig>>,
    /// Optional outer prefix overriding the placement-derived default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Optional outer suffix overriding the placement-derived default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl RoleRendering {
    /// Convert the rendering override to a generic rendering config.
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            emph: self.emph,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone().map(Into::into),
            suffix: self.suffix.clone().map(Into::into),
            ..Default::default()
        }
    }
}

/// When to use delimiter before last contributor.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPrecedesLast {
    AfterInvertedName,
    Always,
    Never,
    #[default]
    Contextual,
}

/// Policy for applying `delimiter-precedes-last` to exactly two contributors.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TwoNameDelimiterPolicy {
    /// Apply the configured delimiter rule literally in every render context.
    #[default]
    FollowRule,
    /// Suppress the delimiter in citations or given-first name lists.
    SuppressInCitationOrGivenFirst,
}

/// Et al. / list shortening options.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct ShortenListOptions {
    /// Minimum number of names to trigger shortening.
    pub min: u8,
    /// Number of names to show when shortened.
    pub use_first: u8,
    /// Number of names to show after the ellipsis (et-al-use-last).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_last: Option<u8>,
    /// How to render "and others".
    #[serde(default)]
    pub and_others: AndOtherOptions,
    /// When to use delimiter before last name.
    #[serde(default)]
    pub delimiter_precedes_last: DelimiterPrecedesLast,
    /// Minimum number of names to trigger shortening on subsequent cites.
    /// Defaults to `min` if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_min: Option<u8>,
    /// Number of names to show when shortened on subsequent cites.
    /// Defaults to `use_first` if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_use_first: Option<u8>,
}

impl Default for ShortenListOptions {
    fn default() -> Self {
        Self {
            min: 4,
            use_first: 1,
            use_last: None,
            and_others: AndOtherOptions::default(),
            delimiter_precedes_last: DelimiterPrecedesLast::default(),
            subsequent_min: None,
            subsequent_use_first: None,
        }
    }
}

/// How to render "and others" / et al.
#[derive(Debug, Default, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AndOtherOptions {
    #[default]
    EtAl,
    Text,
}

fn default_contributor_delimiter() -> Option<crate::template::DelimiterPunctuation> {
    Some(crate::template::DelimiterPunctuation::Comma)
}

fn is_default_contributor_delimiter(value: &Option<crate::template::DelimiterPunctuation>) -> bool {
    value
        .as_ref()
        .is_some_and(|delimiter| delimiter.as_default_str() == ", ")
}

/// Deserialize role options from either a preset name or explicit config.
fn deserialize_role_options<'de, D>(deserializer: D) -> Result<Option<RoleOptions>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<RoleOptionsEntry> = Option::deserialize(deserializer)?;
    Ok(value.map(|entry| entry.resolve()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::template::ContributorRole;

    #[test]
    fn role_label_defaults_none_labels_no_role() {
        // given the `none` bundle
        // then no role receives a default label preset
        assert_eq!(
            RoleLabelDefaults::None.preset_for(&ContributorRole::Editor),
            None
        );
        assert_eq!(
            RoleLabelDefaults::None.preset_for(&ContributorRole::Translator),
            None
        );
    }

    #[test]
    fn role_label_defaults_apa_labels_primary_audiovisual_roles() {
        // given the `apa` bundle
        let apa = RoleLabelDefaults::Apa;
        // then editor and APA audiovisual primary roles receive labels
        assert_eq!(
            apa.preset_for(&ContributorRole::Editor),
            Some(RoleLabelPreset::ShortSuffix)
        );
        assert_eq!(
            apa.preset_for(&ContributorRole::Writer),
            Some(RoleLabelPreset::LongSuffix)
        );
        assert_eq!(
            apa.preset_for(&ContributorRole::Director),
            Some(RoleLabelPreset::LongSuffix)
        );
        assert_eq!(apa.preset_for(&ContributorRole::Translator), None);
        assert_eq!(apa.preset_for(&ContributorRole::Chair), None);
    }

    #[test]
    fn role_label_defaults_mla_labels_documented_role_set() {
        // given the `mla` bundle
        let mla = RoleLabelDefaults::Mla;
        // then its documented roles get the word-form comma suffix
        for role in [
            ContributorRole::Editor,
            ContributorRole::Translator,
            ContributorRole::Director,
            ContributorRole::Illustrator,
            ContributorRole::Interviewer,
        ] {
            assert_eq!(mla.preset_for(&role), Some(RoleLabelPreset::LongSuffix));
        }
        // and roles outside the set get nothing
        assert_eq!(mla.preset_for(&ContributorRole::Composer), None);
    }

    #[test]
    fn role_label_defaults_round_trips_through_yaml() {
        // given a role options block declaring a defaults bundle
        let yaml = "defaults: apa\n";
        // when deserialized and re-serialized
        let opts: RoleOptions = serde_yaml::from_str(yaml).expect("valid role options yaml");
        // then the bundle survives the round trip
        assert_eq!(opts.defaults, Some(RoleLabelDefaults::Apa));
        let out = serde_yaml::to_string(&opts).expect("serializable role options");
        assert_eq!(out, "defaults: apa\n");
    }

    #[test]
    fn style_wide_merge_and_role_presentation_round_trip() {
        let yaml = r#"role:
  defaults: apa
  roles:
    director:
      label:
        form: long
        placement: suffix
        text-case: title
        wrap: parentheses
merge:
  order: role
  labels: collective
  combine-same-person: true
"#;

        let config: ContributorConfig =
            serde_yaml::from_str(yaml).expect("valid contributor defaults");

        assert_eq!(
            config.merge.as_ref().map(|merge| merge.labels),
            Some(crate::template::ContributorLabelMode::Collective)
        );
        assert!(
            config
                .role_label_presentation(&ContributorRole::Director)
                .and_then(|label| label.wrap.as_ref())
                .is_some()
        );
        let serialized = serde_yaml::to_string(&config).expect("serializable defaults");
        assert_eq!(
            serde_yaml::from_str::<ContributorConfig>(&serialized).expect("round-trippable"),
            config
        );
    }

    #[test]
    fn contributor_config_resolves_default_role_label_preset() {
        // given a contributor config with the `mla` defaults bundle
        let config: ContributorConfig =
            serde_yaml::from_str("role:\n  defaults: mla\n").expect("valid contributor config");
        // then the bundle's per-role preset is exposed
        assert_eq!(
            config.default_role_label_preset(&ContributorRole::Translator),
            Some(RoleLabelPreset::LongSuffix)
        );
        assert_eq!(
            config.default_role_label_preset(&ContributorRole::Composer),
            None
        );
    }
}
