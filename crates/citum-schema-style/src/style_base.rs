/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style base-inheritance mechanism for named compiled-in styles.
//!
//! This module provides [`StyleBase`] — a mechanism for naming
//! well-known compiled-in styles so that a YAML file can declare
//! `extends: chicago-notes-18th` and inherit the full style, then override
//! any fields it needs at the top level of the style document.

use crate::Style;
use crate::embedded::get_embedded_style;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A named, compiled-in style base that serves as an inheritance root.
///
/// A style file declares `extends: <key>` to inherit a complete base style.
/// Any top-level fields in the file (`options`, `citation`, `bibliography`,
/// etc.) are merged over the base, with local fields taking
/// ultimate precedence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum StyleBase {
    /// Hidden Elsevier Harvard family root.
    ElsevierHarvardCore,
    /// Hidden Elsevier with-titles family root.
    ElsevierWithTitlesCore,
    /// Hidden Elsevier Vancouver family root.
    ElsevierVancouverCore,
    /// Hidden Springer Basic author-date root.
    SpringerBasicAuthorDateCore,
    /// Hidden Springer Basic brackets root.
    SpringerBasicBracketsCore,
    /// Hidden Springer Vancouver root.
    SpringerVancouverBracketsCore,
    /// Hidden Taylor & Francis Chicago root.
    TaylorAndFrancisChicagoAuthorDateCore,
    /// Hidden Taylor & Francis CSE root.
    TaylorAndFrancisCouncilOfScienceEditorsAuthorDateCore,
    /// Hidden Taylor & Francis NLM root.
    TaylorAndFrancisNationalLibraryOfMedicineCore,
    /// Hidden Chicago shortened-notes root.
    ChicagoShortenedNotesBibliographyCore,
    /// Hidden Chicago 18 shared component base (options-only; both CMOS heads extend it).
    #[serde(rename = "chicago-18-base")]
    Chicago18Base,
    /// Hidden GB/T 7714—2025 shared bibliography base.
    #[serde(rename = "gb-t-7714-2025-base")]
    GbT77142025Base,
    /// Chicago Manual of Style 18th edition — notes without bibliography.
    #[serde(rename = "chicago-notes-18th")]
    ChicagoNotes18th,
    /// Chicago Manual of Style 18th edition — author-date system.
    #[serde(rename = "chicago-author-date-18th")]
    ChicagoAuthorDate18th,
    /// Chicago Manual of Style (shortened notes and bibliography).
    #[serde(rename = "chicago-shortened-notes-bibliography")]
    ChicagoShortenedNotesBibliography,
    /// APA 7th edition — author-date system.
    #[serde(rename = "apa-7th")]
    Apa7th,
    /// Elsevier Harvard (author-date).
    ElsevierHarvard,
    /// Elsevier with Titles (numeric).
    ElsevierWithTitles,
    /// Elsevier Vancouver (numeric).
    ElsevierVancouver,
    /// Springer Basic (author-date).
    SpringerBasicAuthorDate,
    /// Springer Vancouver Brackets (numeric).
    SpringerVancouverBrackets,
    /// Springer Basic Brackets (numeric).
    SpringerBasicBrackets,
    /// American Medical Association 11th edition (numeric).
    AmericanMedicalAssociation,
    /// Institute of Electrical and Electronics Engineers (numeric).
    Ieee,
    /// Taylor & Francis Chicago author-date.
    TaylorAndFrancisChicagoAuthorDate,
    /// Taylor & Francis Council of Science Editors author-date.
    TaylorAndFrancisCouncilOfScienceEditorsAuthorDate,
    /// Taylor & Francis National Library of Medicine.
    TaylorAndFrancisNationalLibraryOfMedicine,
    /// Modern Language Association 9th edition (author-page).
    ModernLanguageAssociation,
}

impl StyleBase {
    /// Return the embedded YAML key used to look up this base.
    fn embedded_key(&self) -> &'static str {
        match self {
            StyleBase::ElsevierHarvardCore => "elsevier-harvard-core",
            StyleBase::ElsevierWithTitlesCore => "elsevier-with-titles-core",
            StyleBase::ElsevierVancouverCore => "elsevier-vancouver-core",
            StyleBase::SpringerBasicAuthorDateCore => "springer-basic-author-date-core",
            StyleBase::SpringerBasicBracketsCore => "springer-basic-brackets-core",
            StyleBase::SpringerVancouverBracketsCore => "springer-vancouver-brackets-core",
            StyleBase::TaylorAndFrancisChicagoAuthorDateCore => {
                "taylor-and-francis-chicago-author-date-core"
            }
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDateCore => {
                "taylor-and-francis-council-of-science-editors-author-date-core"
            }
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicineCore => {
                "taylor-and-francis-national-library-of-medicine-core"
            }
            StyleBase::ChicagoShortenedNotesBibliographyCore => {
                "chicago-shortened-notes-bibliography-core"
            }
            StyleBase::Chicago18Base => "chicago-18-base",
            StyleBase::GbT77142025Base => "gb-t-7714-2025-base",
            StyleBase::ChicagoNotes18th => "chicago-notes-18th",
            StyleBase::ChicagoAuthorDate18th => "chicago-author-date-18th",
            StyleBase::ChicagoShortenedNotesBibliography => "chicago-shortened-notes-bibliography",
            StyleBase::Apa7th => "apa-7th",
            StyleBase::ElsevierHarvard => "elsevier-harvard",
            StyleBase::ElsevierWithTitles => "elsevier-with-titles",
            StyleBase::ElsevierVancouver => "elsevier-vancouver",
            StyleBase::SpringerBasicAuthorDate => "springer-basic-author-date",
            StyleBase::SpringerVancouverBrackets => "springer-vancouver-brackets",
            StyleBase::SpringerBasicBrackets => "springer-basic-brackets",
            StyleBase::AmericanMedicalAssociation => "american-medical-association",
            StyleBase::Ieee => "ieee",
            StyleBase::TaylorAndFrancisChicagoAuthorDate => {
                "taylor-and-francis-chicago-author-date"
            }
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDate => {
                "taylor-and-francis-council-of-science-editors-author-date"
            }
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicine => {
                "taylor-and-francis-national-library-of-medicine"
            }
            StyleBase::ModernLanguageAssociation => "modern-language-association",
        }
    }
}

/// A reference to a base style, which can be either a named builtin base,
/// a URI (e.g., `file://...`, `@hub/...`, `https://...`, `git+https://...`),
/// or a content-addressed identifier (`cid:bafkrei...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum StyleReference {
    /// A named builtin style base.
    Base(StyleBase),
    /// A URI reference to a remote or local style.
    ///
    /// Supported schemes: `file://`, `https://`, `git+https://`, `cid:`.
    Uri(String),
}

impl StyleReference {
    /// Returns a string key identifying this reference.
    pub fn key(&self) -> &str {
        match self {
            StyleReference::Base(base) => base.key(),
            StyleReference::Uri(uri) => uri,
        }
    }

    /// Returns true when this reference is a content-addressed CID URI.
    pub fn is_cid(&self) -> bool {
        matches!(self, StyleReference::Uri(uri) if uri.starts_with("cid:"))
    }
}

impl From<StyleBase> for StyleReference {
    fn from(base: StyleBase) -> Self {
        StyleReference::Base(base)
    }
}

impl StyleBase {
    ///
    /// # Panics
    ///
    /// Panics if the embedded YAML is missing or malformed.
    #[allow(
        clippy::panic,
        reason = "Embedded styles must be valid and present at runtime"
    )]
    pub fn base(&self) -> Style {
        let key = self.embedded_key();
        get_embedded_style(key)
            .unwrap_or_else(|| panic!("StyleBase: missing embedded style for key '{key}'"))
            .unwrap_or_else(|e| panic!("StyleBase: malformed embedded YAML for key '{key}': {e}"))
    }

    /// Return the canonical base key string (kebab-case).
    pub fn key(&self) -> &'static str {
        match self {
            StyleBase::ElsevierHarvardCore => "elsevier-harvard-core",
            StyleBase::ElsevierWithTitlesCore => "elsevier-with-titles-core",
            StyleBase::ElsevierVancouverCore => "elsevier-vancouver-core",
            StyleBase::SpringerBasicAuthorDateCore => "springer-basic-author-date-core",
            StyleBase::SpringerBasicBracketsCore => "springer-basic-brackets-core",
            StyleBase::SpringerVancouverBracketsCore => "springer-vancouver-brackets-core",
            StyleBase::TaylorAndFrancisChicagoAuthorDateCore => {
                "taylor-and-francis-chicago-author-date-core"
            }
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDateCore => {
                "taylor-and-francis-council-of-science-editors-author-date-core"
            }
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicineCore => {
                "taylor-and-francis-national-library-of-medicine-core"
            }
            StyleBase::ChicagoShortenedNotesBibliographyCore => {
                "chicago-shortened-notes-bibliography-core"
            }
            StyleBase::Chicago18Base => "chicago-18-base",
            StyleBase::GbT77142025Base => "gb-t-7714-2025-base",
            StyleBase::ChicagoNotes18th => "chicago-notes-18th",
            StyleBase::ChicagoAuthorDate18th => "chicago-author-date-18th",
            StyleBase::ChicagoShortenedNotesBibliography => "chicago-shortened-notes-bibliography",
            StyleBase::Apa7th => "apa-7th",
            StyleBase::ElsevierHarvard => "elsevier-harvard",
            StyleBase::ElsevierWithTitles => "elsevier-with-titles",
            StyleBase::ElsevierVancouver => "elsevier-vancouver",
            StyleBase::SpringerBasicAuthorDate => "springer-basic-author-date",
            StyleBase::SpringerVancouverBrackets => "springer-vancouver-brackets",
            StyleBase::SpringerBasicBrackets => "springer-basic-brackets",
            StyleBase::AmericanMedicalAssociation => "american-medical-association",
            StyleBase::Ieee => "ieee",
            StyleBase::TaylorAndFrancisChicagoAuthorDate => {
                "taylor-and-francis-chicago-author-date"
            }
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDate => {
                "taylor-and-francis-council-of-science-editors-author-date"
            }
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicine => {
                "taylor-and-francis-national-library-of-medicine"
            }
            StyleBase::ModernLanguageAssociation => "modern-language-association",
        }
    }

    /// Return all known base variants.
    ///
    /// Prefer this over exhaustive `match` when iterating the registry, since
    /// [`StyleBase`] is `#[non_exhaustive]`.
    pub fn all() -> &'static [StyleBase] {
        &[
            StyleBase::ElsevierHarvardCore,
            StyleBase::ElsevierWithTitlesCore,
            StyleBase::ElsevierVancouverCore,
            StyleBase::SpringerBasicAuthorDateCore,
            StyleBase::SpringerBasicBracketsCore,
            StyleBase::SpringerVancouverBracketsCore,
            StyleBase::TaylorAndFrancisChicagoAuthorDateCore,
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDateCore,
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicineCore,
            StyleBase::ChicagoShortenedNotesBibliographyCore,
            StyleBase::Chicago18Base,
            StyleBase::GbT77142025Base,
            StyleBase::ChicagoNotes18th,
            StyleBase::ChicagoAuthorDate18th,
            StyleBase::ChicagoShortenedNotesBibliography,
            StyleBase::Apa7th,
            StyleBase::ElsevierHarvard,
            StyleBase::ElsevierWithTitles,
            StyleBase::ElsevierVancouver,
            StyleBase::SpringerBasicAuthorDate,
            StyleBase::SpringerVancouverBrackets,
            StyleBase::SpringerBasicBrackets,
            StyleBase::AmericanMedicalAssociation,
            StyleBase::Ieee,
            StyleBase::TaylorAndFrancisChicagoAuthorDate,
            StyleBase::TaylorAndFrancisCouncilOfScienceEditorsAuthorDate,
            StyleBase::TaylorAndFrancisNationalLibraryOfMedicine,
            StyleBase::ModernLanguageAssociation,
        ]
    }

    /// Internal resolver with loop protection that preserves profile errors.
    pub(crate) fn try_resolve_with_visited(
        &self,
        resolver: Option<&crate::StyleResolver>,
        visited: &mut HashSet<String>,
    ) -> Result<Style, crate::ResolutionError> {
        self.base()
            .try_into_resolved_recursive_with(resolver, visited)
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
mod tests {
    use super::*;
    use crate::options::{Config, PageRangeFormat};
    use crate::{Style, StyleInfo, TemplateVariant};

    #[test]
    fn style_base_chicago_notes_base_is_valid() {
        let style = StyleBase::ChicagoNotes18th.base();
        let yaml = serde_yaml::to_string(&style).expect("serialization failed");
        let back: Style = serde_yaml::from_str(&yaml).expect("deserialization failed");
        assert!(back.info.title.is_some(), "title should be present");
        assert!(
            back.citation.is_some(),
            "Chicago Notes 18th citation should be present"
        );
    }

    #[test]
    fn style_base_chicago_author_date_base_is_valid() {
        let style = StyleBase::ChicagoAuthorDate18th.base();
        assert!(style.info.title.is_some(), "title should be present");
    }

    #[test]
    fn style_base_apa_7th_base_is_valid() {
        let style = StyleBase::Apa7th.base();
        assert!(style.info.title.is_some(), "title should be present");
        assert!(
            style.extends.is_none(),
            "apa-7th is a Tier-1 base and must not extend anything"
        );
        let citation = style.citation.as_ref().expect("citation should be present");
        assert!(
            citation.template_ref.is_none(),
            "APA base should carry authored citation templates"
        );
        assert!(
            citation.template.is_none(),
            "APA base should not define a top-level citation template"
        );
        assert!(
            citation
                .integral
                .as_ref()
                .is_some_and(|i| i.template.is_some()),
            "APA base should define an authored integral citation template"
        );
        assert!(
            citation
                .non_integral
                .as_ref()
                .is_some_and(|ni| ni.template.is_some()),
            "APA base should define an authored non-integral citation template"
        );

        let bibliography = style
            .bibliography
            .as_ref()
            .expect("bibliography should be present");
        assert!(
            bibliography.template_ref.is_none(),
            "APA base should carry authored bibliography templates"
        );
        assert!(
            bibliography.template.is_some(),
            "APA base should define an authored bibliography template"
        );
        assert!(
            bibliography
                .type_variants
                .as_ref()
                .is_some_and(|variants| !variants.is_empty()),
            "APA base should define authored bibliography type variants"
        );
    }

    #[test]
    fn style_base_yaml_roundtrip() {
        let yaml = "chicago-notes-18th";
        let base: StyleBase = serde_yaml::from_str(yaml).expect("deserialization failed");
        assert_eq!(base, StyleBase::ChicagoNotes18th);

        let back = serde_yaml::to_string(&base).expect("serialization failed");
        assert_eq!(back.trim(), "chicago-notes-18th");
    }

    #[test]
    fn top_level_null_field_clears_inherited_base_value() {
        // A style that inherits Chicago Notes but disables ibid via a
        // top-level citation block — the canonical authoring pattern
        // since there is no separate variant layer.
        let yaml = r#"
extends: chicago-notes-18th
citation:
  ibid: ~
"#;
        let style: Style = Style::from_yaml_str(yaml).expect("style parses");
        let resolved = style.into_resolved();
        assert!(
            resolved
                .citation
                .as_ref()
                .expect("citation present")
                .ibid
                .is_none(),
            "top-level null should clear inherited ibid"
        );
        assert!(
            resolved.citation.as_ref().unwrap().template.is_some(),
            "top-level override should preserve the inherited template"
        );
    }

    #[test]
    fn local_style_overrides_merge_with_base() {
        let style = Style {
            info: StyleInfo {
                title: Some("Taylor & Francis Test".to_string()),
                id: Some("tf-test".into()),
                ..Default::default()
            },
            extends: Some(StyleBase::ChicagoAuthorDate18th.into()),
            options: Some(Config {
                page_range_format: Some(PageRangeFormat::Expanded),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = style.into_resolved();
        let options = resolved
            .options
            .expect("resolved options should be present");
        assert_eq!(options.page_range_format, Some(PageRangeFormat::Expanded));
        assert!(
            options.processing.is_some(),
            "local override should preserve inherited processing"
        );
        assert!(
            resolved.citation.is_some(),
            "local override should preserve inherited citation spec"
        );
    }

    #[test]
    fn style_base_resolution_materializes_template_v3_variants() {
        let mut visited = HashSet::new();
        let resolved = StyleBase::Ieee
            .try_resolve_with_visited(None, &mut visited)
            .expect("ieee base resolves");
        let variants = resolved
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .expect("ieee bibliography variants resolve");

        assert!(
            variants
                .values()
                .all(|variant| matches!(variant, TemplateVariant::Full(_)))
        );
    }

    #[test]
    fn style_base_circular_dependency_is_handled() {
        let mut base = StyleBase::ChicagoNotes18th.base();
        base.extends = Some(StyleBase::ChicagoNotes18th.into());

        let _ = base.try_into_resolved();
    }

    #[test]
    fn all_bases_resolve_cleanly() {
        for base in StyleBase::all() {
            let resolved = base.base().into_resolved();
            // Component bases (key ends in `-base`) are never rendered directly:
            // they carry shared family configuration for descendants and
            // legitimately have no citation grammar of their own.
            if !base.key().ends_with("-base") {
                assert!(
                    resolved.citation.is_some(),
                    "{} resolved citation missing",
                    base.key()
                );
            }
            assert!(
                resolved.options.is_some(),
                "{} resolved options missing",
                base.key()
            );
        }
    }

    #[test]
    fn chicago_18_base_carries_shared_component_options() {
        use crate::options::{DateRangeFormat, PageRangeFormat};

        let resolved = StyleBase::Chicago18Base.base().into_resolved();
        let options = resolved.options.expect("chicago-18-base has options");
        assert_eq!(
            options.page_range_format,
            Some(PageRangeFormat::Chicago16),
            "shared page-range-format must be present"
        );
        assert!(matches!(
            options.dates.as_ref().map(|dates| &dates.range_format),
            Some(DateRangeFormat::Chicago | DateRangeFormat::Expanded)
        ));
        assert!(
            options.punctuation_in_quote,
            "shared punctuation-in-quote must be present"
        );
        assert!(
            options.multilingual.is_some(),
            "shared multilingual policy must be present"
        );
        assert!(
            options
                .contributors
                .as_ref()
                .and_then(|c| c.demote_non_dropping_particle)
                .is_some(),
            "shared demote-non-dropping-particle must be present"
        );
        // A component base must not carry a citation grammar.
        assert!(
            resolved.citation.is_none(),
            "chicago-18-base must not define a citation grammar"
        );
    }

    #[test]
    fn tier1_bases_have_no_extends_field() {
        // Tier-1 base styles must not contain an extends: field — they ARE the root.
        // Profile styles (Tier-2) in StyleBase may legitimately extend a base.
        // The two CMOS heads are now Tier-2: they extend the shared
        // `chicago-18-base` component root (which is itself Tier-1).
        let tier1 = [
            StyleBase::Apa7th,
            StyleBase::Chicago18Base,
            StyleBase::GbT77142025Base,
            StyleBase::Ieee,
            StyleBase::AmericanMedicalAssociation,
            StyleBase::ModernLanguageAssociation,
        ];
        for base in &tier1 {
            assert!(
                base.base().extends.is_none(),
                "{} is a Tier-1 base and must not have an extends: field",
                base.key()
            );
        }
    }

    #[test]
    fn turabian_pattern_disables_ibid_via_top_level_citation() {
        // Turabian 9th ed. = Chicago Notes + ibid disabled.
        // With no variant layer, this is expressed as a top-level citation override.
        let yaml = r#"
info:
  title: "Turabian 9th"
extends: chicago-notes-18th
citation:
  ibid: ~
"#;
        let style = Style::from_yaml_str(yaml).expect("style parses");
        let resolved = style.into_resolved();
        let citation = resolved.citation.expect("citation should be present");
        assert!(citation.ibid.is_none(), "ibid should be disabled");
        assert!(
            citation.template.is_some(),
            "inherited template should be preserved"
        );
    }
}
