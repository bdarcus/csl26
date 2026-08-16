/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for date fields with locale-aware formatting.
//!
//! This module handles date component rendering with support for different date forms,
//! time formatting, and locale-specific date presentation.

use crate::reference::{DateValue, Reference};
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use citum_edtf::{Edtf, Timezone, UnspecifiedYear, Year};
use citum_schema::locale::SubYearCode;
use citum_schema::options::dates::{DateRangeFormat, TimeFormat};
use citum_schema::options::{Config, DateFallbackCandidate};
use citum_schema::reference::types::RefDate;
use citum_schema::reference::{ClassExtension, WorkRelation};
use citum_schema::template::{
    DateForm, DateVariable as TemplateDateVar, Rendering, TemplateComponent, TemplateDate,
};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Zero-pad a rendered day when `zero_pad` is set, otherwise render it as-is.
fn format_day(day: u32, zero_pad: bool) -> String {
    if zero_pad {
        format!("{day:02}")
    } else {
        day.to_string()
    }
}

fn code_to_string(code: u32, names: &BTreeMap<SubYearCode, String>) -> String {
    u8::try_from(code)
        .ok()
        .and_then(SubYearCode::new)
        .and_then(|c| names.get(&c))
        .cloned()
        .unwrap_or_default()
}

/// Zero-padded numeric month (`"01"`–`"12"`) for `month: numeric` rendering.
/// Seasons and literal dates have no numeric form and return `None` so
/// callers fall back to the textual path.
fn extract_month_numeric(date: &DateValue) -> Option<String> {
    let RefDate::Edtf(edtf) = date.parse() else {
        return None;
    };
    let month = edtf.month()?;
    (1..=12).contains(&month).then(|| format!("{month:02}"))
}

fn extract_month(
    date: &DateValue,
    months: &BTreeMap<SubYearCode, String>,
    seasons: &BTreeMap<SubYearCode, String>,
) -> String {
    let parsed_date = date.parse();
    let edtf = match parsed_date {
        RefDate::Edtf(edtf) => edtf,
        RefDate::Literal(_) => return String::new(),
    };
    match edtf.month() {
        Some(month) => code_to_string(month, months),
        None => match edtf.season_code() {
            Some(code) => code_to_string(code, seasons),
            None => String::new(),
        },
    }
}

/// Resolve the reference-level date value a `TemplateDateVar` addresses.
///
/// Shared by `TemplateDate::values` (rendering) and `Disambiguator`
/// (collision-key discrimination, `csl26-huuz`) so both read the date
/// variable → reference-field mapping from one place.
pub(crate) fn resolve_date_variable(
    variable: &TemplateDateVar,
    reference: &Reference,
) -> Option<DateValue> {
    match variable {
        TemplateDateVar::Issued => reference.effective_issued_date(),
        TemplateDateVar::Accessed => reference.accessed(),
        TemplateDateVar::OriginalPublished => reference.original_date(),
        TemplateDateVar::EventDate => event_date(reference),
        TemplateDateVar::Copyright => reference.copyright(),
        TemplateDateVar::Printing => reference.printing(),
        _ => None,
    }
}

/// Resolve the effective options-level candidates for one issued occurrence.
pub(crate) fn effective_date_fallback_candidates<'a>(
    config: &'a Config,
    first_issued: bool,
    ref_type: &str,
) -> Option<Cow<'a, [DateFallbackCandidate]>> {
    config
        .date_fallback
        .as_ref()?
        .rule_for(first_issued, ref_type)?
        .candidates()
}

/// The same date-text formatting `TemplateDate::values` applies to a
/// resolved date value — `form`-restricted range/single-date formatting plus
/// uncertainty/approximation markers — before any year-suffix disambiguation
/// is layered on. Exposed so the disambiguator's collision-key discriminant
/// reads the text a reference will actually render, not the raw stored
/// value (whose `Display` is the unformatted EDTF/literal string and can
/// carry more precision than `form` shows, e.g. a day-precision `copyright`
/// date under `form: year`). See csl26-huuz.
pub(crate) fn formatted_date_text(
    date: &DateValue,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    format_date_range(date, form, locale, date_config)
        .map(|value| apply_date_markers(value, date, date_config))
}

/// Text uniquely identifying what a date component renders for a specific
/// reference, for collision-key purposes: the `form`-restricted formatted
/// value (`formatted_date_text`) plus the candidate's visible rendering
/// configuration and the resolved value's `note` — the same extra text
/// `render_fallback_component`/`append_note` add to the bare value
/// during real rendering. Two candidates whose rendered text differs only in
/// these respects (e.g. a `c`-prefixed `copyright` year and a
/// `印刷`-suffixed `printing` year that happen to share the same bare year)
/// must not discriminant to the same text.
///
/// This does not need to *look like* the rendered text — it only needs the
/// invariant "same render inputs ⟺ same discriminant" — so it Debug-formats
/// the visible rendering config rather than running the full punctuation-
/// realization pipeline, which would require threading a complete
/// `RenderOptions` and `OutputFormat` into `Disambiguator` for no
/// observable benefit. See csl26-huuz, flagged in PR review.
pub(crate) fn fallback_candidate_discriminant(
    date: &DateValue,
    form: &DateForm,
    rendering: &Rendering,
    suppress_note: Option<bool>,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    let formatted = formatted_date_text(date, form, locale, date_config)?;
    let note = (suppress_note != Some(true)
        && date_config.is_some_and(|config| config.note_wrap.is_some()))
    .then_some(date.note.as_deref())
    .flatten()
    .filter(|note| !note.is_empty())
    .unwrap_or_default();
    Some(format!(
        "{formatted}|{note}|{}",
        visible_rendering_discriminant(rendering)
    ))
}

/// Resolve the visible collision-key text for a message fallback candidate.
pub(crate) fn fallback_message_discriminant(
    message: &citum_schema::template::TemplateMessage,
    locale: &citum_schema::locale::Locale,
    config: &Config,
) -> Option<String> {
    if message.rendering.suppress == Some(true) {
        return None;
    }
    let value = crate::values::message::resolve_template_message_value(message, config, locale)?;
    Some(format!(
        "{value}|{}",
        visible_rendering_discriminant(&message.rendering)
    ))
}

fn visible_rendering_discriminant(rendering: &Rendering) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        rendering.emph,
        rendering.quote,
        rendering.strong,
        rendering.small_caps,
        rendering.vertical_align,
        rendering.prefix,
        rendering.suffix,
        rendering.wrap
    )
}

fn event_date(reference: &Reference) -> Option<DateValue> {
    match reference.extension() {
        ClassExtension::Event(event) => event.date.clone(),
        ClassExtension::Monograph(monograph) => embedded_event_date(monograph.event.as_ref()?),
        ClassExtension::SerialComponent(component) => {
            embedded_event_date(component.event.as_ref()?)
        }
        ClassExtension::AudioVisual(audio_visual) => {
            embedded_event_date(audio_visual.event.as_ref()?)
        }
        _ => None,
    }
}

fn embedded_event_date(relation: &WorkRelation) -> Option<DateValue> {
    let WorkRelation::Embedded(reference) = relation else {
        return None;
    };
    let ClassExtension::Event(event) = reference.extension() else {
        return None;
    };
    event.date.clone()
}

/// Compute the delta for unspecified year ranges.
fn unspecified_year_delta(u: &UnspecifiedYear) -> i64 {
    match u {
        UnspecifiedYear::None => 0,
        UnspecifiedYear::One => 9,
        UnspecifiedYear::Two => 99,
        UnspecifiedYear::Three => 999,
        UnspecifiedYear::Four => 9999,
    }
}

/// Format a year with era-aware rendering.
fn format_display_year(
    year: &Year,
    date_terms: &citum_schema::locale::DateTerms,
    era_labels: &citum_schema::options::dates::EraLabels,
    _neg_unspecified: &citum_schema::options::dates::NegativeUnspecifiedYears,
    range_delimiter: &str,
) -> String {
    // Handle positive unspecified years: normalize 'u' to 'X'
    if year.unspecified != UnspecifiedYear::None && year.value > 0 {
        let mut s = year.value.to_string();
        let unspec_count = match year.unspecified {
            UnspecifiedYear::One => 1,
            UnspecifiedYear::Two => 2,
            UnspecifiedYear::Three => 3,
            UnspecifiedYear::Four => 4,
            _ => 0,
        };
        for _ in 0..unspec_count {
            if let Some(last) = s.pop()
                && last != '0'
            {
                s.push('X');
            }
        }
        if s.len() < year.value.to_string().len() {
            let diff = year.value.to_string().len() - s.len();
            for _ in 0..diff {
                s.push('X');
            }
        }
        return s;
    }

    // Handle negative unspecified years: compute historical range
    if year.unspecified != UnspecifiedYear::None && year.value <= 0 {
        let delta = unspecified_year_delta(&year.unspecified);
        let astronomical_min = year.value - delta;
        let astronomical_max = year.value;
        let historical_end = 1 - astronomical_max;
        let historical_start = 1 - astronomical_min;

        let era_term = match era_labels {
            citum_schema::options::dates::EraLabels::Default => {
                date_terms.before_era.as_deref().unwrap_or("")
            }
            citum_schema::options::dates::EraLabels::BcAd => date_terms.bc.as_deref().unwrap_or(""),
            citum_schema::options::dates::EraLabels::BceCe => {
                date_terms.bce.as_deref().unwrap_or("")
            }
        };

        if era_term.is_empty() {
            format!("{historical_start}{range_delimiter}{historical_end}")
        } else {
            format!("{historical_start}{range_delimiter}{historical_end} {era_term}")
        }
    } else if year.value <= 0 {
        // Fully specified negative year
        let historical_year = 1 - year.value;
        let era_term = match era_labels {
            citum_schema::options::dates::EraLabels::Default => {
                date_terms.before_era.as_deref().unwrap_or("")
            }
            citum_schema::options::dates::EraLabels::BcAd => date_terms.bc.as_deref().unwrap_or(""),
            citum_schema::options::dates::EraLabels::BceCe => {
                date_terms.bce.as_deref().unwrap_or("")
            }
        };

        if era_term.is_empty() {
            historical_year.to_string()
        } else {
            format!("{historical_year} {era_term}")
        }
    } else {
        // Positive year
        let era_term = match era_labels {
            citum_schema::options::dates::EraLabels::Default => "",
            citum_schema::options::dates::EraLabels::BcAd => date_terms.ad.as_deref().unwrap_or(""),
            citum_schema::options::dates::EraLabels::BceCe => {
                date_terms.ce.as_deref().unwrap_or("")
            }
        };

        if era_term.is_empty() {
            year.value.to_string()
        } else {
            format!("{} {}", year.value, era_term)
        }
    }
}

/// Legacy format_display_year for backwards compatibility.
fn format_display_year_legacy(year: &Year, before_era: Option<&str>) -> String {
    if year.unspecified != UnspecifiedYear::None {
        return year.to_string();
    }

    if year.value <= 0 {
        let historical_year = 1 - year.value;
        if let Some(term) = before_era.filter(|term| !term.is_empty()) {
            format!("{historical_year} {term}")
        } else {
            historical_year.to_string()
        }
    } else {
        year.value.to_string()
    }
}

#[allow(dead_code, reason = "kept for backwards compatibility")]
fn extract_display_year_legacy(date: &DateValue, before_era: Option<&str>) -> String {
    match date.parse() {
        RefDate::Edtf(edtf) => match edtf {
            Edtf::Date(date) => format_display_year_legacy(&date.year, before_era),
            Edtf::Interval(interval) => {
                format_display_year_legacy(&interval.start.year, before_era)
            }
            Edtf::IntervalFrom(date) | Edtf::IntervalTo(date) => {
                format_display_year_legacy(&date.year, before_era)
            }
        },
        RefDate::Literal(_) => String::new(),
    }
}

/// Formats a time with the specified format, optionally including seconds and timezone.
///
/// Converts 24-hour time to 12-hour format if specified, and appends localized
/// AM/PM or timezone indicators as configured.
fn format_time(
    time: citum_edtf::Time,
    format: &TimeFormat,
    show_seconds: bool,
    show_timezone: bool,
    am_term: Option<&str>,
    pm_term: Option<&str>,
    utc_term: Option<&str>,
) -> String {
    let (display_hour, period) = match format {
        TimeFormat::Hour12 => {
            let (h, p) = if time.hour == 0 {
                (12u32, am_term.unwrap_or("AM"))
            } else if time.hour < 12 {
                (time.hour, am_term.unwrap_or("AM"))
            } else if time.hour == 12 {
                (12u32, pm_term.unwrap_or("PM"))
            } else {
                (time.hour - 12, pm_term.unwrap_or("PM"))
            };
            (h, Some(p))
        }
        TimeFormat::Hour24 => (time.hour, None),
    };

    let time_str = if show_seconds {
        format!("{:02}:{:02}:{:02}", display_hour, time.minute, time.second)
    } else {
        format!("{:02}:{:02}", display_hour, time.minute)
    };

    let with_period = match period {
        Some(p) => format!("{time_str} {p}"),
        None => time_str,
    };

    if show_timezone {
        let tz_str = match time.timezone {
            Some(Timezone::Utc) => utc_term.unwrap_or("UTC").to_string(),
            Some(Timezone::Offset(mins)) => {
                let sign = if mins >= 0 { '+' } else { '-' };
                let abs = mins.unsigned_abs();
                format!("{}{:02}:{:02}", sign, abs / 60, abs % 60)
            }
            None => String::new(),
        };
        if tz_str.is_empty() {
            with_period
        } else {
            format!("{with_period} {tz_str}")
        }
    } else {
        with_period
    }
}

/// Format a single date or a date range (open or closed) according to the
/// given form, delegating both endpoints of a range to
/// [`format_single_date`] so locale patterns apply symmetrically.
fn format_date_range(
    date: &DateValue,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    let delimiter = date_config.map_or("–", |c| c.range_delimiter.as_str());

    match date.parse() {
        RefDate::Edtf(Edtf::Interval(interval)) => {
            format_closed_range(date, &interval, form, locale, date_config, delimiter)
        }
        RefDate::Edtf(Edtf::IntervalFrom(_)) => {
            // Open-ended range (e.g., "1990/..'): the accessors on the whole
            // interval already resolve to the start point.
            let start = format_single_date(date, form, locale, date_config)?;
            if let Some(end_marker) = date_config
                .and_then(|c| c.open_range_marker.as_deref())
                .or(locale.dates.open_ended_term.as_deref())
            {
                Some(format!("{start}{delimiter}{end_marker}"))
            } else {
                Some(start)
            }
        }
        // Non-range dates and open-ended-from-start ranges ("../2020") only
        // have one known point, which the accessors already expose.
        _ => format_single_date(date, form, locale, date_config),
    }
}

/// Format a closed date range, collapsing the start point's year when both
/// endpoints share a year and the form displays a month.
fn format_closed_range(
    date: &DateValue,
    interval: &citum_edtf::Interval,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
    delimiter: &str,
) -> Option<String> {
    if let Some(rendered) =
        format_chicago_year_range(interval, form, locale, date_config, delimiter)
    {
        return Some(rendered);
    }

    let same_year = interval.start.year.value == interval.end.year.value;
    let both_have_month =
        interval.start.month_or_season.is_some() && interval.end.month_or_season.is_some();

    if same_year
        && (both_have_month || matches!(form, DateForm::Year))
        && let Some(collapsed) = format_same_year_range(
            &interval.start,
            &interval.end,
            form,
            locale,
            date_config,
            delimiter,
        )
    {
        return Some(collapsed);
    }

    let start = format_single_date(date, form, locale, date_config);
    let end = format_single_date(
        &DateValue::new(interval.end.to_string()),
        form,
        locale,
        date_config,
    );

    match (start, end) {
        (Some(s), Some(e)) => Some(format!("{s}{delimiter}{e}")),
        (Some(s), None) => Some(s),
        (None, Some(e)) => Some(e),
        (None, None) => None,
    }
}

/// Format a closed year interval with Chicago's inclusive-number abbreviation.
///
/// EDTF represents BCE years astronomically, so this deliberately formats the
/// displayed historical numbers rather than relying on ascending numeric input.
fn format_chicago_year_range(
    interval: &citum_edtf::Interval,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
    delimiter: &str,
) -> Option<String> {
    if !matches!(form, DateForm::Year)
        || !matches!(
            date_config.map(|config| &config.range_format),
            Some(DateRangeFormat::Chicago)
        )
        || interval.start.year.unspecified != UnspecifiedYear::None
        || interval.end.year.unspecified != UnspecifiedYear::None
        || interval.start.month_or_season.is_some()
        || interval.end.month_or_season.is_some()
    {
        return None;
    }

    let start_is_bce = interval.start.year.value <= 0;
    let end_is_bce = interval.end.year.value <= 0;
    if start_is_bce != end_is_bce || interval.end.year.value <= interval.start.year.value {
        return None;
    }

    let start = display_year_number(interval.start.year.value)?;
    let end = display_year_number(interval.end.year.value)?;
    let abbreviated_end = crate::values::number::format_chicago_range_end(start, end);
    let era = chicago_year_range_era_suffix(start_is_bce, locale, date_config);
    Some(format!("{start}{delimiter}{abbreviated_end}{era}"))
}

fn display_year_number(year: i64) -> Option<u32> {
    let historical_year = if year <= 0 {
        1_i64.checked_sub(year)?
    } else {
        year
    };
    u32::try_from(historical_year).ok()
}

fn chicago_year_range_era_suffix(
    is_bce: bool,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> String {
    use citum_schema::options::dates::EraLabels;

    let era_labels = date_config
        .map(|config| &config.era_labels)
        .unwrap_or(&EraLabels::Default);
    let label = match (is_bce, era_labels) {
        (true, EraLabels::Default) => locale.dates.before_era.as_deref(),
        (true, EraLabels::BcAd) => locale.dates.bc.as_deref(),
        (true, EraLabels::BceCe) => locale.dates.bce.as_deref(),
        (false, EraLabels::Default) => None,
        (false, EraLabels::BcAd) => locale.dates.ad.as_deref(),
        (false, EraLabels::BceCe) => locale.dates.ce.as_deref(),
    };
    label
        .filter(|value| !value.is_empty())
        .map(|value| format!(" {value}"))
        .unwrap_or_default()
}

/// Format a closed range whose endpoints share a year, suppressing the
/// redundant year on one side (e.g. "May 14–June 2, 2023").
///
/// Locale interval patterns receive reduced endpoints and the common year.
/// When a locale has no pattern, the pre-existing English layouts remain the
/// fallback for forms that already supported same-year suppression.
fn format_same_year_range(
    start: &citum_edtf::Date,
    end: &citum_edtf::Date,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
    delimiter: &str,
) -> Option<String> {
    let start_date = DateValue::new(start.to_string());
    let end_date = DateValue::new(end.to_string());
    let start_fragment = format_same_year_fragment(&start_date, form, locale, date_config)?;
    let end_fragment = format_same_year_fragment(&end_date, form, locale, date_config)?;
    let shared_year = date_form_displays_year(form)
        .then(|| format_single_date(&start_date, &DateForm::Year, locale, date_config))
        .flatten();

    if let Some(pattern_id) = date_range_pattern_id(form)
        && let Some(rendered) = locale.resolve_date_range_pattern(
            pattern_id,
            &start_fragment,
            &end_fragment,
            shared_year.as_deref(),
        )
    {
        return Some(rendered);
    }

    match form {
        DateForm::Full => {
            let end_full = format_single_date(&end_date, &DateForm::Full, locale, date_config)?;
            Some(format!("{start_fragment}{delimiter}{end_full}"))
        }
        DateForm::YearMonth => {
            let end_full =
                format_single_date(&end_date, &DateForm::YearMonth, locale, date_config)?;
            Some(format!("{start_fragment}{delimiter}{end_full}"))
        }
        DateForm::YearMonthDay => {
            let start_full =
                format_single_date(&start_date, &DateForm::YearMonthDay, locale, date_config)?;
            Some(format!("{start_full}{delimiter}{end_fragment}"))
        }
        _ => None,
    }
}

fn date_range_pattern_id(form: &DateForm) -> Option<&'static str> {
    match form {
        DateForm::Year => Some("pattern.date-range-year"),
        DateForm::Month => Some("pattern.date-range-month"),
        DateForm::MonthDay => Some("pattern.date-range-month-day"),
        DateForm::YearMonth => Some("pattern.date-range-year-month"),
        DateForm::Full => Some("pattern.date-range-full"),
        DateForm::YearMonthDay => Some("pattern.date-range-year-month-day"),
        DateForm::DayMonthAbbrYear => Some("pattern.date-range-day-month-abbr-year"),
        DateForm::MonthAbbrDayYear => Some("pattern.date-range-month-abbr-day-year"),
        _ => None,
    }
}

fn format_same_year_fragment(
    date: &DateValue,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    match form {
        DateForm::Year => format_single_date(date, &DateForm::Year, locale, date_config),
        DateForm::Month | DateForm::YearMonth => {
            format_single_date(date, &DateForm::Month, locale, date_config)
        }
        DateForm::Full | DateForm::MonthDay | DateForm::YearMonthDay => {
            format_single_date(date, &DateForm::MonthDay, locale, date_config)
        }
        DateForm::DayMonthAbbrYear | DateForm::MonthAbbrDayYear => {
            format_abbreviated_month_day_fragment(date, form, locale, date_config)
        }
        _ => None,
    }
}

fn format_abbreviated_month_day_fragment(
    date: &DateValue,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    let numeric_months = date_config
        .is_some_and(|config| config.month == citum_schema::options::MonthFormat::Numeric);
    if numeric_months && let Some(month) = extract_month_numeric(date) {
        return Some(match date.day() {
            Some(day) => format!("{month}-{day:02}"),
            None => month,
        });
    }

    let month = extract_month(date, &locale.dates.months.short, &locale.dates.seasons);
    if month.is_empty() {
        return None;
    }
    let zero_pad_day = date_config.is_some_and(|c| c.day_zero_pad);
    match (form, date.day()) {
        (DateForm::DayMonthAbbrYear, Some(day)) => {
            Some(format!("{} {month}", format_day(day, zero_pad_day)))
        }
        (DateForm::MonthAbbrDayYear, Some(day)) => {
            Some(format!("{month} {}", format_day(day, zero_pad_day)))
        }
        (_, None) => Some(month),
        _ => None,
    }
}

/// Append a date's opaque `note` (e.g. a source-calendar annotation), wrapped
/// per `DateConfig.note_wrap`, directly after the complete formatted date —
/// after any inlined year-suffix, before the component's own outer
/// prefix/suffix/wrap. A no-op when the style has no `note-wrap` configured
/// for this scope, or the date carries no note. The caller additionally
/// skips this function entirely when the component sets
/// `TemplateDate::suppress_note`. See
/// `docs/specs/CALENDAR_DATE_ANNOTATIONS.md`.
fn append_note<F: crate::render::format::OutputFormat<Output = String>>(
    fmt: &F,
    formatted: String,
    date: &DateValue,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
    reference: &Reference,
    options: &RenderOptions<'_>,
) -> String {
    let Some(note) = date.note.as_deref().filter(|n| !n.is_empty()) else {
        return formatted;
    };
    let Some(wrap) = date_config.and_then(|c| c.note_wrap.as_ref()) else {
        return formatted;
    };

    let content = fmt.text(note);
    let content = fmt.inner_affix(
        wrap.inner_prefix.as_deref().unwrap_or_default(),
        content,
        wrap.inner_suffix.as_deref().unwrap_or_default(),
    );
    let marks = crate::render::format::QuoteMarks::from(&options.locale.grammar_options);
    let item_language = crate::values::effective_item_language(reference);
    let (script, realization) = crate::values::punctuation_realization_context(
        item_language.as_deref(),
        options.config.multilingual.as_ref(),
        options.locale.punctuation_realization.as_ref(),
    );
    let wrapped = fmt.wrap_punctuation(
        &wrap.punctuation,
        content,
        &marks,
        script,
        realization.as_deref(),
    );
    format!("{formatted}{wrapped}")
}

/// Apply uncertainty and approximation markers to formatted date.
fn apply_date_markers(
    value: String,
    date: &DateValue,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> String {
    let mut result = value;
    if date.is_approximate()
        && let Some(marker) = date_config.and_then(|c| c.approximation_marker.as_ref())
    {
        let suffix = date_config
            .and_then(|c| c.approximation_marker_suffix.as_deref())
            .unwrap_or("");
        result = format!("{marker}{result}{suffix}");
    }
    if date.is_uncertain()
        && let Some(marker) = date_config.and_then(|c| c.uncertainty_marker.as_ref())
    {
        let prefix = date_config
            .and_then(|c| c.uncertainty_marker_prefix.as_deref())
            .unwrap_or("");
        result = format!("{prefix}{result}{marker}");
    }
    result
}

/// Compute the disambiguation suffix for year-based citations.
fn compute_disamb_suffix<F: crate::render::format::OutputFormat<Output = String>>(
    date: &DateValue,
    form: &DateForm,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> Option<String> {
    if hints.disamb_condition && date_form_displays_year(form) && !date.year().is_empty() {
        compute_disamb_suffix_label(hints, options, fmt)
    } else {
        None
    }
}

fn compute_disamb_suffix_label<F: crate::render::format::OutputFormat<Output = String>>(
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> Option<String> {
    // Check if year suffix is enabled, resolving the processing default
    // centrally so an unset `processing` matches the rest of the engine.
    let use_suffix = options
        .config
        .effective_processing()
        .config()
        .disambiguate
        .as_ref()
        .is_some_and(|d| d.year_suffix);

    if hints.disamb_condition && use_suffix {
        int_to_letter(hints.group_index as u32).map(|s| fmt.text(&s))
    } else {
        None
    }
}

fn date_form_displays_year(form: &DateForm) -> bool {
    !matches!(form, DateForm::MonthDay)
}

fn append_no_date_disamb_suffix(value: &mut String, suffix: &str, options: &RenderOptions<'_>) {
    let delimiter = options.config.dates.as_ref().map_or("-", |date_config| {
        date_config.no_date_year_suffix_delimiter.as_str()
    });
    value.push_str(delimiter);
    value.push_str(suffix);
}

fn inline_disamb_suffix(formatted: &str, form: &DateForm, year: &str, suffix: &str) -> String {
    if year.is_empty() || suffix.is_empty() {
        return formatted.to_string();
    }

    let year_index = match form {
        DateForm::Year | DateForm::YearMonthDay => formatted.find(year),
        DateForm::YearMonth
        | DateForm::Full
        | DateForm::DayMonthAbbrYear
        | DateForm::MonthAbbrDayYear => formatted.rfind(year),
        DateForm::MonthDay => None,
        _ => None,
    };

    let Some(index) = year_index else {
        return format!("{formatted}{suffix}");
    };

    let year_end = index + year.len();
    #[allow(clippy::string_slice, reason = "indices derived from find/rfind")]
    let result = format!(
        "{}{}{}{}",
        &formatted[..index],
        year,
        suffix,
        &formatted[year_end..]
    );
    result
}

/// Format a single date (non-range) according to the given form.
#[allow(
    clippy::too_many_lines,
    reason = "date formatting handles 6 form variants"
)]
fn format_single_date(
    date: &DateValue,
    form: &DateForm,
    locale: &citum_schema::locale::Locale,
    date_config: Option<&citum_schema::options::dates::DateConfig>,
) -> Option<String> {
    let default_era = citum_schema::options::dates::EraLabels::Default;
    let default_neg_unspec = citum_schema::options::dates::NegativeUnspecifiedYears::default();
    let era_labels = date_config.map(|c| &c.era_labels).unwrap_or(&default_era);
    let neg_unspecified = date_config
        .map(|c| &c.negative_unspecified_years)
        .unwrap_or(&default_neg_unspec);
    let range_delimiter = date_config.map_or("–", |c| c.range_delimiter.as_str());
    // `month: numeric` renders month-bearing forms as zero-padded numerals
    // joined with hyphens (GB/T 7714, ISO 690). Dates without a real calendar
    // month (seasons, literals) fall back to the textual path.
    let numeric_months =
        date_config.is_some_and(|c| c.month == citum_schema::options::MonthFormat::Numeric);
    // Independent of `numeric_months`: those numeral paths already
    // zero-pad the day unconditionally as part of their fixed format.
    let zero_pad_day = date_config.is_some_and(|c| c.day_zero_pad);

    let extract_year = |d: &DateValue| -> String {
        match d.parse() {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(dt) => format_display_year(
                    &dt.year,
                    &locale.dates,
                    era_labels,
                    neg_unspecified,
                    range_delimiter,
                ),
                Edtf::Interval(interval) => format_display_year(
                    &interval.start.year,
                    &locale.dates,
                    era_labels,
                    neg_unspecified,
                    range_delimiter,
                ),
                Edtf::IntervalFrom(dt) | Edtf::IntervalTo(dt) => format_display_year(
                    &dt.year,
                    &locale.dates,
                    era_labels,
                    neg_unspecified,
                    range_delimiter,
                ),
            },
            RefDate::Literal(_) => String::new(),
        }
    };

    match form {
        DateForm::Year => {
            let year = extract_year(date);
            if year.is_empty() { None } else { Some(year) }
        }
        DateForm::YearMonth => {
            let year = extract_year(date);
            if year.is_empty() {
                return None;
            }
            if numeric_months && let Some(month) = extract_month_numeric(date) {
                return Some(format!("{year}-{month}"));
            }
            let month = extract_month(date, &locale.dates.months.long, &locale.dates.seasons);
            let month_opt = (!month.is_empty()).then_some(month.as_str());
            if let Some(rendered) = locale.resolve_date_pattern(
                "pattern.date-year-month",
                Some(&year),
                month_opt,
                None,
                zero_pad_day,
            ) {
                return Some(rendered);
            }
            if month.is_empty() {
                Some(year)
            } else {
                Some(format!("{month} {year}"))
            }
        }
        DateForm::Month => {
            if numeric_months && let Some(month) = extract_month_numeric(date) {
                return Some(month);
            }
            let month = extract_month(date, &locale.dates.months.long, &locale.dates.seasons);
            if month.is_empty() { None } else { Some(month) }
        }
        DateForm::MonthDay => {
            if numeric_months && let Some(month) = extract_month_numeric(date) {
                return Some(match date.day() {
                    Some(d) => format!("{month}-{d:02}"),
                    None => month,
                });
            }
            let month = extract_month(date, &locale.dates.months.long, &locale.dates.seasons);
            if month.is_empty() {
                return None;
            }
            let day = date.day();
            if let Some(rendered) = locale.resolve_date_pattern(
                "pattern.date-month-day",
                None,
                Some(&month),
                day,
                zero_pad_day,
            ) {
                return Some(rendered);
            }
            match day {
                Some(d) => Some(format!("{month} {}", format_day(d, zero_pad_day))),
                None => Some(month),
            }
        }
        DateForm::Full => {
            let year = extract_year(date);
            if year.is_empty() {
                return None;
            }
            let month = extract_month(date, &locale.dates.months.long, &locale.dates.seasons);
            let day = date.day();
            let numeric_base = if numeric_months {
                extract_month_numeric(date).map(|month| match day {
                    Some(d) => format!("{year}-{month}-{d:02}"),
                    None => format!("{year}-{month}"),
                })
            } else {
                None
            };
            let base = numeric_base
                .or_else(|| {
                    locale.resolve_date_pattern(
                        "pattern.date-full",
                        Some(&year),
                        (!month.is_empty()).then_some(month.as_str()),
                        day,
                        zero_pad_day,
                    )
                })
                .unwrap_or_else(|| match (month.is_empty(), day) {
                    (true, _) => year.clone(),
                    (false, None) => format!("{month} {year}"),
                    (false, Some(d)) => {
                        format!("{month} {}, {year}", format_day(d, zero_pad_day))
                    }
                });
            // Append time component if configured and present
            if let (Some(time_fmt), Some(time)) = (
                date_config.and_then(|c| c.time_format.as_ref()),
                date.time(),
            ) {
                let show_secs = date_config.is_some_and(|c| c.show_seconds);
                let show_tz = date_config.is_some_and(|c| c.show_timezone);
                let time_str = format_time(
                    time,
                    time_fmt,
                    show_secs,
                    show_tz,
                    locale.dates.am.as_deref(),
                    locale.dates.pm.as_deref(),
                    locale.dates.timezone_utc.as_deref(),
                );
                Some(format!("{base}, {time_str}"))
            } else {
                Some(base)
            }
        }
        DateForm::YearMonthDay => {
            let year = extract_year(date);
            if year.is_empty() {
                return None;
            }
            if numeric_months && let Some(month) = extract_month_numeric(date) {
                return Some(match date.day() {
                    Some(d) => format!("{year}-{month}-{d:02}"),
                    None => format!("{year}-{month}"),
                });
            }
            let month = extract_month(date, &locale.dates.months.long, &locale.dates.seasons);
            let day = date.day();
            let month_opt = (!month.is_empty()).then_some(month.as_str());
            if let Some(rendered) = locale.resolve_date_pattern(
                "pattern.date-year-month-day",
                Some(&year),
                month_opt,
                day,
                zero_pad_day,
            ) {
                return Some(rendered);
            }
            match (month.is_empty(), day) {
                (true, _) => Some(year),
                (false, None) => Some(format!("{year}, {month}")),
                (false, Some(d)) => {
                    Some(format!("{year}, {month} {}", format_day(d, zero_pad_day)))
                }
            }
        }
        DateForm::DayMonthAbbrYear => {
            let year = extract_year(date);
            if year.is_empty() {
                return None;
            }
            let month = extract_month(date, &locale.dates.months.short, &locale.dates.seasons);
            let day = date.day();
            let month_opt = (!month.is_empty()).then_some(month.as_str());
            if let Some(rendered) = locale.resolve_date_pattern(
                "pattern.date-day-month-abbr-year",
                Some(&year),
                month_opt,
                day,
                zero_pad_day,
            ) {
                return Some(rendered);
            }
            match (month.is_empty(), day) {
                (true, _) => Some(year),
                (false, None) => Some(format!("{month} {year}")),
                (false, Some(d)) => Some(format!("{} {month} {year}", format_day(d, zero_pad_day))),
            }
        }
        DateForm::MonthAbbrDayYear => {
            let year = extract_year(date);
            if year.is_empty() {
                return None;
            }
            let month = extract_month(date, &locale.dates.months.short, &locale.dates.seasons);
            let day = date.day();
            let month_opt = (!month.is_empty()).then_some(month.as_str());
            if let Some(rendered) = locale.resolve_date_pattern(
                "pattern.date-month-abbr-day-year",
                Some(&year),
                month_opt,
                day,
                zero_pad_day,
            ) {
                return Some(rendered);
            }
            match (month.is_empty(), day) {
                (true, _) => Some(year),
                (false, None) => Some(format!("{month} {year}")),
                (false, Some(d)) => {
                    Some(format!("{month} {}, {year}", format_day(d, zero_pad_day)))
                }
            }
        }
        _ => Some(extract_year(date)),
    }
}

/// Render a resolved fallback component through the central component renderer.
///
/// `component.values()` only resolves the raw fallback string — it does not
/// go through the generic per-component dispatch that normally applies a
/// component's rendering. Routing the resolved value back through the central
/// renderer preserves the ordinary component contract, including suppression,
/// emphasis, quotes, strong, small caps, vertical alignment, wrapping, and
/// affixes. Shared by date and terminal contributor fallback chains.
pub(crate) fn render_fallback_component<F: crate::render::format::OutputFormat<Output = String>>(
    fmt: &F,
    component: &TemplateComponent,
    values: ProcValues<String>,
    reference: &Reference,
    options: &RenderOptions<'_>,
) -> F::Output {
    let proc_item = crate::render::ProcTemplateComponent {
        template_component: component.clone(),
        template_index: options.current_template_index,
        value: values.value,
        prefix: values.prefix,
        suffix: values.suffix,
        url: values.url,
        ref_type: Some(reference.ref_type()),
        config: Some(options.config.clone()),
        bibliography_config: options.bibliography_config.clone(),
        item_language: crate::values::effective_component_language(reference, component),
        quote_marks: crate::render::format::QuoteMarks::from(options.locale),
        sentence_initial: false,
        pre_formatted: values.pre_formatted,
    };
    crate::render::render_component_with_format_and_renderer::<F>(
        &proc_item,
        fmt,
        options.show_semantics,
    )
}

/// Render an options-level date fallback chain when an issued date is missing.
///
/// Tries each fallback candidate in order and returns the first that
/// renders. A `message:` candidate (the terminal "no data available" case,
/// e.g. GB/T 7714's `无日期`/`n.d.` term via `message: term.no-date`) and a
/// `date:` candidate (e.g. GB/T's access-year fallback, rendering
/// `Anon，[2020a]`) both need the same year-suffix-append convention the
/// first-issued disambiguation path uses, so every policy candidate
/// disambiguates identically. See csl26-6eak, csl26-huuz.
///
/// For a `date:` candidate, the letter must land inside that candidate's own
/// wrap (e.g. brackets) — so it is inlined into the raw formatted text
/// *before* `render_fallback_component` applies the wrap, not
/// appended to the already-wrapped output the way the `message:` case is.
/// Validation rejects a candidate that is itself `date: issued`. The loop also
/// skips one defensively so programmatically constructed, unvalidated styles
/// cannot recursively re-enter this fallback chain.
///
/// If nothing in the chain renders anything, the date slot itself contributes no
/// text, but the collision group this reference belongs to may still need
/// its year-suffix letter rendered standalone (upstream's bare
/// `<text variable="year-suffix"/>` after an empty date; oracle:
/// "Anon，b."). Without this, an entry whose date slot is entirely empty
/// silently loses its disambiguator rather than getting the wrong one.
fn render_date_fallback_chain<F: crate::render::format::OutputFormat<Output = String>>(
    date_component: &TemplateDate,
    fallbacks: &[DateFallbackCandidate],
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> Option<ProcValues<F::Output>> {
    let disamb_eligible = matches!(date_component.date, TemplateDateVar::Issued)
        && date_component.suppress_disamb_suffix != Some(true);

    for candidate in fallbacks {
        if matches!(
            candidate,
            DateFallbackCandidate::Date(candidate)
                if matches!(candidate.date, TemplateDateVar::Issued)
        ) {
            continue;
        }
        let component = candidate.to_template_component();
        let Some(mut values) = component.values::<F>(reference, hints, options) else {
            continue;
        };
        let substituted_key = values.substituted_key.clone();
        let suffix_label = disamb_eligible
            .then(|| compute_disamb_suffix_label(hints, options, fmt))
            .flatten();

        let inlined = match (&component, suffix_label.as_deref()) {
            (TemplateComponent::Date(inner), Some(suffix)) => {
                let year = resolve_date_variable(&inner.date, reference)
                    .map(|d| d.year())
                    .unwrap_or_default();
                values.value = inline_disamb_suffix(&values.value, &inner.form, &year, suffix);
                true
            }
            _ => false,
        };

        let mut output = render_fallback_component(fmt, &component, values, reference, options);
        if output.trim().is_empty() {
            continue;
        }
        if !inlined
            && matches!(component, TemplateComponent::Message(_))
            && let Some(suffix) = suffix_label.as_deref()
        {
            append_no_date_disamb_suffix(&mut output, suffix, options);
        }
        return Some(ProcValues {
            value: output,
            prefix: None,
            suffix: None,
            url: None,
            substituted_key,
            pre_formatted: true,
        });
    }

    disamb_eligible
        .then(|| compute_disamb_suffix_label(hints, options, fmt))
        .flatten()
        .map(|suffix| ProcValues {
            value: suffix,
            prefix: None,
            suffix: None,
            url: None,
            substituted_key: None,
            pre_formatted: true,
        })
}

impl ComponentValues for TemplateDate {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let fmt = F::default();
        let date_opt: Option<DateValue> = resolve_date_variable(&self.date, reference);

        let Some(date) = date_opt.filter(|d| !d.is_empty()) else {
            if matches!(self.date, TemplateDateVar::Issued)
                && let Some(first_issued) = hints.date_fallback_first_issued
                && let Some(fallbacks) = effective_date_fallback_candidates(
                    options.config.as_ref(),
                    first_issued,
                    &reference.ref_type(),
                )
            {
                return render_date_fallback_chain::<F>(
                    self,
                    fallbacks.as_ref(),
                    reference,
                    hints,
                    options,
                    &fmt,
                );
            }
            // A blank issued slot may still carry a year-suffix disambiguator.
            let disamb_eligible = matches!(self.date, TemplateDateVar::Issued)
                && self.suppress_disamb_suffix != Some(true);
            return disamb_eligible
                .then(|| compute_disamb_suffix_label(hints, options, &fmt))
                .flatten()
                .map(|suffix| ProcValues {
                    value: suffix,
                    prefix: None,
                    suffix: None,
                    url: None,
                    substituted_key: None,
                    pre_formatted: true,
                });
        };

        let locale = options.locale;
        let date_config = options.config.dates.as_ref();
        let effective_form = self.form.clone();

        let formatted = format_date_range(&date, &effective_form, locale, date_config);

        // Apply uncertainty and approximation markers
        let formatted = formatted.map(|value| apply_date_markers(value, &date, date_config));

        // Handle disambiguation suffix (a, b, c...).
        // Year-suffix is keyed off the issued year only; suppress it for other date
        // components (e.g. original-published) so a reprint template renders
        // `(1926/1967a)` rather than `(1926a/1967a)`.
        let disamb_suffix = (matches!(self.date, TemplateDateVar::Issued)
            && self.suppress_disamb_suffix != Some(true))
        .then(|| compute_disamb_suffix(&date, &effective_form, hints, options, &fmt))
        .flatten();

        formatted.map(|value| {
            let (value, suffix) = if let Some(ref suffix) = disamb_suffix {
                (
                    inline_disamb_suffix(&value, &effective_form, &date.year(), suffix),
                    None,
                )
            } else {
                (value, None)
            };

            let value = if self.suppress_note == Some(true) {
                value
            } else {
                append_note(&fmt, value, &date, date_config, reference, options)
            };

            ProcValues {
                value,
                prefix: None,
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

/// Convert a 1-based index into an alphabetic suffix (`1 -> "a"`, `27 -> "aa"`).
#[must_use]
pub fn int_to_letter(n: u32) -> Option<String> {
    if n == 0 {
        return None;
    }

    let mut result = String::new();
    let mut num = n - 1;

    loop {
        result.push((b'a' + (num % 26) as u8) as char);
        if num < 26 {
            break;
        }
        num = num / 26 - 1;
    }

    Some(result.chars().rev().collect())
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
    fn unmatched_date_fallback_policy_resolves_no_candidates() {
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    book: standard
"#,
        )
        .expect("date-fallback config should parse");

        assert!(effective_date_fallback_candidates(&config, true, "report").is_none());
    }

    #[test]
    fn standard_date_fallback_policy_expands_to_a_message_candidate() {
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    book: standard
"#,
        )
        .expect("date-fallback config should parse");
        let resolved = effective_date_fallback_candidates(&config, true, "book")
            .expect("matched standard rule should resolve candidates");

        assert!(matches!(resolved, Cow::Owned(_)));
        assert!(matches!(
            resolved.first(),
            Some(DateFallbackCandidate::Message(_))
        ));
    }

    #[test]
    fn test_int_to_letter() {
        // Test basic single-letter conversions (1-26)
        assert_eq!(int_to_letter(1), Some("a".to_string()));
        assert_eq!(int_to_letter(2), Some("b".to_string()));
        assert_eq!(int_to_letter(26), Some("z".to_string()));

        // Test double-letter conversions (27+)
        assert_eq!(int_to_letter(27), Some("aa".to_string()));
        assert_eq!(int_to_letter(52), Some("az".to_string()));
        assert_eq!(int_to_letter(53), Some("ba".to_string()));

        // Test zero returns None
        assert_eq!(int_to_letter(0), None);
    }

    #[test]
    fn test_apply_date_markers_uncertainty_suffix_only_by_default() {
        let date = DateValue::new("1750?");
        let config = citum_schema::options::dates::DateConfig::default();
        let result = apply_date_markers("1750".to_string(), &date, Some(&config));
        assert_eq!(result, "1750?");
    }

    #[test]
    fn test_apply_date_markers_uncertainty_paired_brackets() {
        let date = DateValue::new("1750?");
        let config = citum_schema::options::dates::DateConfig {
            uncertainty_marker: Some("?]".to_string()),
            uncertainty_marker_prefix: Some("[".to_string()),
            ..citum_schema::options::dates::DateConfig::default()
        };
        let result = apply_date_markers("1750".to_string(), &date, Some(&config));
        assert_eq!(result, "[1750?]");
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
mod time_tests {
    use super::*;
    use citum_edtf::{Time, Timezone};

    #[test]
    fn test_format_time_12h_utc() {
        let time = Time {
            hour: 23,
            minute: 20,
            second: 30,
            timezone: Some(Timezone::Utc),
        };
        let result = format_time(
            time,
            &TimeFormat::Hour12,
            false,
            true,
            Some("AM"),
            Some("PM"),
            Some("UTC"),
        );
        assert_eq!(result, "11:20 PM UTC");
    }

    #[test]
    fn test_format_time_24h_utc() {
        let time = Time {
            hour: 23,
            minute: 20,
            second: 30,
            timezone: Some(Timezone::Utc),
        };
        let result = format_time(
            time,
            &TimeFormat::Hour24,
            false,
            true,
            None,
            None,
            Some("UTC"),
        );
        assert_eq!(result, "23:20 UTC");
    }

    #[test]
    fn test_format_time_with_offset() {
        let time = Time {
            hour: 10,
            minute: 10,
            second: 10,
            timezone: Some(Timezone::Offset(330)),
        };
        let result = format_time(
            time,
            &TimeFormat::Hour24,
            false,
            true,
            None,
            None,
            Some("UTC"),
        );
        assert_eq!(result, "10:10 +05:30");
    }

    #[test]
    fn test_format_time_no_timezone() {
        let time = Time {
            hour: 14,
            minute: 30,
            second: 0,
            timezone: None,
        };
        let result = format_time(time, &TimeFormat::Hour24, false, false, None, None, None);
        assert_eq!(result, "14:30");
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
mod era_tests {
    use super::*;
    use citum_edtf::{UnspecifiedYear, Year};
    use citum_schema::locale::{DateTerms, Locale};
    use citum_schema::options::dates::{EraLabels, NegativeUnspecifiedYears};

    fn en_terms() -> DateTerms {
        Locale::en_us().dates
    }

    #[test]
    fn positive_year_default_no_suffix() {
        let year = Year {
            value: 54,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "54");
    }

    #[test]
    fn positive_year_bc_ad() {
        let year = Year {
            value: 54,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::BcAd,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "54 AD");
    }

    #[test]
    fn positive_year_bce_ce() {
        let year = Year {
            value: 54,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::BceCe,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "54 CE");
    }

    #[test]
    fn negative_year_default() {
        let year = Year {
            value: -43,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "44 BC");
    }

    #[test]
    fn negative_year_bc_ad() {
        let year = Year {
            value: -43,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::BcAd,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "44 BC");
    }

    #[test]
    fn negative_year_bce_ce() {
        let year = Year {
            value: -43,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::BceCe,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "44 BCE");
    }

    #[test]
    fn positive_unspecified_ones() {
        let year = Year {
            value: 1990,
            unspecified: UnspecifiedYear::One,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "199X");
    }

    #[test]
    fn positive_unspecified_two() {
        let year = Year {
            value: 1900,
            unspecified: UnspecifiedYear::Two,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "19XX");
    }

    #[test]
    fn negative_unspecified_range() {
        let year = Year {
            value: -90,
            unspecified: UnspecifiedYear::One,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "100–91 BC");
    }

    #[test]
    fn negative_unspecified_century() {
        let year = Year {
            value: 0,
            unspecified: UnspecifiedYear::Two,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "100–1 BC");
    }

    #[test]
    fn backwards_compat_negative_year() {
        let year = Year {
            value: -99,
            unspecified: UnspecifiedYear::None,
        };
        let result = format_display_year(
            &year,
            &en_terms(),
            &EraLabels::Default,
            &NegativeUnspecifiedYears::Range,
            "–",
        );
        assert_eq!(result, "100 BC");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Panicking is acceptable in tests."
)]
mod locale_pattern_tests {
    use super::*;
    use citum_schema::locale::Locale;

    fn en_us() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/en-US.yaml"))
            .expect("en-US locale should parse")
    }

    fn es_es() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/es-ES.yaml"))
            .expect("es-ES locale should parse")
    }

    fn eu_es() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/eu-ES.yaml"))
            .expect("eu-ES locale should parse")
    }

    fn full(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::Full,
            locale,
            None,
        )
        .expect("date should render")
    }

    fn month_day(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::MonthDay,
            locale,
            None,
        )
        .expect("date should render")
    }

    #[test]
    fn en_us_full_unchanged_by_pattern_machinery() {
        // Regression: en-US declares no pattern.date-*, so the engine's
        // hardcoded English assembly must still produce the original output.
        assert_eq!(full(&en_us(), "2023-01-12"), "January 12, 2023");
    }

    #[test]
    fn en_us_month_day_unchanged_by_pattern_machinery() {
        assert_eq!(month_day(&en_us(), "2023-01-12"), "January 12");
    }

    #[test]
    fn en_us_month_form_renders_month_name_only() {
        // given a year-month date and the month-only form
        let out = format_single_date(
            &DateValue::new("2023-06".to_string()),
            &DateForm::Month,
            &en_us(),
            None,
        );
        // then only the month name renders (no year), e.g. magazines
        assert_eq!(out.as_deref(), Some("June"));
    }

    #[test]
    fn en_us_month_form_renders_season_name() {
        // given an EDTF season date and the month-only form
        let out = format_single_date(
            &DateValue::new("2023-21".to_string()),
            &DateForm::Month,
            &en_us(),
            None,
        );
        // then the locale's season term renders in place of a month name
        assert_eq!(out.as_deref(), Some("Spring"));
    }

    #[test]
    fn en_us_year_month_form_renders_season_and_year() {
        let out = format_single_date(
            &DateValue::new("2023-21".to_string()),
            &DateForm::YearMonth,
            &en_us(),
            None,
        );
        assert_eq!(out.as_deref(), Some("Spring 2023"));
    }

    #[test]
    fn en_us_full_form_renders_season_and_year() {
        assert_eq!(full(&en_us(), "2023-21"), "Spring 2023");
    }

    /// `day-zero-pad` is off by default, so a plain `format_single_date` call
    /// (no `DateConfig`) renders the day unpadded — the pre-existing
    /// behavior this option must not change unless a style opts in.
    #[test]
    fn day_zero_pad_defaults_to_unpadded_day() {
        assert_eq!(full(&en_us(), "2023-02-07"), "February 7, 2023");
    }

    /// `day-zero-pad: true` zero-pads the day across every day-bearing
    /// single-date form, not only `Full`.
    #[test]
    fn day_zero_pad_true_pads_day_in_full_and_month_day_forms() {
        let config = citum_schema::options::dates::DateConfig {
            day_zero_pad: true,
            ..Default::default()
        };
        assert_eq!(
            format_single_date(
                &DateValue::new("2023-02-07".to_string()),
                &DateForm::Full,
                &en_us(),
                Some(&config)
            ),
            Some("February 07, 2023".to_string())
        );
        assert_eq!(
            format_single_date(
                &DateValue::new("2023-02-07".to_string()),
                &DateForm::MonthDay,
                &en_us(),
                Some(&config)
            ),
            Some("February 07".to_string())
        );
    }

    /// The range-fragment path (`format_abbreviated_month_day_fragment`,
    /// used by same-year date-range rendering) is a separate code path from
    /// `format_single_date` and must honor `day-zero-pad` independently —
    /// this is the surface most likely to be missed when wiring the option.
    #[test]
    fn day_zero_pad_true_pads_day_in_abbreviated_range_fragment() {
        let config = citum_schema::options::dates::DateConfig {
            day_zero_pad: true,
            ..Default::default()
        };
        let date = DateValue::new("2023-02-07".to_string());
        assert_eq!(
            format_abbreviated_month_day_fragment(
                &date,
                &DateForm::DayMonthAbbrYear,
                &en_us(),
                Some(&config)
            ),
            Some("07 Feb.".to_string())
        );
        assert_eq!(
            format_abbreviated_month_day_fragment(
                &date,
                &DateForm::MonthAbbrDayYear,
                &en_us(),
                Some(&config)
            ),
            Some("Feb. 07".to_string())
        );
    }

    /// The bean's own reported example: a `LocaleOverride` replacing a
    /// single short-month abbreviation, combined with `day-zero-pad`,
    /// renders "Jul. 13, 2021" instead of the base locale's "July 13, 2021"
    /// — without redeclaring the other eleven months.
    #[test]
    fn locale_override_month_abbreviation_with_day_zero_pad_matches_bean_example() {
        use citum_schema::locale::{DateNameOverride, LocaleOverride, MonthNames, SubYearCode};

        let mut locale = en_us();
        let july = SubYearCode::new(7).expect("valid month code");
        assert_eq!(
            locale.dates.months.short.get(&july).map(String::as_str),
            Some("July"),
            "sanity: base locale renders July's short form unabbreviated"
        );

        // Override to IEEE's tighter form and confirm June is untouched.
        let ov = LocaleOverride {
            dates: DateNameOverride {
                months: MonthNames {
                    long: std::collections::BTreeMap::new(),
                    short: [(july, "Jul.".to_string())].into(),
                },
                seasons: std::collections::BTreeMap::new(),
            },
            ..Default::default()
        };
        locale.apply_override(&ov);

        let config = citum_schema::options::dates::DateConfig {
            day_zero_pad: true,
            ..Default::default()
        };
        assert_eq!(
            format_single_date(
                &DateValue::new("2021-07-13".to_string()),
                &DateForm::MonthAbbrDayYear,
                &locale,
                Some(&config)
            ),
            Some("Jul. 13, 2021".to_string())
        );
        let june = SubYearCode::new(6).expect("valid month code");
        assert_eq!(
            locale.dates.months.short.get(&june).map(String::as_str),
            Some("June")
        );
    }

    #[test]
    fn es_es_year_month_form_renders_localized_season() {
        let out = format_single_date(
            &DateValue::new("2023-23".to_string()),
            &DateForm::YearMonth,
            &es_es(),
            None,
        );
        assert_eq!(out.as_deref(), Some("otoño de 2023"));
    }

    #[test]
    fn es_es_full_uses_locale_pattern() {
        // Spanish day-first assembly via pattern.date-full.
        assert_eq!(full(&es_es(), "2023-01-12"), "12 de enero de 2023");
    }

    #[test]
    fn es_es_month_day_uses_locale_pattern() {
        assert_eq!(month_day(&es_es(), "2023-01-12"), "12 de enero");
    }

    #[test]
    fn eu_es_full_uses_locale_pattern() {
        // Basque genitive-absolutive shape via pattern.date-full.
        // Content is PROVISIONAL — see locales/eu-ES.yaml header comment.
        assert_eq!(full(&eu_es(), "2023-01-12"), "2023ko urtarrilaren 12a");
    }

    #[test]
    fn eu_es_month_day_uses_locale_pattern() {
        assert_eq!(month_day(&eu_es(), "2023-01-12"), "urtarrilaren 12a");
    }

    fn year_month(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::YearMonth,
            locale,
            None,
        )
        .expect("date should render")
    }

    fn year_month_day(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::YearMonthDay,
            locale,
            None,
        )
        .expect("date should render")
    }

    fn day_month_abbr_year(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::DayMonthAbbrYear,
            locale,
            None,
        )
        .expect("date should render")
    }

    fn month_abbr_day_year(locale: &Locale, edtf: &str) -> String {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &DateForm::MonthAbbrDayYear,
            locale,
            None,
        )
        .expect("date should render")
    }

    #[test]
    fn en_us_year_month_unchanged_by_pattern_machinery() {
        // en-US has no pattern.date-year-month, so hardcoded assembly must hold.
        assert_eq!(year_month(&en_us(), "2023-01"), "January 2023");
    }

    #[test]
    fn en_us_year_month_day_unchanged_by_pattern_machinery() {
        assert_eq!(year_month_day(&en_us(), "2023-01-12"), "2023, January 12");
    }

    #[test]
    fn en_us_day_month_abbr_year_unchanged_by_pattern_machinery() {
        assert_eq!(day_month_abbr_year(&en_us(), "2023-01-12"), "12 Jan. 2023");
    }

    #[test]
    fn en_us_month_abbr_day_year_unchanged_by_pattern_machinery() {
        assert_eq!(month_abbr_day_year(&en_us(), "2023-01-12"), "Jan. 12, 2023");
    }

    #[test]
    fn es_es_year_month_uses_locale_pattern() {
        // Spanish: month before year connected with "de".
        assert_eq!(year_month(&es_es(), "2023-01"), "enero de 2023");
    }

    #[test]
    fn eu_es_year_month_uses_locale_pattern() {
        // Basque: year-first genitive shape. PROVISIONAL — see locales/eu-ES.yaml.
        assert_eq!(year_month(&eu_es(), "2023-01"), "2023ko urtarrila");
    }

    #[test]
    fn year_month_missing_month_falls_back_to_year() {
        // Year-only EDTF: no month to pattern-assemble, returns year alone.
        assert_eq!(year_month(&es_es(), "2023"), "2023");
    }

    #[test]
    fn es_es_year_month_day_uses_locale_pattern() {
        // Spanish: year first, then day/month connected with "de".
        assert_eq!(year_month_day(&es_es(), "2023-01-12"), "2023, 12 de enero");
    }

    #[test]
    fn es_es_year_month_day_missing_day_falls_back() {
        // Pattern requires $day; evaluator returns None, falls back to
        // hardcoded "{year}, {month}".
        assert_eq!(year_month_day(&es_es(), "2023-01"), "2023, enero");
    }

    #[test]
    fn es_es_day_month_abbr_year_uses_locale_pattern() {
        // Spanish abbreviated form: "12 ene. de 2023" via pattern.
        assert_eq!(
            day_month_abbr_year(&es_es(), "2023-01-12"),
            "12 ene. de 2023"
        );
    }

    #[test]
    fn es_es_day_month_abbr_year_missing_day_falls_back() {
        // Pattern requires $day; falls back to hardcoded "{month} {year}".
        assert_eq!(day_month_abbr_year(&es_es(), "2023-01"), "ene. 2023");
    }

    #[test]
    fn es_es_month_abbr_day_year_uses_locale_pattern() {
        // Spanish abbreviated form: "ene. 12 de 2023" via pattern.
        assert_eq!(
            month_abbr_day_year(&es_es(), "2023-01-12"),
            "ene. 12 de 2023"
        );
    }

    #[test]
    fn es_es_month_abbr_day_year_missing_day_falls_back() {
        // Pattern requires $day; falls back to hardcoded "{month} {year}".
        assert_eq!(month_abbr_day_year(&es_es(), "2023-01"), "ene. 2023");
    }

    #[test]
    fn pattern_missing_day_falls_back_to_english_assembly() {
        // Year-month only input: pattern.date-full requires {$day} so the
        // evaluator returns None, and the engine falls through to its
        // hardcoded `{month} {year}` assembly. (A future pattern.date-year-month
        // can fix this for inflected locales — out of scope for this bean.)
        assert_eq!(full(&es_es(), "2023-01"), "enero 2023");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Panicking is acceptable in tests."
)]
mod numeric_month_tests {
    use super::*;
    use citum_schema::locale::Locale;
    use citum_schema::options::MonthFormat;
    use citum_schema::options::dates::DateConfig;

    fn en_us() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/en-US.yaml"))
            .expect("en-US locale should parse")
    }

    fn numeric_config() -> DateConfig {
        DateConfig {
            month: MonthFormat::Numeric,
            ..Default::default()
        }
    }

    fn render(form: DateForm, edtf: &str) -> Option<String> {
        format_single_date(
            &DateValue::new(edtf.to_string()),
            &form,
            &en_us(),
            Some(&numeric_config()),
        )
    }

    #[test]
    fn given_month_numeric_when_year_month_day_then_iso_hyphenated() {
        // GB/T 7714 / ISO 690 access and update dates: [2024-01-15].
        assert_eq!(
            render(DateForm::YearMonthDay, "2024-01-15").as_deref(),
            Some("2024-01-15")
        );
    }

    #[test]
    fn given_month_numeric_when_day_missing_then_year_month_only() {
        assert_eq!(
            render(DateForm::YearMonthDay, "2024-01").as_deref(),
            Some("2024-01")
        );
    }

    #[test]
    fn given_month_numeric_when_year_only_then_plain_year() {
        assert_eq!(
            render(DateForm::YearMonthDay, "2024").as_deref(),
            Some("2024")
        );
    }

    #[test]
    fn given_month_numeric_when_year_month_form_then_hyphenated() {
        assert_eq!(
            render(DateForm::YearMonth, "2024-03").as_deref(),
            Some("2024-03")
        );
    }

    #[test]
    fn given_month_numeric_when_month_day_form_then_zero_padded() {
        assert_eq!(
            render(DateForm::MonthDay, "2024-03-05").as_deref(),
            Some("03-05")
        );
    }

    #[test]
    fn given_month_numeric_when_full_form_then_iso_hyphenated() {
        assert_eq!(
            render(DateForm::Full, "2024-01-15").as_deref(),
            Some("2024-01-15")
        );
    }

    #[test]
    fn given_month_numeric_when_season_date_then_textual_fallback() {
        // Seasons have no numeric month; the textual path must still render.
        assert_eq!(
            render(DateForm::YearMonth, "2024-22").as_deref(),
            Some("Summer 2024")
        );
    }

    #[test]
    fn given_long_month_config_when_year_month_day_then_unchanged() {
        // Regression guard: the default textual assembly is untouched.
        let out = format_single_date(
            &DateValue::new("2024-01-15".to_string()),
            &DateForm::YearMonthDay,
            &en_us(),
            None,
        );
        assert_eq!(out.as_deref(), Some("2024, January 15"));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Panicking is acceptable in tests."
)]
mod range_tests {
    use super::*;
    use citum_schema::locale::Locale;
    use citum_schema::options::dates::{DateConfig, EraLabels};

    fn en_us() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/en-US.yaml"))
            .expect("en-US locale should parse")
    }

    fn es_es() -> Locale {
        Locale::from_yaml_str(include_str!("../../../../locales/es-ES.yaml"))
            .expect("es-ES locale should parse")
    }

    fn range(locale: &Locale, edtf: &str, form: DateForm) -> Option<String> {
        format_date_range(&DateValue::new(edtf.to_string()), &form, locale, None)
    }

    fn chicago_range(locale: &Locale, edtf: &str, form: DateForm) -> Option<String> {
        let config = DateConfig {
            range_format: DateRangeFormat::Chicago,
            ..Default::default()
        };
        format_date_range(
            &DateValue::new(edtf.to_string()),
            &form,
            locale,
            Some(&config),
        )
    }

    fn range_with_config(
        locale: &Locale,
        edtf: &str,
        form: DateForm,
        config: &DateConfig,
    ) -> Option<String> {
        format_date_range(
            &DateValue::new(edtf.to_string()),
            &form,
            locale,
            Some(config),
        )
    }

    #[test]
    fn closed_range_year_form_regression() {
        // given a closed range with distinct years and the Year form
        // then it renders as a plain year-to-year range (no collapse)
        assert_eq!(
            range(&en_us(), "2020/2022", DateForm::Year).as_deref(),
            Some("2020–2022")
        );
    }

    #[test]
    fn chicago_year_range_condenses_the_end_year() {
        assert_eq!(
            chicago_range(&en_us(), "2021/2026", DateForm::Year).as_deref(),
            Some("2021–26")
        );
    }

    #[test]
    fn chicago_range_format_keeps_cross_year_month_ranges_expanded() {
        assert_eq!(
            chicago_range(&en_us(), "2021-05/2026-06", DateForm::YearMonth).as_deref(),
            Some("May 2021–June 2026")
        );
    }

    #[test]
    fn shared_year_month_range_uses_spanish_mf2_pattern() {
        assert_eq!(
            range(&es_es(), "2026-05/2026-06", DateForm::YearMonth).as_deref(),
            Some("mayo a junio, 2026")
        );
    }

    #[test]
    fn shared_year_full_range_uses_spanish_mf2_pattern() {
        assert_eq!(
            range(&es_es(), "2026-05-14/2026-06-02", DateForm::Full).as_deref(),
            Some("14 de mayo a 2 de junio de 2026")
        );
    }

    #[test]
    fn chicago_year_range_condenses_same_era_bce_years() {
        let config = DateConfig {
            range_format: DateRangeFormat::Chicago,
            era_labels: EraLabels::BceCe,
            ..Default::default()
        };
        assert_eq!(
            range_with_config(&en_us(), "-0326/-0020", DateForm::Year, &config).as_deref(),
            Some("327–21 BCE")
        );
    }

    #[test]
    fn expanded_same_era_bce_years_keep_both_endpoints() {
        let config = DateConfig {
            era_labels: EraLabels::BceCe,
            ..Default::default()
        };
        assert_eq!(
            range_with_config(&en_us(), "-0326/-0020", DateForm::Year, &config).as_deref(),
            Some("327 BCE–21 BCE")
        );
    }

    #[test]
    fn chicago_year_range_preserves_cross_era_endpoints() {
        let config = DateConfig {
            range_format: DateRangeFormat::Chicago,
            era_labels: EraLabels::BcAd,
            ..Default::default()
        };
        assert_eq!(
            range_with_config(&en_us(), "-0114/0010", DateForm::Year, &config).as_deref(),
            Some("115 BC–10 AD")
        );
    }

    #[test]
    fn chicago_year_range_keeps_reversed_input_expanded() {
        assert_eq!(
            chicago_range(&en_us(), "2026/2021", DateForm::Year).as_deref(),
            Some("2026–2021")
        );
    }

    #[test]
    fn chicago_range_format_does_not_condense_non_four_digit_or_unspecified_years() {
        assert_eq!(
            chicago_range(&en_us(), "0999/1000", DateForm::Year).as_deref(),
            Some("999–1000")
        );
        assert_eq!(
            chicago_range(&en_us(), "202u/203u", DateForm::Year).as_deref(),
            Some("202X–203X")
        );
    }

    #[test]
    fn closed_range_full_form_different_years() {
        // given a closed range spanning two years, Full form
        // then both endpoints render in full
        assert_eq!(
            range(&en_us(), "2023-05-14/2024-06-02", DateForm::Full).as_deref(),
            Some("May 14, 2023–June 2, 2024")
        );
    }

    #[test]
    fn closed_range_full_form_same_year_collapses() {
        // given a closed range within a single year, Full form
        // then the start's year is suppressed and trails the end instead
        assert_eq!(
            range(&en_us(), "2023-05-14/2023-06-02", DateForm::Full).as_deref(),
            Some("May 14–June 2, 2023")
        );
    }

    #[test]
    fn closed_range_year_month_day_same_year_collapses() {
        // given a closed range within a single year, YearMonthDay form
        // then the leading year renders once and the end's year is suppressed
        assert_eq!(
            range(&en_us(), "2023-05-14/2023-06-02", DateForm::YearMonthDay).as_deref(),
            Some("2023, May 14–June 2")
        );
    }

    #[test]
    fn closed_range_full_form_es_es_locale_pattern() {
        // given a closed range spanning two years under a locale that
        // declares pattern.date-full
        // then both endpoints render through the Spanish pattern
        assert_eq!(
            range(&es_es(), "2023-01-12/2024-02-03", DateForm::Full).as_deref(),
            Some("12 de enero de 2023–3 de febrero de 2024")
        );
    }

    #[test]
    fn interval_to_year_form() {
        // given an open-ended-from-start range ("../2020")
        // then it renders as the single known (end) point
        assert_eq!(
            range(&en_us(), "../2020", DateForm::Year).as_deref(),
            Some("2020")
        );
    }

    #[test]
    fn closed_range_year_month_same_year_collapses() {
        // given month-only endpoints in the same year, YearMonth form
        // then the start month renders without the (shared) year
        assert_eq!(
            range(&en_us(), "2023-05/2023-06", DateForm::YearMonth).as_deref(),
            Some("May–June 2023")
        );
    }

    #[test]
    fn chicago_range_format_keeps_same_year_month_ranges_locale_driven() {
        assert_eq!(
            chicago_range(&en_us(), "2026-05/2026-06", DateForm::YearMonth).as_deref(),
            Some("May–June 2026")
        );
    }

    #[test]
    fn closed_range_season_same_year_collapses() {
        // given EDTF season endpoints in the same year, YearMonth form
        // then the start season renders without the (shared) year
        assert_eq!(
            range(&en_us(), "2023-21/2023-22", DateForm::YearMonth).as_deref(),
            Some("Spring–Summer 2023")
        );
    }
}
