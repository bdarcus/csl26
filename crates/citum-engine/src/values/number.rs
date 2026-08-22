/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for numeric variables (volume, issue, pages, citation numbers, etc.).
//!
//! This module handles number component rendering with support for page range formatting,
//! edition labels, and numeric citation identifiers.

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use citum_schema::locale::{DigitSystem, GeneralTerm, GrammaticalGender, MessageArgs, TermForm};
use citum_schema::reference::ClassExtension;
use citum_schema::template::{LabelForm, NumberForm, NumberVariable, TemplateNumber};

/// Resolve the raw value string for a number variable from a reference.
fn resolve_number_value(
    number: &NumberVariable,
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    show_with_locator: bool,
) -> Option<String> {
    match number {
        NumberVariable::Volume => reference.volume().map(|v| v.to_string()),
        NumberVariable::NumberOfVolumes => reference.number_of_volumes(),
        NumberVariable::Issue => reference.issue().map(|v| v.to_string()),
        NumberVariable::Pages => {
            let suppress = !show_with_locator
                && options.context == crate::values::RenderContext::Citation
                && options.locator_raw.is_some()
                && matches!(
                    options.config.processing,
                    Some(citum_schema::options::Processing::Note)
                );
            if suppress {
                None
            } else {
                reference.pages().map(|p| {
                    let delimiter =
                        options.config.page_range_delimiter.as_deref().unwrap_or(
                            options.locale.grammar_options.page_range_delimiter.as_str(),
                        );
                    format_page_range(
                        &p.to_string(),
                        options.config.page_range_format.as_ref(),
                        delimiter,
                    )
                })
            }
        }
        NumberVariable::ChapterNumber => match reference.extension() {
            ClassExtension::Statute(r) => r.chapter_number.clone(),
            _ => reference.numbering_value(&citum_schema::reference::NumberingType::Chapter),
        },
        NumberVariable::Edition => reference.edition(),
        NumberVariable::CollectionNumber => reference.collection_number(),
        NumberVariable::Number => reference.number(),
        NumberVariable::Custom(kind) => reference.numbering_value(
            &citum_schema::reference::NumberingType::Custom(kind.clone()),
        ),
        NumberVariable::DocketNumber => match reference.extension() {
            ClassExtension::Brief(r) => r.docket_number.clone(),
            _ => None,
        },
        NumberVariable::PatentNumber => match reference.extension() {
            // GB/T 7714 and most citation styles cite the filing/application
            // number (CSL `call-number`) in preference to the granted
            // number when both are known; fall back to the granted number
            // otherwise (e.g. no application number was recorded).
            ClassExtension::Patent(r) => Some(
                r.application_number
                    .clone()
                    .unwrap_or_else(|| r.patent_number.clone()),
            ),
            _ => None,
        },
        NumberVariable::StandardNumber => match reference.extension() {
            ClassExtension::Standard(r) => Some(r.standard_number.clone()),
            _ => None,
        },
        NumberVariable::ReportNumber => reference.report_number(),
        NumberVariable::PartNumber => {
            reference.numbering_value(&citum_schema::reference::NumberingType::Part)
        }
        NumberVariable::SupplementNumber => {
            reference.numbering_value(&citum_schema::reference::NumberingType::Supplement)
        }
        NumberVariable::PrintingNumber => {
            reference.numbering_value(&citum_schema::reference::NumberingType::Printing)
        }
        NumberVariable::FirstReferenceNoteNumber => {
            hints.first_reference_note_number.map(|n| n.to_string())
        }
        _ => None,
    }
}

/// Convert a component-level [`LabelForm`] to the locale's [`TermForm`] vocabulary.
fn label_form_to_term_form(label_form: &LabelForm) -> TermForm {
    match label_form {
        LabelForm::Long => TermForm::Long,
        LabelForm::Short => TermForm::Short,
        LabelForm::Symbol => TermForm::Symbol,
    }
}

/// Resolve a label prefix for a number variable if `label_form` is configured.
fn resolve_number_label<F: crate::render::format::OutputFormat<Output = String>>(
    number: &NumberVariable,
    label_form: &LabelForm,
    value: &str,
    requested_gender: Option<GrammaticalGender>,
    effective_rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> Option<String> {
    // Chicago labels only bare numeric part numbers. Exported values such as
    // `bk. 2` already carry their own label and must not become `pt. bk. 2`.
    if matches!(number, NumberVariable::PartNumber) && !is_numeric(value) {
        return None;
    }
    if let Some(locator_type) = number_var_to_locator_type(number) {
        // `number-of-volumes` is a count, so a bare value greater than one
        // selects the plural volume label. Other volume values are
        // identifiers (`vol. 2`) and retain the ordinary locator heuristic.
        let plural = if matches!(number, NumberVariable::NumberOfVolumes) {
            let Ok(count) = value.trim().parse::<u64>() else {
                // citeproc-js labels this variable only when its value is
                // numeric; values such as `3 vols. in 9 bks.` are already
                // human-readable and must remain unlabeled.
                return None;
            };
            count != 1
        } else {
            check_plural(value, &locator_type)
        };
        let term_form = label_form_to_term_form(label_form);

        options
            .locale
            .resolved_locator_term(&locator_type, plural, &term_form, requested_gender)
            .map(|t| {
                let term_str = if crate::values::should_strip_periods(effective_rendering, options)
                {
                    crate::values::strip_trailing_periods(&t)
                } else {
                    t
                };
                fmt.text(&format!("{term_str} "))
            })
    } else {
        None
    }
}

/// Maps a number variable to its corresponding general locale term, for
/// [`TemplateNumber::when_numeric`] resolution. Distinct from
/// [`number_var_to_locator_type`]: locators are for citation-position labels
/// (`p. 35`); general terms cover non-locator numbering concepts like
/// `edition` that a citation would never point a reader to.
#[must_use]
fn number_var_to_general_term(var: &NumberVariable) -> Option<GeneralTerm> {
    match var {
        NumberVariable::Edition => Some(GeneralTerm::Edition),
        NumberVariable::Volume => Some(GeneralTerm::Volume),
        _ => None,
    }
}

/// Split a resolved locale term into a `(prefix, suffix)` affix pair around
/// the value it wraps.
///
/// A term containing a literal `%s` (the CSL-M circumfix convention, e.g.
/// zh-CN's `第%s卷`) splits into the text before and after that marker. A
/// term without `%s` (e.g. `版`) follows the value as a space-separated
/// suffix, matching GB/T 7714's `<number/> <label/>` ordering for numeric
/// editions.
fn split_numeric_term(term: &str) -> (Option<String>, Option<String>) {
    if let Some((before, after)) = term.split_once("%s") {
        let prefix = (!before.is_empty()).then(|| before.to_string());
        let suffix = (!after.is_empty()).then(|| after.to_string());
        (prefix, suffix)
    } else {
        (None, Some(format!(" {term}")))
    }
}

/// Render one numeric value through the active locale's ordinal message.
///
/// Values that are not a single unsigned integer remain unchanged because MF2
/// ordinal categories only apply to countable whole numbers.
fn render_ordinal(value: String, options: &RenderOptions<'_>) -> String {
    let Ok(count) = value.parse::<u64>() else {
        return value;
    };
    let args = MessageArgs {
        count: Some(count),
        value: Some(&value),
        ..MessageArgs::default()
    };
    options
        .locale
        .resolve_message("number.ordinal", &args)
        .unwrap_or(value)
}

/// Replace ASCII digits in a rendered numeric value with the locale's configured glyphs.
///
/// Reference markers render outside the template pipeline but still need this,
/// so it is visible to the marker module.
///
/// Non-digit characters remain unchanged, so ranges and mixed identifiers preserve their
/// punctuation and letters while their numeric portions follow locale conventions.
pub(crate) fn localize_digits(value: String, digit_system: &DigitSystem) -> String {
    let digits = match digit_system {
        DigitSystem::Western => return value,
        DigitSystem::ArabicIndic => ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'],
        DigitSystem::ExtendedArabicIndic => ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'],
        DigitSystem::Devanagari => ['०', '१', '२', '३', '४', '५', '६', '७', '८', '९'],
        _ => return value,
    };

    value
        .chars()
        .map(|ch| {
            ch.to_digit(10)
                .filter(|_| ch.is_ascii_digit())
                .and_then(|digit| digits.get(digit as usize).copied())
                .unwrap_or(ch)
        })
        .collect()
}

impl ComponentValues for TemplateNumber {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let fmt = F::default();

        let value = resolve_number_value(
            &self.number,
            reference,
            hints,
            options,
            self.show_with_locator.unwrap_or(false),
        );

        value.filter(|s| !s.is_empty()).map(|value| {
            // Resolve effective rendering options
            let effective_rendering = &self.rendering;

            // Free-text number values (e.g. `edition`) honor an explicit
            // text-case override the same way string variables do.
            let value = if let Some(tc) = effective_rendering.text_case {
                let language = reference.language();
                crate::values::text_case::apply_text_case_with_language(
                    &value,
                    tc,
                    language.as_deref(),
                )
            } else {
                value
            };
            let value_is_numeric = is_numeric(&value);

            let mut value = if self.form == Some(NumberForm::Ordinal) {
                render_ordinal(value, options)
            } else {
                value
            };

            // Handle label if label_form is specified
            let mut label_prefix = if let Some(label_form) = &self.label_form {
                resolve_number_label(
                    &self.number,
                    label_form,
                    &value,
                    self.gender.clone(),
                    effective_rendering,
                    options,
                    &fmt,
                )
            } else {
                None
            };

            // CSL renders `number-of-volumes` as a count followed by its
            // pluralized label (`2 vols.`), unlike locator-style numbers
            // whose labels precede the value (`vol. 2`).
            if matches!(self.number, NumberVariable::NumberOfVolumes)
                && let Some(label) = label_prefix.take()
            {
                value.push(' ');
                value.push_str(label.trim());
            }

            // `when_numeric` resolves this number's locale term (GB/T 7714's
            // `edition`/`volume` general terms) and wraps the value with it —
            // only when the resolved value is numeric; free-text values like
            // `修订版` or a pre-labeled `美国卷` render bare.
            let (numeric_prefix, numeric_suffix) = self
                .when_numeric
                .as_ref()
                .filter(|_| value_is_numeric)
                .and_then(|form| {
                    let general_term = number_var_to_general_term(&self.number)?;
                    options.locale.resolved_general_term(
                        &general_term,
                        &label_form_to_term_form(form),
                        self.gender.clone(),
                    )
                })
                .map(|term| split_numeric_term(&term))
                .unwrap_or((None, None));

            let prefix = match (label_prefix, numeric_prefix) {
                (Some(label), Some(numeric)) => Some(format!("{label}{numeric}")),
                (Some(label), None) => Some(label),
                (None, Some(numeric)) => Some(numeric),
                (None, None) => None,
            };
            let suffix = numeric_suffix;
            let value = localize_digits(value, &options.locale.number_formats.digit_system);

            ProcValues {
                value,
                prefix,
                suffix,
                url: crate::values::resolve_effective_url(
                    self.links.as_ref(),
                    options.config.links.as_ref(),
                    reference,
                    citum_schema::options::LinkAnchor::Component,
                ),
                substituted_key: None,
                pre_formatted: false,
            }
        })
    }
}

/// Citeproc-style `is-numeric` check used to gate [`TemplateNumber::when_numeric`]
/// affixes.
///
/// True for digit runs optionally joined by whitespace, commas, hyphens, or
/// ampersands (bare numerals, ranges, and lists like `"2"`, `"1-3"`,
/// `"12, 14"`). False for any value that mixes a digit with other text
/// (`"新1版"`) or contains none (`"修订版"`, `"美国卷"`, `"第二卷"` — the last
/// uses a CJK numeral character, not an ASCII digit, and so is treated as an
/// already-complete label rather than a bare number to wrap.
pub(crate) fn is_numeric(value: &str) -> bool {
    let value = value.trim();
    let mut saw_digit = false;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            saw_digit = true;
        } else if !matches!(ch, '-' | ',' | '&' | ' ') {
            return false;
        }
    }
    saw_digit
}

#[cfg(test)]
mod is_numeric_tests {
    use super::is_numeric;

    #[test]
    fn given_bare_digits_when_checked_then_numeric() {
        assert!(is_numeric("2"));
        assert!(is_numeric("510"));
    }

    #[test]
    fn given_digit_range_or_list_when_checked_then_numeric() {
        assert!(is_numeric("1-3"));
        assert!(is_numeric("12, 14"));
    }

    #[test]
    fn given_digit_embedded_in_free_text_when_checked_then_not_numeric() {
        assert!(!is_numeric("新1版"));
    }

    #[test]
    fn given_free_text_without_digits_when_checked_then_not_numeric() {
        assert!(!is_numeric("修订版"));
        assert!(!is_numeric("美国卷"));
    }

    #[test]
    fn given_cjk_numeral_when_checked_then_not_numeric() {
        // "二" is a CJK numeral character, not an ASCII digit; GB/T 7714
        // treats "第二卷" as an already-complete label, not a bare number.
        assert!(!is_numeric("第二卷"));
    }

    #[test]
    fn given_empty_value_when_checked_then_not_numeric() {
        assert!(!is_numeric(""));
        assert!(!is_numeric("   "));
    }
}

#[cfg(test)]
mod digit_system_tests {
    use super::localize_digits;
    use citum_schema::locale::DigitSystem;

    #[test]
    fn localizes_ascii_digits_for_supported_digit_systems() {
        for (digit_system, expected) in [
            (DigitSystem::Western, "AB-12, 34"),
            (DigitSystem::ArabicIndic, "AB-١٢, ٣٤"),
            (DigitSystem::ExtendedArabicIndic, "AB-۱۲, ۳۴"),
            (DigitSystem::Devanagari, "AB-१२, ३४"),
        ] {
            assert_eq!(
                localize_digits("AB-12, 34".to_string(), &digit_system),
                expected
            );
        }
    }

    #[test]
    fn preserves_western_digits_for_unknown_digit_system() {
        assert_eq!(
            localize_digits(
                "12".to_string(),
                &DigitSystem::Unknown("future".to_string())
            ),
            "12"
        );
    }
}

#[cfg(test)]
mod split_numeric_term_tests {
    use super::split_numeric_term;

    #[test]
    fn given_circumfix_term_when_split_then_wraps_around_marker() {
        assert_eq!(
            split_numeric_term("第%s卷"),
            (Some("第".to_string()), Some("卷".to_string()))
        );
    }

    #[test]
    fn given_suffix_only_term_when_split_then_prefix_only_marker() {
        // "%s卷" (no leading text) has an empty prefix half, so no prefix is emitted.
        assert_eq!(split_numeric_term("%s卷"), (None, Some("卷".to_string())));
    }

    #[test]
    fn given_plain_term_when_split_then_space_separated_suffix() {
        assert_eq!(split_numeric_term("版"), (None, Some(" 版".to_string())));
    }
}

/// Maps a number variable to its corresponding locator type.
///
/// Determines which `LocatorType` corresponds to a given numeric variable,
/// allowing proper label selection when rendering page, volume, or issue information.
/// Returns `None` for variables with no locator equivalent (e.g. edition, version).
#[must_use]
pub fn number_var_to_locator_type(
    var: &NumberVariable,
) -> Option<citum_schema::citation::LocatorType> {
    use citum_schema::citation::LocatorType;
    match var {
        NumberVariable::Volume => Some(LocatorType::Volume),
        NumberVariable::Pages => Some(LocatorType::Page),
        NumberVariable::ChapterNumber => Some(LocatorType::Chapter),
        NumberVariable::NumberOfPages => Some(LocatorType::Page),
        NumberVariable::NumberOfVolumes => Some(LocatorType::Volume),
        NumberVariable::Number
        | NumberVariable::DocketNumber
        | NumberVariable::PatentNumber
        | NumberVariable::StandardNumber
        | NumberVariable::ReportNumber
        | NumberVariable::PrintingNumber => Some(LocatorType::Number),
        NumberVariable::PartNumber => Some(LocatorType::Part),
        NumberVariable::SupplementNumber => Some(LocatorType::Supplement),
        NumberVariable::Issue => Some(LocatorType::Issue),
        NumberVariable::Custom(kind) => Some(LocatorType::Custom(kind.clone())),
        _ => None,
    }
}

/// Heuristically detect whether a locator string should use plural labeling.
///
/// Returns `true` if the value contains range or list separators — hyphens (`-`),
/// en-dashes (`–`), commas (`,`), or ampersands (`&`) — indicating multiple items
/// such as `"1-10"`, `"1, 3"`, or `"1 & 3"`.
#[must_use]
pub fn check_plural(value: &str, _locator_type: &citum_schema::citation::LocatorType) -> bool {
    // Simple heuristic: if contains ranges or separators, it's plural.
    // "1-10", "1, 3", "1 & 3"
    value.contains('–') || value.contains('-') || value.contains(',') || value.contains('&')
}

/// Format a page range according to the specified format.
///
/// Formats: expanded (default), minimal, minimal-two, chicago, chicago-16.
/// `delimiter` is the range separator (usually the locale's
/// `page-range-delimiter`, en-dash by default; AMA and similar use a hyphen).
#[must_use]
pub fn format_page_range(
    pages: &str,
    format: Option<&citum_schema::options::PageRangeFormat>,
    delimiter: &str,
) -> String {
    use citum_schema::options::PageRangeFormat;

    // Normalize any en-dash separator to a plain hyphen so splitting below is
    // delimiter-agnostic; ranges are re-joined with the configured `delimiter`.
    let normalized = pages.replace('\u{2013}', "-");
    let with_delimiter = || normalized.replace('-', delimiter);

    // If no format specified, just apply the delimiter to the range.
    let Some(format) = format else {
        return with_delimiter();
    };

    let parts: Vec<&str> = normalized.split('-').collect();
    let [start, end] = parts.as_slice() else {
        return with_delimiter(); // Not a simple range
    };
    let start = start.trim();
    let end = end.trim();

    // Parse as numbers
    let start_num: Option<u32> = start.parse().ok();
    let end_num: Option<u32> = end.parse().ok();

    match (start_num, end_num) {
        (Some(s), Some(e)) if e > s => {
            let formatted_end = match format {
                PageRangeFormat::Expanded => end.to_string(),
                PageRangeFormat::Minimal => format_minimal(start, end, 1),
                PageRangeFormat::MinimalTwo => format_minimal(start, end, 2),
                PageRangeFormat::Chicago | PageRangeFormat::Chicago16 => {
                    format_chicago_range_end(s, e)
                }
                _ => end.to_string(), // Future variants: default to expanded
            };
            format!("{start}{delimiter}{formatted_end}")
        }
        _ => with_delimiter(), // Can't parse or invalid range
    }
}

/// Minimal format: keep only differing digits, with minimum `min_digits`
#[must_use]
pub fn format_minimal(start: &str, end: &str, min_digits: usize) -> String {
    let start_chars: Vec<char> = start.chars().collect();
    let end_chars: Vec<char> = end.chars().collect();

    if start_chars.len() != end_chars.len() {
        return end.to_string();
    }

    // Find first differing position
    let mut first_diff = 0;
    for (i, (s, e)) in start_chars.iter().zip(end_chars.iter()).enumerate() {
        if s != e {
            first_diff = i;
            break;
        }
    }

    // Keep at least min_digits from the end
    let keep_from = first_diff.min(end_chars.len().saturating_sub(min_digits));
    end_chars
        .get(keep_from..)
        .unwrap_or_default()
        .iter()
        .collect()
}

/// Format the second number in a Chicago Manual of Style inclusive-number range.
#[must_use]
pub(crate) fn format_chicago_range_end(start: u32, end: u32) -> String {
    // Chicago rules:
    // - start < 100: use all digits
    // - start exact multiple of 100: use all digits
    // - start % 100 is 1..=9: use the changed part only and trim any leading
    //   zero from that suffix (1002–6, 505–17, 107–8)
    // - otherwise: keep at least two digits unless more are needed to show the
    //   changed part (321–25, 1087–89, 1496–500, 13792–803)

    if start < 100 || start.is_multiple_of(100) {
        return end.to_string();
    }

    let start_str = start.to_string();
    let end_str = end.to_string();
    let changed_end_part = if start % 100 <= 9 {
        format_minimal(&start_str, &end_str, 1)
    } else {
        format_minimal(&start_str, &end_str, 2)
    };

    if changed_end_part.len() > 1 && changed_end_part.starts_with('0') {
        let trimmed = changed_end_part.trim_start_matches('0');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        changed_end_part
    }
}

/// Format a Chicago Manual of Style inclusive-number range end.
#[must_use]
pub fn format_chicago(start: u32, end: u32) -> String {
    format_chicago_range_end(start, end)
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
    use citum_schema::options::PageRangeFormat;

    #[test]
    fn test_format_chicago_page_range_end() {
        for (start, end, expected) in [
            (3, 10, "10"),
            (71, 72, "72"),
            (92, 113, "113"),
            (100, 104, "104"),
            (600, 613, "613"),
            (107, 108, "8"),
            (505, 517, "17"),
            (1002, 1006, "6"),
            (321, 325, "25"),
            (415, 532, "532"),
            (1087, 1089, "89"),
            (1496, 1500, "500"),
            (13792, 13803, "803"),
            (12991, 13001, "3001"),
        ] {
            assert_eq!(format_chicago_range_end(start, end), expected);
        }
    }

    #[test]
    fn test_format_minimal() {
        for (start, end, min_digits, expected) in [
            ("100", "105", 1, "5"),
            ("100", "105", 2, "05"),
            ("1536", "1538", 1, "8"),
            ("1536", "1538", 2, "38"),
            ("1536", "1538", 4, "1538"),
            ("12", "15", 1, "5"),
            ("12", "15", 2, "15"),
            ("10", "150", 1, "150"),
        ] {
            assert_eq!(format_minimal(start, end, min_digits), expected);
        }
    }

    #[test]
    fn test_format_page_range() {
        // Default en-dash delimiter.
        let en = "\u{2013}";
        for (input, format, expected) in [
            ("10-15", None, "10–15"),
            ("10–15", None, "10–15"),
            ("321-328", None, "321–328"),
            ("10-15", Some(PageRangeFormat::Expanded), "10–15"),
            ("42-45", Some(PageRangeFormat::Expanded), "42–45"),
            ("3-10", Some(PageRangeFormat::Chicago), "3–10"),
            ("71-72", Some(PageRangeFormat::Chicago), "71–72"),
            ("92-113", Some(PageRangeFormat::Chicago), "92–113"),
            ("100-104", Some(PageRangeFormat::Chicago), "100–104"),
            ("600-613", Some(PageRangeFormat::Chicago), "600–613"),
            ("107-108", Some(PageRangeFormat::Chicago), "107–8"),
            ("505-517", Some(PageRangeFormat::Chicago), "505–17"),
            ("1002-1006", Some(PageRangeFormat::Chicago), "1002–6"),
            ("321-325", Some(PageRangeFormat::Chicago), "321–25"),
            ("415-532", Some(PageRangeFormat::Chicago), "415–532"),
            ("1087-1089", Some(PageRangeFormat::Chicago), "1087–89"),
            ("1496-1500", Some(PageRangeFormat::Chicago), "1496–500"),
            ("13792-13803", Some(PageRangeFormat::Chicago), "13792–803"),
            ("12991-13001", Some(PageRangeFormat::Chicago), "12991–3001"),
            ("3-10", Some(PageRangeFormat::Chicago16), "3–10"),
            ("71-72", Some(PageRangeFormat::Chicago16), "71–72"),
            ("92-113", Some(PageRangeFormat::Chicago16), "92–113"),
            ("100-104", Some(PageRangeFormat::Chicago16), "100–104"),
            ("600-613", Some(PageRangeFormat::Chicago16), "600–613"),
            ("107-108", Some(PageRangeFormat::Chicago16), "107–8"),
            ("505-517", Some(PageRangeFormat::Chicago16), "505–17"),
            ("1002-1006", Some(PageRangeFormat::Chicago16), "1002–6"),
            ("321-325", Some(PageRangeFormat::Chicago16), "321–25"),
            ("415-532", Some(PageRangeFormat::Chicago16), "415–532"),
            ("1087-1089", Some(PageRangeFormat::Chicago16), "1087–89"),
            ("1496-1500", Some(PageRangeFormat::Chicago16), "1496–500"),
            ("13792-13803", Some(PageRangeFormat::Chicago16), "13792–803"),
            (
                "12991-13001",
                Some(PageRangeFormat::Chicago16),
                "12991–3001",
            ),
            ("100-105", Some(PageRangeFormat::Minimal), "100–5"),
            ("321-328", Some(PageRangeFormat::Minimal), "321–8"),
            ("42-45", Some(PageRangeFormat::Minimal), "42–5"),
            ("12-17", Some(PageRangeFormat::Minimal), "12–7"),
            ("100-105", Some(PageRangeFormat::MinimalTwo), "100–05"),
            ("42-45", Some(PageRangeFormat::MinimalTwo), "42–45"),
            ("10", Some(PageRangeFormat::Chicago), "10"),
            ("10-5", Some(PageRangeFormat::Chicago), "10–5"),
            ("X-Y", Some(PageRangeFormat::Chicago), "X–Y"),
            ("10-15-20", Some(PageRangeFormat::Chicago), "10–15–20"),
        ] {
            assert_eq!(format_page_range(input, format.as_ref(), en), expected);
        }
    }

    #[test]
    fn test_format_page_range_hyphen_delimiter() {
        // AMA-style hyphen delimiter: en-dash input is normalized to a hyphen,
        // and range formats still apply.
        for (input, format, expected) in [
            ("436-444", None, "436-444"),
            ("436–444", None, "436-444"),
            ("321-328", Some(PageRangeFormat::Expanded), "321-328"),
            ("321-328", Some(PageRangeFormat::Chicago), "321-28"),
        ] {
            assert_eq!(format_page_range(input, format.as_ref(), "-"), expected);
        }
    }

    #[test]
    fn test_check_plural() {
        for (value, expected) in [
            ("1-10", true),
            ("1–10", true),
            ("1, 3", true),
            ("1 & 3", true),
            ("1", false),
            ("IV", false),
        ] {
            assert_eq!(
                check_plural(value, &citum_schema::citation::LocatorType::Page),
                expected
            );
        }
    }

    #[test]
    fn number_var_to_locator_type_maps_printing_number() {
        assert_eq!(
            number_var_to_locator_type(&NumberVariable::PrintingNumber),
            Some(citum_schema::citation::LocatorType::Number)
        );
    }

    #[test]
    fn number_var_to_locator_type_maps_part_number() {
        assert_eq!(
            number_var_to_locator_type(&NumberVariable::PartNumber),
            Some(citum_schema::citation::LocatorType::Part)
        );
    }

    #[test]
    fn number_var_to_locator_type_maps_supplement_number() {
        assert_eq!(
            number_var_to_locator_type(&NumberVariable::SupplementNumber),
            Some(citum_schema::citation::LocatorType::Supplement)
        );
    }
}
