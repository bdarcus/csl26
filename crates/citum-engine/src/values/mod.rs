/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Value extraction for template components.
//!
//! This module provides the logic to extract formatted values from references
//! based on template component specifications.

/// Contributor extraction and name-formatting helpers.
pub mod contributor;
/// Date extraction and date-formatting helpers.
pub mod date;
/// Supplementary standardized identifier extraction.
pub mod identifier;
/// List-component value extraction helpers.
pub mod list;
/// Locator rendering logic.
pub mod locator;
/// Locale message component rendering.
pub mod message;
/// Numeric variable extraction and page-range helpers.
pub mod number;
/// Shared helpers for collapsing consecutive numeric or ordinal numbering.
pub mod range;
/// Locale term resolution helpers.
pub mod term;
/// Title text-case transform functions.
pub mod text_case;
/// Title extraction and title-formatting helpers.
pub mod title;
/// Single source of truth for reference-type classification (title
/// category, `TypeClass` membership, serial-parent-ness, selector aliases,
/// DOI-URL synthesis).
pub(crate) mod type_class;
/// `type-label` component rendering (localized reference-type description).
pub mod type_label;
/// Generic variable extraction helpers.
pub mod variable;

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
mod tests;

use crate::reference::Reference;
use citum_schema::locale::Locale;
use citum_schema::options::{Config, bibliography::BibliographyConfig};
use citum_schema::reference::types::Title;
use citum_schema::template::{TemplateComponent, TitleType};
use std::sync::Arc;

thread_local! {
    static EXTENDED_LOCALE_EXPANDER: icu_locale::LocaleExpander =
        const { icu_locale::LocaleExpander::new_extended() };
}

pub use contributor::format_contributors_short;
pub use date::int_to_letter;

/// Return whether a template group's field-presence condition matches a
/// reference. Rendering and semantic consumers share this predicate so they
/// traverse the same effective template branches.
pub(crate) fn group_condition_matches(
    reference: &Reference,
    condition: &citum_schema::template::TemplateGroupCondition,
) -> bool {
    condition
        .field_present
        .as_ref()
        .is_none_or(|field| condition_field_present(reference, field))
        && condition
            .field_absent
            .as_ref()
            .is_none_or(|field| !condition_field_present(reference, field))
}

fn condition_field_present(
    reference: &Reference,
    field: &citum_schema::template::TemplateConditionField,
) -> bool {
    use citum_schema::template::TemplateConditionField;

    match field {
        TemplateConditionField::Author => reference.author().is_some(),
        TemplateConditionField::Editor => reference.editor().is_some(),
        TemplateConditionField::Recipient => reference
            .contributor(citum_schema::reference::ContributorRole::Recipient)
            .is_some(),
        TemplateConditionField::Translator => reference.translator().is_some(),
        TemplateConditionField::Title => reference.title().is_some(),
        TemplateConditionField::CollectionTitle => reference.collection_title().is_some(),
        TemplateConditionField::Issued => reference.effective_issued_date().is_some(),
        TemplateConditionField::OriginalPublished => reference.original_date().is_some(),
        TemplateConditionField::Publisher => reference.publisher_str().is_some(),
        TemplateConditionField::OriginalPublisher => reference.original_publisher_str().is_some(),
        TemplateConditionField::OriginalPublisherPlace => {
            reference.original_publisher_place().is_some()
        }
        TemplateConditionField::OriginalTitle => reference.original_title().is_some(),
        TemplateConditionField::Doi => reference.doi().is_some(),
        TemplateConditionField::Genre => reference.genre().is_some(),
        TemplateConditionField::Archive => reference.archive().is_some(),
        TemplateConditionField::ArchiveLocation => reference.archive_location().is_some(),
        TemplateConditionField::VolumeOrIssue => {
            reference.volume().is_some() || reference.issue().is_some()
        }
    }
}

/// Resolve the preferred variant from a map keyed by BCP 47 tag or script code.
///
/// Applies priority-based matching:
/// 1. Preferred transliteration list: exact match
/// 2. Preferred transliteration list: substring match
/// 3. Preferred script: exact match
/// 4. Preferred script: substring match
///
/// Substring passes select the lexicographically smallest matching key:
/// HashMap iteration order is randomized per process, and both rendering and
/// bibliography sorting need a reproducible choice when several keys match.
pub(crate) fn resolve_preferred_variant<'a, T>(
    variants: &'a std::collections::HashMap<String, T>,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
) -> Option<&'a T> {
    let substring_match = |needle: &str| {
        variants
            .iter()
            .filter(|(key, _)| key.contains(needle))
            .min_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, value)| value)
    };

    if let Some(tags) = preferred_transliteration {
        for tag in tags {
            if let Some(value) = variants.get(tag) {
                return Some(value);
            }
        }
        for tag in tags {
            if let Some(value) = substring_match(tag) {
                return Some(value);
            }
        }
    }

    if let Some(script) = preferred_script {
        if let Some(value) = variants.get(script) {
            return Some(value);
        }
        if let Some(value) = substring_match(script) {
            return Some(value);
        }
    }

    None
}

/// Resolve preferred transliteration from a map of transliterations.
fn resolve_transliteration<'a>(
    transliterations: &'a std::collections::HashMap<String, String>,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
) -> Option<&'a str> {
    resolve_preferred_variant(
        transliterations,
        preferred_transliteration,
        preferred_script,
    )
    .map(String::as_str)
}

fn resolve_translation<'a>(
    translations: &'a std::collections::HashMap<citum_schema::reference::LangID, String>,
    style_locale: &str,
) -> Option<&'a str> {
    translations
        .get(style_locale)
        .or_else(|| {
            style_locale
                .split(['-', '_'])
                .next()
                .and_then(|base| translations.get(base))
        })
        .map(String::as_str)
}

/// Resolve a multilingual string based on style configuration.
///
/// Applies BCP 47 fallback logic:
/// 1. Exact tag match (e.g., "ja-Latn-hepburn")
/// 2. Script prefix match (e.g., "ja-Latn")
/// 3. Fallback to original field
///
/// # Arguments
/// * `string` - The multilingual string to resolve
/// * `mode` - The rendering mode from style config
/// * `preferred_transliteration` - Optional ordered list of BCP 47 transliteration tags
/// * `preferred_script` - Optional preferred script (e.g., "Latn")
/// * `style_locale` - The style's locale for translation matching
#[must_use]
pub fn resolve_multilingual_string(
    string: &citum_schema::reference::types::MultilingualString,
    mode: Option<&citum_schema::options::MultilingualMode>,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
    style_locale: &str,
) -> String {
    use citum_schema::options::MultilingualMode;
    use citum_schema::reference::types::MultilingualString;

    match string {
        MultilingualString::Simple(s) => s.clone(),
        MultilingualString::Complex(complex) => {
            let mode = mode.unwrap_or(&MultilingualMode::Primary);

            match mode {
                MultilingualMode::Primary => complex.original.clone(),

                MultilingualMode::Transliterated => {
                    if let Some(trans) = resolve_transliteration(
                        &complex.transliterations,
                        preferred_transliteration,
                        preferred_script,
                    ) {
                        return trans.to_string();
                    }

                    // Fallback: use any available transliteration, or original
                    complex
                        .transliterations
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| complex.original.clone())
                }

                MultilingualMode::Translated => {
                    // Try to match style locale
                    resolve_translation(&complex.translations, style_locale)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| complex.original.clone())
                }

                MultilingualMode::Combined => {
                    // Format: "transliterated [translated]" or fallback variants
                    let trans = resolve_transliteration(
                        &complex.transliterations,
                        preferred_transliteration,
                        preferred_script,
                    );

                    let translation = resolve_translation(&complex.translations, style_locale);

                    match (trans, translation) {
                        (Some(t), Some(tr)) => format!("{t} [{tr}]"),
                        (Some(t), None) => t.to_string(),
                        (None, Some(tr)) => format!("{} [{}]", complex.original, tr),
                        (None, None) => complex.original.clone(),
                    }
                }

                MultilingualMode::Pattern(segments) => resolve_multilingual_pattern(
                    segments,
                    &complex.original,
                    &complex.transliterations,
                    &complex.translations,
                    preferred_transliteration,
                    preferred_script,
                    style_locale,
                ),
            }
        }
    }
}

/// Render a [`MultilingualMode::Pattern`] against a complex multilingual string.
///
/// Each segment is resolved to its text; segments that are empty or identical to
/// the previous non-empty segment are skipped (dedup).  The surviving segments are
/// joined by a single space.
fn resolve_multilingual_pattern(
    segments: &[citum_schema::options::MultilingualSegment],
    original: &str,
    transliterations: &std::collections::HashMap<String, String>,
    translations: &std::collections::HashMap<citum_schema::reference::types::LangID, String>,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
    style_locale: &str,
) -> String {
    use citum_schema::options::{MultilingualView, SegmentWrap};
    let mut parts: Vec<String> = Vec::with_capacity(segments.len());
    let mut last_text: Option<String> = None;

    for seg in segments {
        let text: Option<String> = match &seg.view {
            MultilingualView::OriginalScript => Some(original.to_string()),
            MultilingualView::Transliterated => resolve_transliteration(
                transliterations,
                preferred_transliteration,
                preferred_script,
            )
            .map(ToString::to_string),
            MultilingualView::Translated => {
                resolve_translation(translations, style_locale).map(ToString::to_string)
            }
        };

        let Some(text) = text else { continue };
        if text.is_empty() {
            continue;
        }
        // Skip duplicate: if this text is identical to the previous segment (e.g. when
        // transliteration falls back to the same value as original).
        if last_text.as_deref() == Some(text.as_str()) {
            continue;
        }

        let wrapped = match &seg.wrap {
            SegmentWrap::None => text.clone(),
            other => other.apply(&text),
        };
        last_text = Some(text);
        parts.push(wrapped);
    }

    parts.join(" ")
}

/// Resolve the effective language for one logical field scope on a reference.
///
/// This prefers an explicit `field_languages` entry, then a multilingual title
/// language tag for the provided title value, and finally the reference-level
/// language.
#[must_use]
pub fn effective_field_language(
    reference: &Reference,
    scope: &str,
    title: Option<&Title>,
) -> Option<String> {
    reference
        .field_languages()
        .get(scope)
        .map(ToString::to_string)
        .or_else(|| match title {
            Some(Title::Multilingual(multilingual)) => {
                multilingual.lang.as_ref().map(ToString::to_string)
            }
            _ => None,
        })
        .or_else(|| reference.language().map(|lang| lang.to_string()))
}

/// Resolve the effective language for the primary title of a reference.
#[must_use]
pub fn effective_item_language(reference: &Reference) -> Option<String> {
    effective_field_language(reference, "title", reference.title().as_ref())
}

/// Resolve the effective language for the specific template component being rendered.
#[must_use]
pub fn effective_component_language(
    reference: &Reference,
    component: &TemplateComponent,
) -> Option<String> {
    match component {
        TemplateComponent::Title(title_component) => {
            let title = match title_component.title {
                TitleType::Primary => reference.title(),
                TitleType::ContainerTitle => reference.container_title(),
                TitleType::ParentMonograph => reference.container_title(),
                TitleType::ParentSerial => reference.container_title(),
                TitleType::CollectionTitle => reference.collection_title(),
                _ => reference.title(),
            };

            let scope = match title_component.title {
                TitleType::Primary => "title",
                TitleType::ContainerTitle => "container-title",
                TitleType::ParentMonograph => "parent-monograph.title",
                TitleType::ParentSerial => "parent-serial.title",
                TitleType::CollectionTitle => "collection-title",
                _ => "title",
            };

            effective_field_language(reference, scope, title.as_ref())
        }
        _ => effective_item_language(reference),
    }
}

/// Parse a language tag after applying Citum's accepted normalization and fallback rules.
///
/// Underscores are normalized to BCP 47 separators, and unrecognized rightmost
/// subtags are progressively removed so Citum-specific locale suffixes do not
/// hide an otherwise valid language identifier.
pub(crate) fn parse_language_identifier(
    lang: Option<&str>,
) -> Option<icu_locale::LanguageIdentifier> {
    use std::borrow::Cow;

    let lang = lang?.trim();
    if lang.is_empty() {
        return None;
    }

    let normalized = if lang.contains('_') {
        Cow::Owned(lang.replace('_', "-"))
    } else {
        Cow::Borrowed(lang)
    };
    let mut candidate = normalized.as_ref();
    while !candidate.is_empty() {
        if let Ok(locale) = candidate.parse::<icu_locale::Locale>() {
            return Some(locale.id);
        }
        candidate = candidate.rsplit_once('-')?.0;
    }

    None
}

/// Resolve a BCP 47 language tag to its effective ISO 15924 script code.
///
/// An explicit script subtag takes precedence. Otherwise, the tag's language
/// and region are expanded with CLDR likely-subtags data. Missing, malformed,
/// private-use-only, and unrecognized language evidence resolves to `None`;
/// this function never supplies a default script.
#[must_use]
pub fn resolve_language_script(lang: Option<&str>) -> Option<String> {
    let mut langid = parse_language_identifier(lang)?;

    if let Some(script) = langid.script {
        return Some(script.to_string());
    }
    if langid.language.is_unknown() || matches!(langid.language.as_str(), "mul" | "zxx") {
        return None;
    }

    EXTENDED_LOCALE_EXPANDER.with(|expander| expander.maximize(&mut langid));
    langid.script.map(|script| script.to_string())
}

/// Whether a BCP 47 language tag resolves to the Latin script.
///
/// This compatibility adapter preserves the positive-evidence behavior used
/// by the Latin punctuation remapping call sites.
#[must_use]
pub fn is_latin_script_language(lang: Option<&str>) -> bool {
    resolve_language_script(lang).as_deref() == Some("Latn")
}

/// Script partition used to select semantic wrap-punctuation glyphs
/// (`docs/specs/PUNCTUATION_REALIZATION.md`, increment 1). Extends by adding
/// variants (`Cyrillic`, `Arabic`, …) as later increments broaden coverage;
/// v1 covers the two classes the embedded bilingual styles need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptClass {
    /// Latin-script realization: half-width delimiters (`(`, `[`, …).
    Latin,
    /// CJK-script realization: full-width delimiters (`（`, `【`, …).
    Cjk,
    /// Mixed table: full-width except periods and square brackets.
    Mixed,
}

/// Resolve the effective script class from positive script evidence only.
///
/// Returns `None` when the language tag carries no usable evidence for either
/// supported class. Latin and Cyrillic both select the narrow/Latin class;
/// unrecognized scripts have no positive evidence. Callers realizing wrap
/// punctuation should fall back to the style-declared default via
/// [`wrap_script_class`] rather than treating `None` as a script itself.
#[must_use]
pub fn script_class(lang: Option<&str>) -> Option<ScriptClass> {
    match resolve_language_script(lang).as_deref() {
        Some("Hani" | "Hans" | "Hant" | "Jpan" | "Kore" | "Hang" | "Bopo") => {
            Some(ScriptClass::Cjk)
        }
        Some("Latn" | "Cyrl") => Some(ScriptClass::Latin),
        Some(_) | None => None,
    }
}

/// Resolve the script class to realize semantic wrap punctuation as.
///
/// Per-item script evidence only overrides the style-declared `default` when
/// that default is [`ScriptClass::Cjk`] — i.e. the style has opted in via
/// `options.multilingual.realization-default: cjk`. A style that has not
/// opted in (`default` is [`ScriptClass::Latin`], which includes every
/// existing style today) realizes Latin wrap punctuation unconditionally,
/// regardless of the item's language.
///
/// This gate exists because raw item language is not the same as the
/// item's *rendered* script: a style can romanize a non-Latin source (e.g.
/// Chicago's "romanized + original-script \[translation\]" mode for East Asian
/// references) so the citation displays as Latin-script prose even though
/// the reference's `language` is `zh`/`ja`/`ko`. Without the gate, such an
/// item's CJK language evidence would incorrectly force full-width wrap
/// punctuation onto an otherwise entirely Latin-script citation. See
/// `docs/specs/PUNCTUATION_REALIZATION.md` §5.
#[must_use]
pub fn wrap_script_class(lang: Option<&str>, default: ScriptClass) -> ScriptClass {
    match default {
        ScriptClass::Latin => ScriptClass::Latin,
        ScriptClass::Cjk => script_class(lang).unwrap_or(ScriptClass::Cjk),
        ScriptClass::Mixed => ScriptClass::Mixed,
    }
}

/// Resolve the style-declared realization-default script class from
/// `options.multilingual.realization-default`, defaulting to
/// [`ScriptClass::Latin`] when unset — today's `wrap: parentheses`/`brackets`
/// behavior, byte-for-byte.
#[must_use]
pub fn realization_default_script_class(
    multilingual: Option<&citum_schema::options::MultilingualConfig>,
) -> ScriptClass {
    match multilingual.map(|ml| ml.realization_default) {
        Some(citum_schema::options::RealizationDefault::Cjk) => ScriptClass::Cjk,
        _ => ScriptClass::Latin,
    }
}

/// Resolve the effective script class and punctuation realization overrides for
/// one item.
#[must_use]
pub fn punctuation_realization_context<'a>(
    lang: Option<&str>,
    multilingual: Option<&'a citum_schema::options::MultilingualConfig>,
    locale_realization: Option<&'a citum_schema::options::PunctuationRealization>,
) -> (
    ScriptClass,
    Option<std::borrow::Cow<'a, citum_schema::options::PunctuationRealization>>,
) {
    let legacy_script = wrap_script_class(lang, realization_default_script_class(multilingual));
    let script = match multilingual.and_then(|config| config.punctuation_width) {
        Some(citum_schema::options::PunctuationWidth::Half) => ScriptClass::Latin,
        Some(citum_schema::options::PunctuationWidth::Full) => ScriptClass::Cjk,
        Some(citum_schema::options::PunctuationWidth::Mixed) => ScriptClass::Mixed,
        Some(citum_schema::options::PunctuationWidth::Bylan) => {
            wrap_script_class(lang, ScriptClass::Cjk)
        }
        None => legacy_script,
    };
    let override_script = script_class(lang).unwrap_or(match script {
        // `mixed` has no corresponding script override table; CJK owns the
        // full-width side of its default realization table.
        ScriptClass::Mixed => ScriptClass::Cjk,
        script => script,
    });
    let key = match override_script {
        ScriptClass::Latin => "latin",
        ScriptClass::Cjk => "cjk",
        ScriptClass::Mixed => "cjk",
    };
    let style_realization = multilingual
        .and_then(|config| config.scripts.get(key))
        .and_then(|script| script.realization.as_ref());
    (
        script,
        merge_punctuation_realizations(style_realization, locale_realization),
    )
}

/// Merge a style-owned table over a locale-owned table, preserving missing
/// marks for the engine's selected default realization.
fn merge_punctuation_realizations<'a>(
    style: Option<&'a citum_schema::options::PunctuationRealization>,
    locale: Option<&'a citum_schema::options::PunctuationRealization>,
) -> Option<std::borrow::Cow<'a, citum_schema::options::PunctuationRealization>> {
    match (style, locale) {
        (None, None) => None,
        (Some(style), None) => Some(std::borrow::Cow::Borrowed(style)),
        (None, Some(locale)) => Some(std::borrow::Cow::Borrowed(locale)),
        (Some(style), Some(locale)) => Some(std::borrow::Cow::Owned(
            citum_schema::options::PunctuationRealization {
                comma: style.comma.clone().or_else(|| locale.comma.clone()),
                colon: style.colon.clone().or_else(|| locale.colon.clone()),
                semicolon: style.semicolon.clone().or_else(|| locale.semicolon.clone()),
                period: style.period.clone().or_else(|| locale.period.clone()),
                parentheses: style
                    .parentheses
                    .clone()
                    .or_else(|| locale.parentheses.clone()),
                brackets: style.brackets.clone().or_else(|| locale.brackets.clone()),
            },
        )),
    }
}

/// Select a structured name from transliteration maps using priority-list then script-match rules.
fn select_by_transliteration<'a>(
    m: &'a citum_schema::reference::contributor::MultilingualName,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
) -> &'a citum_schema::reference::contributor::StructuredName {
    // 1. Priority list: exact match
    if let Some(tags) = preferred_transliteration {
        for tag in tags {
            if let Some(name) = m.transliterations.get(tag) {
                return name;
            }
        }
        // 2. Priority list: substring match
        for tag in tags {
            if let Some((_, name)) = m
                .transliterations
                .iter()
                .find(|(k, _)| k.contains(tag.as_str()))
            {
                return name;
            }
        }
    }
    // 3. Preferred script: exact match
    if let Some(script) = preferred_script {
        if let Some(name) = m.transliterations.get(script) {
            return name;
        }
        // 4. Preferred script: substring match
        if let Some((_, name)) = m
            .transliterations
            .iter()
            .find(|(tag, _)| tag.contains(script))
        {
            return name;
        }
    }
    // Fallback: any available transliteration before falling back to original
    m.transliterations.values().next().unwrap_or(&m.original)
}

/// Render the original-script display form of a structured name.
///
/// CJK names display family-first with no separator (`华林甫`); other scripts
/// display given-first with a space.
fn original_script_display(name: &citum_schema::reference::contributor::StructuredName) -> String {
    use unicode_script::{Script, UnicodeScript};

    let family = name.family.to_string();
    let given = name.given.to_string();
    let is_cjk = family.chars().chain(given.chars()).any(|ch| {
        matches!(
            ch.script(),
            Script::Han | Script::Hiragana | Script::Katakana | Script::Hangul
        )
    });
    if is_cjk || family.is_empty() || given.is_empty() {
        format!("{family}{given}")
    } else {
        format!("{given} {family}")
    }
}

/// Resolve a multilingual contributor name based on style configuration.
///
/// Uses holistic name matching - selects the entire name variant (original/transliterated/translated)
/// as a unit rather than mixing fields from different variants.
///
/// # Arguments
/// * `contributor` - The contributor to resolve
/// * `mode` - The rendering mode from style config
/// * `preferred_transliteration` - Optional ordered list of BCP 47 transliteration tags
/// * `preferred_script` - Optional preferred script (e.g., "Latn")
/// * `style_locale` - The style's locale for translation matching
#[must_use]
pub fn resolve_multilingual_name(
    contributor: &citum_schema::reference::contributor::Contributor,
    mode: Option<&citum_schema::options::MultilingualMode>,
    preferred_transliteration: Option<&[String]>,
    preferred_script: Option<&String>,
    style_locale: &str,
) -> Vec<crate::reference::FlatName> {
    use citum_schema::options::MultilingualMode;
    use citum_schema::reference::contributor::Contributor;

    match contributor {
        // Simple and structured names have no multilingual data
        Contributor::SimpleName(_) | Contributor::StructuredName(_) => contributor.to_names_vec(),

        // Multilingual names: select variant holistically
        Contributor::Multilingual(m) => {
            let mode = mode.unwrap_or(&MultilingualMode::Primary);

            let selected_name = match mode {
                MultilingualMode::Primary => &m.original,
                MultilingualMode::Transliterated => {
                    select_by_transliteration(m, preferred_transliteration, preferred_script)
                }
                MultilingualMode::Translated => {
                    m.translations.get(style_locale).unwrap_or(&m.original)
                }
                // Combined mode for names defaults to transliterated (parenthetical combo not common for names)
                MultilingualMode::Combined => {
                    select_by_transliteration(m, preferred_transliteration, preferred_script)
                }
                // Pattern mode for names: render the romanized view, carrying the
                // original-script form along when the pattern requests it
                // (e.g. "Hua Linfu 华林甫").
                MultilingualMode::Pattern(_) => {
                    select_by_transliteration(m, preferred_transliteration, preferred_script)
                }
            };

            // When a name pattern includes an `original-script` view alongside
            // the selected transliteration, carry the original-script display
            // form (with the segment's wrap applied) so formatting can append
            // it after the romanized name.
            let original_script = match mode {
                MultilingualMode::Pattern(segments) if selected_name != &m.original => segments
                    .iter()
                    .find(|segment| {
                        segment.view == citum_schema::options::MultilingualView::OriginalScript
                    })
                    .map(|segment| segment.wrap.apply(&original_script_display(&m.original))),
                _ => None,
            };

            // Convert selected name to FlatName
            vec![crate::reference::FlatName {
                given: Some(selected_name.given.to_string()),
                family: Some(selected_name.family.to_string()),
                suffix: selected_name.suffix.clone(),
                dropping_particle: selected_name.dropping_particle.clone(),
                non_dropping_particle: selected_name.non_dropping_particle.clone(),
                literal: None,
                short_name: None,
                original_script,
            }]
        }

        Contributor::ContributorList(l) => {
            l.0.iter()
                .flat_map(|c| {
                    resolve_multilingual_name(
                        c,
                        mode,
                        preferred_transliteration,
                        preferred_script,
                        style_locale,
                    )
                })
                .collect()
        }
    }
}

/// Resolve the URL for a component based on its links configuration and the reference data.
#[must_use]
pub fn resolve_url(
    links: &citum_schema::options::LinksConfig,
    reference: &Reference,
) -> Option<String> {
    use citum_schema::options::LinkTarget;

    let target = links.target.as_ref().unwrap_or(&LinkTarget::UrlOrDoi);

    let url = match target {
        LinkTarget::Url => reference.url().map(|u| u.to_string()),
        LinkTarget::Doi => reference.doi().map(|d| format!("https://doi.org/{d}")),
        LinkTarget::UrlOrDoi => reference
            .url()
            .map(|u| u.to_string())
            .or_else(|| reference.doi().map(|d| format!("https://doi.org/{d}"))),
        LinkTarget::Pubmed => reference
            .id()
            .filter(|id| id.starts_with("pmid:"))
            .map(|id| {
                #[allow(clippy::string_slice, reason = "known ASCII prefix")]
                let result = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", &id[5..]);
                result
            }),
        LinkTarget::Pmcid => reference
            .id()
            .filter(|id| id.starts_with("pmc:"))
            .map(|id| {
                #[allow(clippy::string_slice, reason = "known ASCII prefix")]
                let result = format!("https://www.ncbi.nlm.nih.gov/pmc/articles/{}/", &id[4..]);
                result
            }),
    };

    if links.strip_protocol == Some(true) {
        url.map(|u| {
            u.strip_prefix("https://")
                .or_else(|| u.strip_prefix("http://"))
                .map_or_else(|| u.clone(), ToString::to_string)
        })
    } else {
        url
    }
}

/// Resolve the effective URL for a component, checking local links then falling back to global config.
#[must_use]
pub fn resolve_effective_url(
    local_links: Option<&citum_schema::options::LinksConfig>,
    global_links: Option<&citum_schema::options::LinksConfig>,
    reference: &Reference,
    component_anchor: citum_schema::options::LinkAnchor,
) -> Option<String> {
    use citum_schema::options::LinkAnchor;

    // 1. Check local links first
    if let Some(links) = local_links {
        let anchor = links.anchor.as_ref().unwrap_or(&LinkAnchor::Component);
        if matches!(anchor, LinkAnchor::Component) || *anchor == component_anchor {
            return resolve_url(links, reference);
        }
    }

    // 2. Fall back to global links if anchor matches this component type
    if let Some(links) = global_links
        && let Some(anchor) = &links.anchor
        && *anchor == component_anchor
    {
        return resolve_url(links, reference);
    }

    None
}

/// Processed values ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct ProcValues<T = String> {
    /// The primary formatted value.
    pub value: T,
    /// Optional prefix to prepend.
    pub prefix: Option<String>,
    /// Optional suffix to append.
    pub suffix: Option<String>,
    /// Optional URL for hyperlinking.
    pub url: Option<String>,
    /// Variable key that was substituted (e.g., "title:Primary" when title replaces author).
    /// Used to prevent duplicate rendering per CSL variable-once rule.
    pub substituted_key: Option<String>,
    /// Whether the value is already pre-formatted.
    pub pre_formatted: bool,
}

/// Processing hints computed before rendering a reference or citation item.
#[derive(Debug, Clone, Default)]
pub struct ProcHints {
    /// Whether disambiguation is active (triggers year-suffix).
    pub disamb_condition: bool,
    /// Index in the disambiguation group (1-based).
    pub group_index: usize,
    /// Total size of the disambiguation group.
    pub group_length: usize,
    /// The grouping key used.
    pub group_key: String,
    /// Whether to expand given names for disambiguation.
    pub expand_given_names: bool,
    /// Whether to expand given names for primary author only.
    pub expand_given_names_primary_only: bool,
    /// When `expand_given_names` is set, whether the expansion must escalate
    /// past the style's configured given-name form (e.g. initials) to the
    /// full given name to actually resolve the collision. Set when the
    /// initials-level representation still collides (e.g. "Brandon" and
    /// "Biff" both reduce to "B."). See csl26-h9jy.
    pub expand_given_names_full: bool,
    /// Minimum number of names to show to resolve ambiguity (overrides et-al-use-first).
    pub min_names_to_show: Option<usize>,
    /// Citation number for numeric citation styles (1-based).
    pub citation_number: Option<usize>,
    /// Optional sub-label for compound numeric citation addressing (e.g., "a" in "1a").
    pub citation_sub_label: Option<String>,
    /// Citation position (first, subsequent, ibid, etc.).
    pub position: Option<citum_schema::citation::Position>,
    /// Explicit integral citation name-memory state for this rendered item.
    pub integral_name_state: Option<citum_schema::citation::IntegralNameState>,
    /// Explicit org-abbreviation state for this rendered item.
    pub org_abbreviation_state: Option<citum_schema::citation::IntegralNameState>,
    /// First note number in which this reference was cited (note styles only).
    /// Set for subsequent-position citations; `None` otherwise.
    pub first_reference_note_number: Option<u32>,
    /// When true, suppress a `disambiguate_only` title component.
    /// Set when `first_reference_note_number` is present — the note number
    /// already identifies the work; the disambiguating short title is redundant.
    pub suppress_disambiguation_title: bool,
    /// Whether the current issued-date component is the first recursively
    /// encountered issued occurrence in the effective template.
    pub date_fallback_first_issued: Option<bool>,
}

/// Context for rendering (citation vs bibliography).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderContext {
    #[default]
    /// Render values for citation output.
    Citation,
    /// Render values for bibliography output.
    Bibliography,
}

/// Options for rendering.
#[derive(Clone)]
pub struct RenderOptions<'a> {
    /// Effective configuration after style and default resolution.
    pub config: Arc<Config>,
    /// Effective bibliography-only configuration when rendering bibliography behavior.
    pub bibliography_config: Option<Arc<BibliographyConfig>>,
    /// Locale used for term lookup and locale-sensitive formatting.
    pub locale: &'a Locale,
    /// Whether the current render target is a citation or bibliography.
    pub context: RenderContext,
    /// Citation mode for the current render operation.
    pub mode: citum_schema::citation::CitationMode,
    /// Whether to suppress the author name for this citation.
    /// Set from the citation-level `suppress_author` flag.
    pub suppress_author: bool,
    /// Optional raw citation locator for rendering via locator config.
    pub locator_raw: Option<&'a citum_schema::citation::CitationLocator>,
    /// Reference type for optional type-class gating in locator patterns.
    pub ref_type: Option<String>,
    /// Whether to output semantic markup (HTML spans, Djot attributes).
    pub show_semantics: bool,
    /// The current top-level template index, when propagating preview annotations.
    pub current_template_index: Option<usize>,
    /// Document-level abbreviation map for post-render substitution.
    pub abbreviation_map: Option<&'a crate::api::AbbreviationMap>,
}

/// Trait for extracting values from template components.
pub trait ComponentValues {
    /// Resolve the component into processed render values for one reference.
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>>;
}

impl ComponentValues for TemplateComponent {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        match self {
            TemplateComponent::Contributor(c) => c.values::<F>(reference, hints, options),
            TemplateComponent::Date(d) => d.values::<F>(reference, hints, options),
            TemplateComponent::Title(t) => t.values::<F>(reference, hints, options),
            TemplateComponent::Number(n) => n.values::<F>(reference, hints, options),
            TemplateComponent::Identifier(i) => i.values::<F>(reference, hints, options),
            TemplateComponent::Variable(v) => v.values::<F>(reference, hints, options),
            TemplateComponent::Message(m) => m.values::<F>(reference, hints, options),
            TemplateComponent::Group(l) => l.values::<F>(reference, hints, options),
            TemplateComponent::Term(t) => t.values::<F>(reference, hints, options),
            TemplateComponent::TypeLabel(t) => t.values::<F>(reference, hints, options),
            _ => None,
        }
    }
}

/// Check if periods should be stripped based on three-tier precedence.
///
/// Resolution order:
/// 1. Component-level `strip_periods`
/// 2. Global config `strip_periods`
/// 3. Defaults to false
#[must_use]
pub fn should_strip_periods(
    rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
) -> bool {
    rendering
        .strip_periods
        .or(options.config.strip_periods)
        .unwrap_or(false)
}

/// Strip trailing periods from a string.
///
/// Only removes periods at the end of the string, preserves internal periods
/// (e.g., "Ph.D." remains unchanged if there's no trailing period).
#[must_use]
pub fn strip_trailing_periods(s: &str) -> String {
    s.trim_end_matches('.').to_string()
}

/// Strip every period from a string.
///
/// Matches the CSL `strip-periods` attribute's actual semantics (remove all
/// periods, not just a trailing one) — used for abbreviated journal titles
/// like "Br. Med. J." → "Br Med J", where periods can appear after every
/// abbreviated word, not only at the end.
#[must_use]
pub fn strip_all_periods(s: &str) -> String {
    s.chars().filter(|c| *c != '.').collect()
}

/// Apply abbreviation substitution if the map contains an entry for `value`.
///
/// Returns the abbreviation if found, otherwise returns the original value unchanged.
#[must_use]
pub fn apply_abbreviation(value: String, map: Option<&crate::api::AbbreviationMap>) -> String {
    if let Some(abbr) = map.and_then(|m| m.0.get(&value)) {
        return abbr.clone();
    }
    value
}
