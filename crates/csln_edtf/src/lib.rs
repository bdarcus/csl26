//! csln_edtf - A modern EDTF (Extended Date/Time Format) parser
//!
//! This crate implements ISO 8601-2:2019 (EDTF) Level 0 and Level 1.

use winnow::ascii::dec_int;
use winnow::combinator::{alt, opt, preceded};
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

/// A date interval.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Interval {
    pub start: Date,
    pub end: Date,
}

/// Represents the Month or the EDTF Season (Level 1)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MonthOrSeason {
    Month(u32),
    Unspecified, // 'uu' or 'XX'
    Spring,      // 21
    Summer,      // 22
    Autumn,      // 23
    Winter,      // 24
}

/// Metadata about the certainty and precision of a date component
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Quality {
    pub uncertain: bool,   // '?'
    pub approximate: bool, // '~'
}

/// A year in an EDTF date, which may contain unspecified digits.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Year {
    pub value: i64,
    pub unspecified: UnspecifiedYear,
}

/// Unspecified digits in a year (EDTF Level 1).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnspecifiedYear {
    #[default]
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
    Day(u32),
    Unspecified, // 'uu'
}

/// The core EDTF Date structure
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Date {
    pub year: Year,
    pub year_quality: Quality,
    pub month_or_season: Option<MonthOrSeason>,
    pub month_quality: Quality,
    pub day: Option<Day>,
    pub day_quality: Quality,
    pub time: Option<Time>,
}

/// Basic ISO 8601-style time
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

use std::fmt;

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Edtf::Date(d) => write!(f, "{}", d),
            Edtf::Interval(i) => write!(f, "{}/{}", i.start, i.end),
            Edtf::IntervalFrom(d) => write!(f, "{}/..", d),
            Edtf::IntervalTo(d) => write!(f, "../{}", d),
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
            write!(f, "T{}", t)?;
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
            let mut s = format!("{:04}", abs_val);
            match self.unspecified {
                UnspecifiedYear::None => write!(f, "-{}", s),
                UnspecifiedYear::One => {
                    s.replace_range(3..4, "u");
                    write!(f, "-{}", s)
                }
                UnspecifiedYear::Two => {
                    s.replace_range(2..4, "uu");
                    write!(f, "-{}", s)
                }
                UnspecifiedYear::Three => {
                    s.replace_range(1..4, "uuu");
                    write!(f, "-{}", s)
                }
                UnspecifiedYear::Four => {
                    s.replace_range(0..4, "uuuu");
                    write!(f, "-{}", s)
                }
            }
        } else {
            let mut s = format!("{:04}", self.value);
            match self.unspecified {
                UnspecifiedYear::None => write!(f, "{}", s),
                UnspecifiedYear::One => {
                    s.replace_range(3..4, "u");
                    write!(f, "{}", s)
                }
                UnspecifiedYear::Two => {
                    s.replace_range(2..4, "uu");
                    write!(f, "{}", s)
                }
                UnspecifiedYear::Three => {
                    s.replace_range(1..4, "uuu");
                    write!(f, "{}", s)
                }
                UnspecifiedYear::Four => {
                    s.replace_range(0..4, "uuuu");
                    write!(f, "{}", s)
                }
            }
        }
    }
}

impl fmt::Display for MonthOrSeason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonthOrSeason::Month(m) => write!(f, "{:02}", m),
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
            Day::Day(d) => write!(f, "{:02}", d),
            Day::Unspecified => write!(f, "uu"),
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
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
    Ok(Day::Day(val))
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

    Ok(Time {
        hour,
        minute,
        second,
    })
}

/// Parses a single date component.
pub fn parse_date(input: &mut &str) -> Result<Date, ErrMode<ContextError>> {
    let year = parse_year.parse_next(input)?;
    let year_quality = parse_quality.parse_next(input)?;

    let month_or_season = opt(preceded('-', parse_month_or_season)).parse_next(input)?;
    let month_quality = if month_or_season.is_some() {
        parse_quality.parse_next(input)?
    } else {
        Quality::default()
    };

    let day =
        if let Some(MonthOrSeason::Month(_)) | Some(MonthOrSeason::Unspecified) = month_or_season {
            opt(preceded('-', parse_day)).parse_next(input)?
        } else {
            None
        };
    let day_quality = if day.is_some() {
        parse_quality.parse_next(input)?
    } else {
        Quality::default()
    };

    let time = opt(preceded('T', parse_time)).parse_next(input)?;

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

/// Main entry point for parsing an EDTF Level 1 string.
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
        assert_eq!(res.year.value, 17000000002_i64);
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
        ];
        for case in cases {
            let mut input = case;
            let res = parse(&mut input).unwrap();
            assert_eq!(res.to_string(), case);
        }
    }
}
