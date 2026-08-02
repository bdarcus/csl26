/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! `citum_edtf` - A modern EDTF (Extended Date/Time Format) parser
//!
//! This crate implements ISO 8601-2:2019 (EDTF) Level 0 and Level 1.

use winnow::ascii::dec_int;
use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::take;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents the top-level EDTF value.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Edtf {
    /// A single date.
    Date(Date),
    /// A date interval.
    Interval(Interval),
    /// An open-ended interval starting at a specific date.
    IntervalFrom(Date),
    /// An open-ended interval ending at a specific date.
    IntervalTo(Date),
}

impl Edtf {
    /// Extract the year component. For intervals, this is the start year.
    pub fn year(&self) -> i64 {
        match self {
            Self::Date(date) => date.year.value,
            Self::Interval(interval) => interval.start.year.value,
            Self::IntervalFrom(date) => date.year.value,
            Self::IntervalTo(date) => date.year.value,
        }
    }

    /// Extract the month component if present. For intervals, this is the start month.
    pub fn month(&self) -> Option<u32> {
        let m_opt = match self {
            Self::Date(date) => date.month_or_season,
            Self::Interval(interval) => interval.start.month_or_season,
            Self::IntervalFrom(date) => date.month_or_season,
            Self::IntervalTo(date) => date.month_or_season,
        };
        match m_opt {
            Some(MonthOrSeason::Month(m)) => Some(m),
            _ => None,
        }
    }

    /// Extract the EDTF Level 1 season sub-year code (`21`=Spring,
    /// `22`=Summer, `23`=Autumn, `24`=Winter) if present. For intervals,
    /// this is the start season.
    ///
    /// Returns the raw EDTF code rather than a 1-based index so callers can
    /// use it directly as a key alongside calendar-month codes (`1`-`12`)
    /// in a single sub-year keyspace. See
    /// `docs/specs/LOCALE_DATE_NAME_KEYING.md`.
    pub fn season_code(&self) -> Option<u32> {
        let m_opt = match self {
            Self::Date(date) => date.month_or_season,
            Self::Interval(interval) => interval.start.month_or_season,
            Self::IntervalFrom(date) => date.month_or_season,
            Self::IntervalTo(date) => date.month_or_season,
        };
        match m_opt {
            Some(MonthOrSeason::Spring) => Some(21),
            Some(MonthOrSeason::Summer) => Some(22),
            Some(MonthOrSeason::Autumn) => Some(23),
            Some(MonthOrSeason::Winter) => Some(24),
            _ => None,
        }
    }

    /// Extract the day component if present. For intervals, this is the start day.
    pub fn day(&self) -> Option<u32> {
        let d_opt = match self {
            Self::Date(date) => date.day,
            Self::Interval(interval) => interval.start.day,
            Self::IntervalFrom(date) => date.day,
            Self::IntervalTo(date) => date.day,
        };
        match d_opt {
            Some(Day::Day(d)) => Some(d),
            _ => None,
        }
    }

    /// Check if this is a range (interval).
    pub fn is_range(&self) -> bool {
        matches!(
            self,
            Self::Interval(_) | Self::IntervalFrom(_) | Self::IntervalTo(_)
        )
    }

    /// Check if the range is open-ended (ends with "..").
    pub fn is_open_range(&self) -> bool {
        matches!(self, Self::IntervalFrom(_))
    }

    /// Extract the time component from the date, if present.
    pub fn time(&self) -> Option<Time> {
        match self {
            Self::Date(date) => date.time,
            _ => None,
        }
    }

    /// Check if the date has a time component.
    pub fn has_time(&self) -> bool {
        self.time().is_some()
    }
}

/// A date interval.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Interval {
    /// The starting date of the interval.
    pub start: Date,
    /// The ending date of the interval.
    pub end: Date,
}

/// Represents either a calendar month or an EDTF Level 1 season code.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MonthOrSeason {
    /// A calendar month value from `1` through `12`.
    Month(u32),
    /// An unspecified month marker such as `uu` or `XX`.
    Unspecified,
    /// The EDTF season code `21`.
    Spring,
    /// The EDTF season code `22`.
    Summer,
    /// The EDTF season code `23`.
    Autumn,
    /// The EDTF season code `24`.
    Winter,
}

/// Records EDTF uncertainty and approximation qualifiers for one date component.
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Quality {
    /// Whether the component is marked uncertain with `?`.
    pub uncertain: bool,
    /// Whether the component is marked approximate with `~`.
    pub approximate: bool,
}

/// A year in an EDTF date, which may contain unspecified digits.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Year {
    /// The numeric year value with unspecified digits normalized to `0`.
    pub value: i64,
    /// The number of trailing unspecified digits represented in the source.
    pub unspecified: UnspecifiedYear,
}

/// Unspecified digits in a year (EDTF Level 1).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnspecifiedYear {
    #[default]
    /// The year is fully specified.
    None,
    /// One unspecified digit (e.g., 199u)
    One,
    /// Two unspecified digits (e.g., 19uu)
    Two,
    /// Three unspecified digits (e.g., 1uuu)
    Three,
    /// Four unspecified digits (e.g., uuuu)
    Four,
}

/// A day in an EDTF date, which may be unspecified.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Day {
    /// A calendar day value from `1` through `31`.
    Day(u32),
    /// An unspecified day marker such as `uu` or `XX`.
    Unspecified,
}

/// Stores a parsed EDTF date or datetime with per-component quality markers.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Date {
    /// The parsed year component.
    pub year: Year,
    /// Qualifiers that apply to the year component.
    pub year_quality: Quality,
    /// The parsed month or season component, if present.
    pub month_or_season: Option<MonthOrSeason>,
    /// Qualifiers that apply to the month or season component.
    pub month_quality: Quality,
    /// The parsed day component, if present.
    pub day: Option<Day>,
    /// Qualifiers that apply to the day component.
    pub day_quality: Quality,
    /// The parsed time component, if present.
    pub time: Option<Time>,
}

/// Timezone specification for an EDTF datetime.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Timezone {
    /// UTC (Z suffix)
    Utc,
    /// Offset in minutes from UTC (positive = east, negative = west)
    Offset(i16),
}

/// Stores a basic ISO 8601 time with an optional timezone offset.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Time {
    /// The hour component in the range `0..=23`.
    pub hour: u32,
    /// The minute component in the range `0..=59`.
    pub minute: u32,
    /// The second component in the range `0..=59`.
    pub second: u32,
    /// The parsed timezone designator, if present.
    pub timezone: Option<Timezone>,
}

use std::fmt;

/// Error returned when an EDTF string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid EDTF: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl std::str::FromStr for Edtf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut input = s;
        let edtf = parse(&mut input).map_err(|e| ParseError(e.to_string()))?;
        if !input.is_empty() {
            return Err(ParseError(format!("unexpected trailing input: {input}")));
        }
        Ok(edtf)
    }
}

impl std::str::FromStr for Date {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut input = s;
        let date = parse_date(&mut input).map_err(|e| ParseError(e.to_string()))?;
        if !input.is_empty() {
            return Err(ParseError(format!("unexpected trailing input: {input}")));
        }
        Ok(date)
    }
}

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Edtf::Date(d) => write!(f, "{d}"),
            Edtf::Interval(i) => write!(f, "{}/{}", i.start, i.end),
            Edtf::IntervalFrom(d) => write!(f, "{d}/.."),
            Edtf::IntervalTo(d) => write!(f, "../{d}"),
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.year, self.year_quality)?;
        if let Some(m) = self.month_or_season {
            write!(f, "-{}{}", m, self.month_quality)?;
            if let Some(d) = self.day {
                write!(f, "-{}{}", d, self.day_quality)?;
            }
        }
        if let Some(t) = self.time {
            write!(f, "T{t}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value > 9999 || self.value < -9999 {
            write!(f, "Y{}", self.value)
        } else if self.value < 0 {
            let abs_val = self.value.abs();
            let mut s = format!("{abs_val:04}");
            match self.unspecified {
                UnspecifiedYear::None => write!(f, "-{s}"),
                UnspecifiedYear::One => {
                    s.replace_range(3..4, "u");
                    write!(f, "-{s}")
                }
                UnspecifiedYear::Two => {
                    s.replace_range(2..4, "uu");
                    write!(f, "-{s}")
                }
                UnspecifiedYear::Three => {
                    s.replace_range(1..4, "uuu");
                    write!(f, "-{s}")
                }
                UnspecifiedYear::Four => {
                    s.replace_range(0..4, "uuuu");
                    write!(f, "-{s}")
                }
            }
        } else {
            let mut s = format!("{:04}", self.value);
            match self.unspecified {
                UnspecifiedYear::None => write!(f, "{s}"),
                UnspecifiedYear::One => {
                    s.replace_range(3..4, "u");
                    write!(f, "{s}")
                }
                UnspecifiedYear::Two => {
                    s.replace_range(2..4, "uu");
                    write!(f, "{s}")
                }
                UnspecifiedYear::Three => {
                    s.replace_range(1..4, "uuu");
                    write!(f, "{s}")
                }
                UnspecifiedYear::Four => {
                    s.replace_range(0..4, "uuuu");
                    write!(f, "{s}")
                }
            }
        }
    }
}

impl fmt::Display for MonthOrSeason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonthOrSeason::Month(m) => write!(f, "{m:02}"),
            MonthOrSeason::Unspecified => write!(f, "uu"),
            MonthOrSeason::Spring => write!(f, "21"),
            MonthOrSeason::Summer => write!(f, "22"),
            MonthOrSeason::Autumn => write!(f, "23"),
            MonthOrSeason::Winter => write!(f, "24"),
        }
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Day::Day(d) => write!(f, "{d:02}"),
            Day::Unspecified => write!(f, "uu"),
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        match self.timezone {
            Some(Timezone::Utc) => write!(f, "Z"),
            Some(Timezone::Offset(mins)) => {
                let sign = if mins >= 0 { '+' } else { '-' };
                let abs = mins.unsigned_abs();
                write!(f, "{}{:02}:{:02}", sign, abs / 60, abs % 60)
            }
            None => Ok(()),
        }
    }
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.uncertain, self.approximate) {
            (true, true) => write!(f, "%"),
            (true, false) => write!(f, "?"),
            (false, true) => write!(f, "~"),
            (false, false) => Ok(()),
        }
    }
}

fn parse_quality(input: &mut &str) -> Result<Quality, ErrMode<ContextError>> {
    let qualifier = opt(alt(('?', '~', '%'))).parse_next(input)?;
    Ok(match qualifier {
        Some('?') => Quality {
            uncertain: true,
            approximate: false,
        },
        Some('~') => Quality {
            uncertain: false,
            approximate: true,
        },
        Some('%') => Quality {
            uncertain: true,
            approximate: true,
        },
        _ => Quality::default(),
    })
}

fn parse_year(input: &mut &str) -> Result<Year, ErrMode<ContextError>> {
    if input.starts_with('Y') {
        let _ = 'Y'.parse_next(input)?;
        let value: i64 = dec_int.parse_next(input)?;
        return Ok(Year {
            value,
            unspecified: UnspecifiedYear::None,
        });
    }

    let sign = opt(alt(('-', '+'))).parse_next(input)?;
    let s = take(4_usize).parse_next(input)?;

    let mut value_str = String::with_capacity(4);
    let mut unspecified_count = 0;

    for c in s.chars() {
        if c == 'u' || c == 'X' {
            value_str.push('0');
            unspecified_count += 1;
        } else if c.is_ascii_digit() {
            value_str.push(c);
        } else {
            return Err(ErrMode::Backtrack(ContextError::default()));
        }
    }

    let mut value = value_str
        .parse::<i64>()
        .map_err(|_| ErrMode::Backtrack(ContextError::default()))?;

    if let Some('-') = sign {
        value = -value;
    }

    let unspecified = match unspecified_count {
        0 => UnspecifiedYear::None,
        1 => UnspecifiedYear::One,
        2 => UnspecifiedYear::Two,
        3 => UnspecifiedYear::Three,
        4 => UnspecifiedYear::Four,
        _ => return Err(ErrMode::Backtrack(ContextError::default())),
    };

    Ok(Year { value, unspecified })
}

fn parse_month_or_season(input: &mut &str) -> Result<MonthOrSeason, ErrMode<ContextError>> {
    let s = take(2_usize).parse_next(input)?;
    if s == "uu" || s == "XX" {
        return Ok(MonthOrSeason::Unspecified);
    }

    let val: u32 = s
        .parse()
        .map_err(|_| ErrMode::Backtrack(ContextError::default()))?;

    match val {
        1..=12 => Ok(MonthOrSeason::Month(val)),
        21 => Ok(MonthOrSeason::Spring),
        22 => Ok(MonthOrSeason::Summer),
        23 => Ok(MonthOrSeason::Autumn),
        24 => Ok(MonthOrSeason::Winter),
        _ => Err(ErrMode::Backtrack(ContextError::default())),
    }
}

fn parse_day(input: &mut &str) -> Result<Day, ErrMode<ContextError>> {
    let s = take(2_usize).parse_next(input)?;
    if s == "uu" || s == "XX" {
        return Ok(Day::Unspecified);
    }

    let val: u32 = s
        .parse()
        .map_err(|_| ErrMode::Backtrack(ContextError::default()))?;
    match val {
        1..=31 => Ok(Day::Day(val)),
        _ => Err(ErrMode::Backtrack(ContextError::default())),
    }
}

fn parse_timezone(input: &mut &str) -> Result<Option<Timezone>, ErrMode<ContextError>> {
    if input.starts_with('Z') {
        let _ = 'Z'.parse_next(input)?;
        return Ok(Some(Timezone::Utc));
    }
    if input.starts_with('+') || input.starts_with('-') {
        let sign = opt(alt(('+', '-'))).parse_next(input)?.unwrap_or('+');
        let h = take(2_usize)
            .try_map(|s: &str| s.parse::<i16>())
            .parse_next(input)?;
        let _ = ':'.parse_next(input)?;
        let m = take(2_usize)
            .try_map(|s: &str| s.parse::<i16>())
            .parse_next(input)?;
        let total = h * 60 + m;
        let offset = if sign == '-' { -total } else { total };
        return Ok(Some(Timezone::Offset(offset)));
    }
    Ok(None)
}

fn parse_time(input: &mut &str) -> Result<Time, ErrMode<ContextError>> {
    let hour = take(2_usize)
        .try_map(|s: &str| s.parse::<u32>())
        .parse_next(input)?;
    let _ = ':'.parse_next(input)?;
    let minute = take(2_usize)
        .try_map(|s: &str| s.parse::<u32>())
        .parse_next(input)?;
    let _ = ':'.parse_next(input)?;
    let second = take(2_usize)
        .try_map(|s: &str| s.parse::<u32>())
        .parse_next(input)?;
    let timezone = parse_timezone(input)?;

    if hour > 23 || minute > 59 || second > 59 {
        return Err(ErrMode::Backtrack(ContextError::default()));
    }

    Ok(Time {
        hour,
        minute,
        second,
        timezone,
    })
}

/// Parses one EDTF date or datetime from the start of `input`.
///
/// This parser consumes only the recognized prefix and leaves any remaining
/// input in `input`, following `winnow` parser conventions.
///
/// # Errors
///
/// Returns an error when the input does not begin with a valid EDTF date or
/// datetime production.
pub fn parse_date(input: &mut &str) -> Result<Date, ErrMode<ContextError>> {
    let year = parse_year.parse_next(input)?;
    let year_quality = parse_quality.parse_next(input)?;

    let month_or_season = if input.starts_with('-') {
        let _ = '-'.parse_next(input)?;
        Some(parse_month_or_season.parse_next(input)?)
    } else {
        None
    };
    let month_quality = if month_or_season.is_some() {
        parse_quality.parse_next(input)?
    } else {
        Quality::default()
    };

    let day = if let Some(MonthOrSeason::Month(_) | MonthOrSeason::Unspecified) = month_or_season {
        if input.starts_with('-') {
            let _ = '-'.parse_next(input)?;
            Some(parse_day.parse_next(input)?)
        } else {
            None
        }
    } else {
        None
    };
    let day_quality = if day.is_some() {
        parse_quality.parse_next(input)?
    } else {
        Quality::default()
    };

    let time = if input.starts_with('T') {
        let _ = 'T'.parse_next(input)?;
        Some(parse_time.parse_next(input)?)
    } else {
        None
    };

    // Final check: if the last component parsed didn't have a quality marker,
    // but there is one at the end of the string, it applies to the whole thing?
    // Actually, ISO 8601-2 says it applies to what's on the left.
    // If we have "2004-06-11?", it applies to "11".

    Ok(Date {
        year,
        year_quality,
        month_or_season,
        month_quality,
        day,
        day_quality,
        time,
    })
}

/// Parses one top-level EDTF value from the start of `input`.
///
/// This parser accepts a single date, a closed interval, or an open-ended
/// interval and leaves any unconsumed suffix in `input`.
///
/// # Errors
///
/// Returns an error when the input does not begin with a valid EDTF date or
/// interval production.
pub fn parse(input: &mut &str) -> Result<Edtf, ErrMode<ContextError>> {
    if input.starts_with("../") {
        let _ = "../".parse_next(input)?;
        let date = parse_date.parse_next(input)?;
        return Ok(Edtf::IntervalTo(date));
    }

    let start_date = parse_date.parse_next(input)?;

    if input.starts_with('/') {
        let _ = '/'.parse_next(input)?;
        if input.is_empty() || *input == ".." {
            if *input == ".." {
                let _ = "..".parse_next(input)?;
            }
            Ok(Edtf::IntervalFrom(start_date))
        } else {
            let end_date = parse_date.parse_next(input)?;
            Ok(Edtf::Interval(Interval {
                start: start_date,
                end: end_date,
            }))
        }
    } else {
        Ok(Edtf::Date(start_date))
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

    #[test]
    fn test_parse_date() {
        let mut input = "2023-05-15";
        let res = parse_date(&mut input).unwrap();
        assert_eq!(res.year.value, 2023);
        assert_eq!(res.month_or_season, Some(MonthOrSeason::Month(5)));
        assert_eq!(res.day, Some(Day::Day(15)));
    }

    #[test]
    fn test_unspecified_year() {
        let mut input = "199u";
        let res = parse_date(&mut input).unwrap();
        assert_eq!(res.year.value, 1990);
        assert_eq!(res.year.unspecified, UnspecifiedYear::One);
    }

    #[test]
    fn test_extended_year() {
        let mut input = "Y17000000002";
        let res = parse_date(&mut input).unwrap();
        assert_eq!(res.year.value, 17_000_000_002_i64);
    }

    #[test]
    fn test_unspecified_month_day() {
        let mut input = "2004-uu-uu";
        let res = parse_date(&mut input).unwrap();
        assert_eq!(res.month_or_season, Some(MonthOrSeason::Unspecified));
        assert_eq!(res.day, Some(Day::Unspecified));
    }

    #[test]
    fn test_component_quality() {
        let mut input = "2004?-06-11";
        let res = parse_date(&mut input).unwrap();
        assert!(res.year_quality.uncertain);
        assert!(!res.month_quality.uncertain);
        assert!(!res.day_quality.uncertain);

        let mut input2 = "2004-06-11?";
        let res2 = parse_date(&mut input2).unwrap();
        assert!(!res2.year_quality.uncertain);
        assert!(!res2.month_quality.uncertain);
        assert!(res2.day_quality.uncertain);
    }

    #[test]
    fn test_parse_interval() {
        let mut input = "2023-05/2024-06";
        let res = parse(&mut input).unwrap();
        if let Edtf::Interval(interval) = res {
            assert_eq!(interval.start.year.value, 2023);
            assert_eq!(interval.end.year.value, 2024);
        } else {
            panic!("Expected Interval");
        }
    }

    #[test]
    fn test_parse_interval_from() {
        let mut input = "2023-05/..";
        let res = parse(&mut input).unwrap();
        if let Edtf::IntervalFrom(date) = res {
            assert_eq!(date.year.value, 2023);
        } else {
            panic!("Expected IntervalFrom");
        }
    }

    #[test]
    fn test_parse_interval_to() {
        let mut input = "../2023-05";
        let res = parse(&mut input).unwrap();
        if let Edtf::IntervalTo(date) = res {
            assert_eq!(date.year.value, 2023);
            assert_eq!(date.month_or_season, Some(MonthOrSeason::Month(5)));
        } else {
            panic!("Expected IntervalTo");
        }
    }

    #[test]
    fn test_parse_season() {
        let mut input = "2023-21";
        let res = parse_date(&mut input).unwrap();
        assert_eq!(res.month_or_season, Some(MonthOrSeason::Spring));
        assert_eq!(res.to_string(), "2023-21");
    }

    #[test]
    fn test_edtf_season_accessor() {
        let mut input = "2023-24";
        let res = parse(&mut input).unwrap();
        assert_eq!(res.season_code(), Some(24));
        assert_eq!(res.month(), None);
    }

    #[test]
    fn test_edtf_season_accessor_interval_start() {
        let mut input = "2023-21/2023-23";
        let res = parse(&mut input).unwrap();
        assert_eq!(res.season_code(), Some(21));
    }

    #[test]
    fn test_edtf_season_accessor_none_for_month_date() {
        let mut input = "2023-05";
        let res = parse(&mut input).unwrap();
        assert_eq!(res.season_code(), None);
        assert_eq!(res.month(), Some(5));
    }

    #[test]
    fn test_round_trip() {
        let cases = vec![
            "2023-05-15",
            "199u",
            "2004-uu-uu",
            "2004?-06-11",
            "2004-06-11?",
            "2023-05/2024-06",
            "2023-05/..",
            "../2023-05",
            "Y17000000002",
            "1985-04-12T23:20:30Z",
            "2004-01-01T10:10:10+05:30",
        ];
        for case in cases {
            let mut input = case;
            let res = parse(&mut input).unwrap();
            assert_eq!(res.to_string(), case);
        }
    }

    #[test]
    fn test_parse_datetime_utc() {
        let mut input = "1985-04-12T23:20:30Z";
        let res = parse_date(&mut input).unwrap();
        let t = res.time.unwrap();
        assert_eq!(t.hour, 23);
        assert_eq!(t.minute, 20);
        assert_eq!(t.second, 30);
        assert_eq!(t.timezone, Some(Timezone::Utc));
    }

    #[test]
    fn test_parse_datetime_offset() {
        let mut input = "2004-01-01T10:10:10+05:30";
        let res = parse_date(&mut input).unwrap();
        let t = res.time.unwrap();
        assert_eq!(t.timezone, Some(Timezone::Offset(330)));
    }

    #[test]
    fn test_parse_datetime_no_tz() {
        let mut input = "2004-01-01T10:10:10";
        let res = parse_date(&mut input).unwrap();
        let t = res.time.unwrap();
        assert_eq!(t.timezone, None);
    }

    #[test]
    fn test_parse_leaves_unconsumed_suffix() {
        let mut input = "2023-05 trailing";
        let res = parse(&mut input).unwrap();
        assert_eq!(res.to_string(), "2023-05");
        assert_eq!(input, " trailing");
    }

    #[test]
    fn test_invalid_day_is_rejected() {
        let mut input = "2023-05-32";
        assert!(parse_date(&mut input).is_err());
    }

    #[test]
    fn test_invalid_time_is_rejected() {
        let mut invalid_hour = "2023-05-15T24:00:00";
        assert!(parse_date(&mut invalid_hour).is_err());

        let mut invalid_minute = "2023-05-15T23:60:00";
        assert!(parse_date(&mut invalid_minute).is_err());

        let mut invalid_second = "2023-05-15T23:59:60";
        assert!(parse_date(&mut invalid_second).is_err());
    }
}
