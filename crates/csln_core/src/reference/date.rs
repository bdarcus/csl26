use crate::locale::MonthList;
use crate::reference::types::RefDate;
use csln_edtf::{Day, Edtf, MonthOrSeason};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An EDTF string.
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct EdtfString(pub String);

impl EdtfString {
    /// Parse the string as an EDTF date etc, or return the string as a literal.
    pub fn parse(&self) -> RefDate {
        let mut input = self.0.as_str();
        match csln_edtf::parse(&mut input) {
            Ok(edtf) => RefDate::Edtf(edtf),
            Err(_) => RefDate::Literal(self.0.clone()),
        }
    }

    /// Extract the year from the date.
    pub fn year(&self) -> String {
        let parsed_date = self.parse();
        match parsed_date {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => date.year.value.to_string(),
                Edtf::Interval(interval) => interval.start.year.value.to_string(),
                Edtf::IntervalFrom(date) => date.year.value.to_string(),
                Edtf::IntervalTo(date) => date.year.value.to_string(),
            },
            RefDate::Literal(_) => String::new(),
        }
    }

    fn month_to_string(month: u32, months: &[String]) -> String {
        if month > 0 {
            let index = month - 1;
            if index < months.len() as u32 {
                months[index as usize].clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Extract the month from the date.
    pub fn month(&self, months: &[String]) -> String {
        let parsed_date = self.parse();
        let month: Option<u32> = match parsed_date {
            RefDate::Edtf(edtf) => {
                let m_opt = match edtf {
                    Edtf::Date(date) => date.month_or_season,
                    Edtf::Interval(interval) => interval.start.month_or_season,
                    Edtf::IntervalFrom(date) => date.month_or_season,
                    Edtf::IntervalTo(date) => date.month_or_season,
                };
                match m_opt {
                    Some(MonthOrSeason::Month(m)) => Some(m),
                    _ => None,
                }
            }
            RefDate::Literal(_) => None,
        };
        match month {
            Some(month) => EdtfString::month_to_string(month, months),
            None => String::new(),
        }
    }

    /// Format as "Month Year".
    pub fn year_month(&self, months: &MonthList) -> String {
        let month = self.month(months);
        let year = self.year();
        if month.is_empty() || year.is_empty() {
            String::new()
        } else {
            format!("{} {}", month, year)
        }
    }

    /// Extract the day from the date.
    pub fn day(&self) -> Option<u32> {
        let parsed_date = self.parse();
        match parsed_date {
            RefDate::Edtf(edtf) => {
                let d_opt = match edtf {
                    Edtf::Date(date) => date.day,
                    Edtf::Interval(interval) => interval.start.day,
                    Edtf::IntervalFrom(date) => date.day,
                    Edtf::IntervalTo(date) => date.day,
                };
                match d_opt {
                    Some(Day::Day(d)) => Some(d),
                    _ => None,
                }
            }
            RefDate::Literal(_) => None,
        }
        .filter(|&d| d > 0)
    }

    /// Format as "Month Day".
    pub fn month_day(&self, months: &MonthList) -> String {
        let month = self.month(months);
        let day = self.day();
        match day {
            Some(d) if !month.is_empty() => format!("{} {}", month, d),
            _ => String::new(),
        }
    }

    /// Check if the date is uncertain (has "?" qualifier).
    pub fn is_uncertain(&self) -> bool {
        self.0.contains('?')
    }

    /// Check if the date is approximate (has "~" qualifier).
    pub fn is_approximate(&self) -> bool {
        self.0.contains('~')
    }

    /// Check if the date is a range (interval).
    pub fn is_range(&self) -> bool {
        matches!(
            self.parse(),
            RefDate::Edtf(Edtf::Interval(_) | Edtf::IntervalFrom(_) | Edtf::IntervalTo(_))
        )
    }

    /// Get the range end date if this is a range, formatted as a string.
    pub fn range_end(&self, months: &MonthList) -> Option<String> {
        match self.parse() {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Interval(interval) => {
                    let end = &interval.end;
                    let year = end.year.value.to_string();
                    let month = match end.month_or_season {
                        Some(MonthOrSeason::Month(m)) => Some(m),
                        _ => None,
                    };
                    let day = match end.day {
                        Some(Day::Day(d)) => Some(d),
                        _ => None,
                    };

                    match (month, day) {
                        (Some(m), Some(d)) if m > 0 && d > 0 => {
                            let month_str = EdtfString::month_to_string(m, months);
                            Some(format!("{} {}, {}", month_str, d, year))
                        }
                        (Some(m), _) if m > 0 => {
                            let month_str = EdtfString::month_to_string(m, months);
                            Some(format!("{} {}", month_str, year))
                        }
                        _ => Some(year),
                    }
                }
                Edtf::IntervalFrom(_date) => None, // Open-ended
                Edtf::IntervalTo(date) => {
                    let year = date.year.value.to_string();
                    Some(year)
                }
                _ => None,
            },
            RefDate::Literal(_) => None,
        }
    }

    /// Check if the range is open-ended (ends with "..").
    pub fn is_open_range(&self) -> bool {
        matches!(self.parse(), RefDate::Edtf(Edtf::IntervalFrom(_)))
    }
}

impl fmt::Display for EdtfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
