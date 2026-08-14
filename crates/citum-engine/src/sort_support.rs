/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Shared bibliography sort-key normalization and collation helpers.

use std::cmp::Ordering;

use crate::reference::{FlatName, Reference};
use citum_schema::grouping::NameSortOrder;
use citum_schema::locale::Locale;
use citum_schema::options::{Config, SortingLocale, SortingMultilingualMode};
use citum_schema::reference::contributor::{Contributor, MultilingualName, StructuredName};
use citum_schema::reference::types::{MultilingualComplex, MultilingualString, Title};

#[cfg(feature = "icu")]
use icu_collator::options::{AlternateHandling, CaseLevel, CollatorOptions, Strength};
#[cfg(feature = "icu")]
use icu_collator::{CollatorBorrowed, CollatorPreferences};
#[cfg(feature = "icu")]
use icu_locale::Locale as IcuLocale;

/// Locale-aware comparator used by bibliography sorting paths.
pub(crate) struct TextCollator {
    #[cfg(feature = "icu")]
    collator: CollatorBorrowed<'static>,
}

impl TextCollator {
    /// Create a collator for the active Citum locale.
    ///
    /// Configures the collator with:
    /// - Secondary strength (base letters + accents, no case distinction)
    /// - Case level off (case-insensitive via collator, not preprocessing)
    /// - Alternate handling shifted (punctuation/spaces ignorable at primary/secondary)
    #[must_use]
    pub(crate) fn new(locale: &Locale) -> Self {
        Self::new_for_locale_id(&locale.locale)
    }

    /// Create a collator for a locale identifier.
    #[must_use]
    pub(crate) fn new_for_locale_id(locale_id: &str) -> Self {
        #[cfg(feature = "icu")]
        {
            let mut options = CollatorOptions::default();
            options.strength = Some(Strength::Secondary);
            options.case_level = Some(CaseLevel::Off);
            options.alternate_handling = Some(AlternateHandling::Shifted);
            // Note: Numeric ordering and script reordering are not explicitly
            // configurable at the ICU4X collator API level; they follow CLDR
            // defaults for the resolved locale.
            #[allow(clippy::expect_used, reason = "ICU bootstrap failure is fatal")]
            let collator = CollatorBorrowed::try_new(collator_preferences(locale_id), options)
                .expect("ICU4X compiled collation data should be available");
            Self { collator }
        }
        #[cfg(not(feature = "icu"))]
        {
            let _ = locale_id;
            Self {}
        }
    }

    /// Compare two already-normalized sort keys.
    #[must_use]
    pub(crate) fn compare(&self, left: &str, right: &str) -> Ordering {
        #[cfg(feature = "icu")]
        {
            self.collator.compare(left, right)
        }
        #[cfg(not(feature = "icu"))]
        {
            left.cmp(right)
        }
    }
}

/// Sort-key construction options for bibliography text keys.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct SortKeyOptions {
    mode: SortingMultilingualMode,
    preferred_transliteration: Option<Vec<String>>,
    preferred_script: Option<String>,
}

impl SortKeyOptions {
    /// Build sort-key options from effective bibliography configuration.
    #[must_use]
    pub(crate) fn from_config(config: &Config) -> Self {
        let mode = config
            .sorting
            .as_ref()
            .map_or(SortingMultilingualMode::Uniform, |sorting| {
                sorting.effective_multilingual()
            });
        let preferred_transliteration = config
            .multilingual
            .as_ref()
            .and_then(|ml| ml.preferred_transliteration.clone());
        let preferred_script = config
            .multilingual
            .as_ref()
            .and_then(|ml| ml.preferred_script.clone())
            .or_else(|| (mode == SortingMultilingualMode::Romanized).then(|| "Latn".to_string()));

        Self {
            mode,
            preferred_transliteration,
            preferred_script,
        }
    }

    /// Return uniform sort-key behavior.
    #[must_use]
    pub(crate) fn uniform() -> Self {
        Self::default()
    }

    /// Return whether romanized hidden keys should be considered.
    #[must_use]
    pub(crate) fn is_romanized(&self) -> bool {
        self.mode == SortingMultilingualMode::Romanized
    }

    fn preferred_transliteration(&self) -> Option<&[String]> {
        self.preferred_transliteration.as_deref()
    }

    fn preferred_script(&self) -> Option<&String> {
        self.preferred_script.as_ref()
    }
}

/// Resolve the locale ID used by the sorting collator.
#[must_use]
pub(crate) fn collator_locale_id<'a>(
    bibliography_locale: &'a Locale,
    config: &'a Config,
) -> &'a str {
    config
        .sorting
        .as_ref()
        .and_then(|sorting| sorting.locale.as_ref())
        .and_then(SortingLocale::as_explicit_tag)
        .unwrap_or(bibliography_locale.locale.as_str())
}

/// Build the normalized author sort key using existing fallback rules.
#[must_use]
pub(crate) fn author_sort_key_opt(
    reference: &Reference,
    name_order: NameSortOrder,
    locale: &Locale,
    fallback_to_title: bool,
) -> Option<String> {
    author_sort_key_opt_with_options(
        reference,
        name_order,
        locale,
        fallback_to_title,
        &SortKeyOptions::uniform(),
    )
}

/// Build the normalized author sort key using configured multilingual behavior.
#[must_use]
pub(crate) fn author_sort_key_opt_with_options(
    reference: &Reference,
    name_order: NameSortOrder,
    locale: &Locale,
    fallback_to_title: bool,
    options: &SortKeyOptions,
) -> Option<String> {
    reference
        .author()
        .and_then(|c| contributor_sort_key(&c, name_order, options))
        .filter(|key| !key.is_empty())
        .or_else(|| {
            reference
                .editor()
                .and_then(|c| contributor_sort_key(&c, name_order, options))
                .filter(|key| !key.is_empty())
        })
        .or_else(|| {
            fallback_to_title.then(|| title_sort_key_with_options(reference, locale, options))
        })
        .filter(|key| !key.is_empty())
}

/// Build a sort key from the first name in an already-resolved merged list.
#[must_use]
pub(crate) fn flat_names_sort_key(names: &[FlatName], name_order: NameSortOrder) -> Option<String> {
    let name = names.first()?;
    if let Some(literal) = non_empty_str(name.literal.as_deref()) {
        return non_empty_normalized(literal);
    }
    let family = non_empty_str(name.family.as_deref()).unwrap_or_default();
    let given = non_empty_str(name.given.as_deref()).unwrap_or_default();
    let value = match name_order {
        NameSortOrder::FamilyGiven => format!("{family}\u{0}{given}"),
        NameSortOrder::GivenFamily => format!("{given}\u{0}{family}"),
    };
    non_empty_normalized(&value)
}

/// Build the normalized title sort key with configured multilingual behavior.
#[must_use]
pub(crate) fn title_sort_key_with_options(
    reference: &Reference,
    locale: &Locale,
    options: &SortKeyOptions,
) -> String {
    let title = reference
        .title()
        .map(|title| title_sort_text(&title, options))
        .unwrap_or_default();
    normalize_sort_text(locale.strip_sort_articles(&title))
}

/// Normalize plain text for bibliography sorting.
///
/// When the `icu` feature is enabled, returns the text unchanged; the collator
/// handles case-insensitive comparison internally via `CaseLevel::Off`.
///
/// When the `icu` feature is disabled, the fallback comparison is case-sensitive.
#[must_use]
pub(crate) fn normalize_sort_text(text: &str) -> String {
    text.to_string()
}

/// Build a configured sort key directly from a resolved contributor payload.
pub(crate) fn contributor_sort_key(
    contributor: &Contributor,
    name_order: NameSortOrder,
    options: &SortKeyOptions,
) -> Option<String> {
    let key = match contributor {
        Contributor::SimpleName(name) => multilingual_string_sort_text(&name.name, options),
        Contributor::StructuredName(name) => structured_name_sort_text(name, name_order, options),
        Contributor::Multilingual(name) => multilingual_name_sort_text(name, name_order, options),
        Contributor::ContributorList(list) => list
            .0
            .first()
            .and_then(|contributor| contributor_sort_key(contributor, name_order, options))?,
    };

    non_empty_normalized(key.as_str())
}

fn multilingual_name_sort_text(
    name: &MultilingualName,
    name_order: NameSortOrder,
    options: &SortKeyOptions,
) -> String {
    if options.is_romanized() {
        if let Some(sort_as) = non_empty_str(name.sort_as.as_deref()) {
            return sort_as.to_string();
        }
        if let Some(part_key) = structured_name_sort_as_text(&name.original, name_order) {
            return part_key.to_string();
        }
        if let Some(transliterated) = select_structured_transliteration(name, options) {
            return structured_name_original_text(transliterated, name_order);
        }
    }

    structured_name_sort_text(&name.original, name_order, options)
}

fn structured_name_sort_text(
    name: &StructuredName,
    name_order: NameSortOrder,
    options: &SortKeyOptions,
) -> String {
    let family = multilingual_string_sort_text(&name.family, options);
    let given = multilingual_string_sort_text(&name.given, options);
    compose_family_given_key(&family, &given, name_order)
}

fn structured_name_original_text(name: &StructuredName, name_order: NameSortOrder) -> String {
    let family = name.family.to_string();
    let given = name.given.to_string();
    compose_family_given_key(&family, &given, name_order)
}

/// Combine a family and given name into a single composite sort key so a tie
/// on one part is broken by the other, matching [`flat_names_sort_key`].
///
/// An empty result (both parts blank) is preserved as an empty string rather
/// than a bare separator — callers rely on emptiness to fall back to another
/// sort key (e.g. title) for anonymous entries.
fn compose_family_given_key(family: &str, given: &str, name_order: NameSortOrder) -> String {
    if family.trim().is_empty() && given.trim().is_empty() {
        return String::new();
    }
    match name_order {
        NameSortOrder::FamilyGiven => format!("{family}\u{0}{given}"),
        NameSortOrder::GivenFamily => format!("{given}\u{0}{family}"),
    }
}

fn structured_name_sort_as_text(name: &StructuredName, name_order: NameSortOrder) -> Option<&str> {
    match name_order {
        NameSortOrder::FamilyGiven | NameSortOrder::GivenFamily => {
            multilingual_string_sort_as_text(&name.family)
        }
    }
}

fn title_sort_text(title: &Title, options: &SortKeyOptions) -> String {
    match title {
        Title::Multilingual(complex) => multilingual_complex_sort_text(complex, options),
        _ => title.to_string(),
    }
}

fn multilingual_string_sort_text(string: &MultilingualString, options: &SortKeyOptions) -> String {
    match string {
        MultilingualString::Simple(value) => value.clone(),
        MultilingualString::Complex(complex) => multilingual_complex_sort_text(complex, options),
    }
}

fn multilingual_complex_sort_text(
    complex: &MultilingualComplex,
    options: &SortKeyOptions,
) -> String {
    if options.is_romanized() {
        if let Some(sort_as) = non_empty_str(complex.sort_as.as_deref()) {
            return sort_as.to_string();
        }
        if let Some(transliteration) = resolve_transliteration(&complex.transliterations, options) {
            return transliteration.to_string();
        }
    }

    complex.original.clone()
}

fn multilingual_string_sort_as_text(string: &MultilingualString) -> Option<&str> {
    match string {
        MultilingualString::Complex(complex) => non_empty_str(complex.sort_as.as_deref()),
        MultilingualString::Simple(_) => None,
    }
}

fn select_structured_transliteration<'a>(
    name: &'a MultilingualName,
    options: &SortKeyOptions,
) -> Option<&'a StructuredName> {
    crate::values::resolve_preferred_variant(
        &name.transliterations,
        options.preferred_transliteration(),
        options.preferred_script(),
    )
}

fn resolve_transliteration<'a>(
    transliterations: &'a std::collections::HashMap<String, String>,
    options: &SortKeyOptions,
) -> Option<&'a str> {
    crate::values::resolve_preferred_variant(
        transliterations,
        options.preferred_transliteration(),
        options.preferred_script(),
    )
    .map(String::as_str)
    .and_then(|value| non_empty_str(Some(value)))
}

fn non_empty_normalized(value: &str) -> Option<String> {
    non_empty_str(Some(value)).map(normalize_sort_text)
}

fn non_empty_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(feature = "icu")]
fn collator_preferences(locale_id: &str) -> CollatorPreferences {
    parse_icu_locale(locale_id)
        .unwrap_or_else(default_icu_locale)
        .into()
}

#[cfg(feature = "icu")]
fn parse_icu_locale(locale_id: &str) -> Option<IcuLocale> {
    let mut candidate = locale_id.trim();
    while !candidate.is_empty() {
        if let Ok(locale) = candidate.parse::<IcuLocale>() {
            return Some(locale);
        }
        match candidate.rsplit_once('-') {
            Some((prefix, _)) => candidate = prefix,
            None => break,
        }
    }
    None
}

#[cfg(feature = "icu")]
fn default_icu_locale() -> IcuLocale {
    #[allow(clippy::expect_used, reason = "ICU bootstrap failure is fatal")]
    "en-US"
        .parse::<IcuLocale>()
        .expect("en-US should always be a valid ICU locale")
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

    #[test]
    #[cfg(feature = "icu")]
    fn test_parse_icu_locale_trims_unparseable_override_suffix() {
        let parsed = parse_icu_locale("de-DE-foo_bar")
            .expect("fallback parsing should produce a base locale");
        assert_eq!(parsed.to_string(), "de-DE");
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_sorts_accented_names_near_ascii_peers() {
        let collator = TextCollator::new(&Locale::en_us());
        assert_eq!(collator.compare("celik", "çelik"), Ordering::Less);
        assert_eq!(collator.compare("ó tuathail", "zukin"), Ordering::Less);
    }

    #[test]
    fn test_normalize_sort_text_preserves_locale_sensitive_case_points() {
        assert_eq!(normalize_sort_text("İnce"), "İnce");
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_is_case_insensitive() {
        let collator = TextCollator::new(&Locale::en_us());
        // "smith" and "Smith" should compare equal at primary/secondary levels
        assert_eq!(collator.compare("smith", "Smith"), Ordering::Equal);
        assert_eq!(collator.compare("Jones", "jones"), Ordering::Equal);
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_nfc_nfd_equivalence() {
        let collator = TextCollator::new(&Locale::en_us());
        // é as single codepoint (NFC) vs e + combining acute (NFD) should compare equal
        let nfc = "café"; // é as U+00E9
        let nfd = "cafe\u{0301}"; // e + U+0301 combining acute
        assert_eq!(collator.compare(nfc, nfd), Ordering::Equal);
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_hangul_latin_consistent_order() {
        let collator = TextCollator::new(&Locale::en_us());
        // Under en-US tailored collator, these should have a consistent order.
        // Hangul block (U+AC00 onwards) sorts after Latin-1 Supplement.
        let latin = "Smith";
        let hangul = "김"; // Hangul syllable "Kim"
        // Just verify consistent comparison (both directions give opposite results)
        let fwd = collator.compare(latin, hangul);
        let rev = collator.compare(hangul, latin);
        assert_ne!(fwd, rev); // One is Less, the other is Greater
        assert_eq!(fwd.reverse(), rev); // Reverse of Less is Greater
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_arabic_latin_consistent_order() {
        let collator = TextCollator::new(&Locale::en_us());
        // Under en-US tailored collator, Arabic script sorts consistently.
        let latin = "Smith";
        let arabic = "محمد"; // Arabic "Muhammad"
        let fwd = collator.compare(latin, arabic);
        let rev = collator.compare(arabic, latin);
        assert_ne!(fwd, rev);
        assert_eq!(fwd.reverse(), rev);
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_text_collator_punctuation_ignorable() {
        let collator = TextCollator::new(&Locale::en_us());
        // With AlternateHandling::Shifted, punctuation and spaces should be ignorable
        // at primary/secondary levels, so names with and without apostrophes/hyphens compare equal.
        assert_eq!(collator.compare("O'Brien", "Obrien"), Ordering::Equal);
        assert_eq!(collator.compare("al-Rashid", "alRashid"), Ordering::Equal);
    }

    /// `compose_family_given_key` must break family-name ties with the given
    /// name (in the order `NameSortOrder` requests), and must not fabricate
    /// a non-empty key from a bare separator when both parts are blank —
    /// callers depend on emptiness to fall back to another sort key.
    #[rstest::rstest]
    #[case::family_given_order("Johnson", "Alice", NameSortOrder::FamilyGiven, "Johnson\u{0}Alice")]
    #[case::given_family_order("Johnson", "Alice", NameSortOrder::GivenFamily, "Alice\u{0}Johnson")]
    #[case::blank_given_still_keys_on_family(
        "Johnson",
        "",
        NameSortOrder::FamilyGiven,
        "Johnson\u{0}"
    )]
    fn given_family_and_given_names_when_composing_sort_key_then_order_matches_name_order(
        #[case] family: &str,
        #[case] given: &str,
        #[case] name_order: NameSortOrder,
        #[case] expected: &str,
    ) {
        assert_eq!(
            compose_family_given_key(family, given, name_order),
            expected
        );
    }

    #[test]
    fn given_no_family_or_given_name_when_composing_sort_key_then_result_is_empty() {
        // then: an empty result (not a bare "\0" separator) so callers can
        // fall back to another sort key (e.g. title) for anonymous entries.
        assert_eq!(
            compose_family_given_key("", "", NameSortOrder::FamilyGiven),
            ""
        );
        assert_eq!(
            compose_family_given_key("  ", "\t", NameSortOrder::FamilyGiven),
            ""
        );
    }

    /// Same family name, different given names: the composite key must order
    /// the pair by given name so a same-surname collision (e.g. two Johnson
    /// works) breaks the tie by given name rather than falling through to an
    /// unrelated key like title.
    #[test]
    fn given_two_names_sharing_a_family_when_composing_sort_keys_then_given_name_breaks_the_tie() {
        let alice = compose_family_given_key("Johnson", "Alice", NameSortOrder::FamilyGiven);
        let brian = compose_family_given_key("Johnson", "Brian", NameSortOrder::FamilyGiven);
        assert!(alice < brian, "expected {alice:?} < {brian:?}");
    }
}
