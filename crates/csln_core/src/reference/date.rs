use crate::locale::MonthList;
use crate::reference::types::RefDate;
use edtf::level_1::Edtf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An EDTF string.
#[derive(Debug, Deserialize, Serialize, Clone, Default, JsonSchema, PartialEq)]
pub struct EdtfString(pub String);

impl EdtfString {
    /// Parse the string as an EDTF date etc, or return the string as a literal.
    pub fn parse(&self) -> RefDate {
        match Edtf::parse(&self.0) {
            Ok(edtf) => RefDate::Edtf(edtf),
            Err(_) => RefDate::Literal(self.0.clone()),
        }
    }

    fn component_to_u32(&self, component: Option<edtf::level_1::Component>) -> u32 {
        match component {
            Some(component) => component.value().unwrap_or(0),
            None => 0,
        }
    }

    /// Extract the year from the date.
    pub fn year(&self) -> String {
        let parsed_date = self.parse();
        match parsed_date {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => date.year().to_string(),
                Edtf::YYear(year) => format!("{}", year.value()),
                Edtf::DateTime(datetime) => datetime.date().year().to_string(),
                Edtf::Interval(start, _end) => format!("{}", start.year()),
                Edtf::IntervalFrom(date, _terminal) => format!("{}", date.year()),
                Edtf::IntervalTo(_terminal, date) => format!("{}", date.year()),
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
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => Some(self.component_to_u32(date.month())),
                Edtf::YYear(_year) => None,
                Edtf::DateTime(datetime) => Some(datetime.date().month()),
                Edtf::Interval(_start, _end) => None,
                Edtf::IntervalFrom(_date, _terminal) => None,
                Edtf::IntervalTo(_terminal, _date) => None,
            },
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
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Date(date) => Some(self.component_to_u32(date.day())),
                Edtf::YYear(_) => None,
                Edtf::DateTime(datetime) => Some(datetime.date().day()),
                Edtf::Interval(_, _) => None,
                Edtf::IntervalFrom(_, _) => None,
                Edtf::IntervalTo(_, _) => None,
            },
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
            RefDate::Edtf(Edtf::Interval(_, _) | Edtf::IntervalFrom(_, _) | Edtf::IntervalTo(_, _))
        )
    }

    /// Get the range end date if this is a range, formatted as a string.
    pub fn range_end(&self, months: &MonthList) -> Option<String> {
        match self.parse() {
            RefDate::Edtf(edtf) => match edtf {
                Edtf::Interval(_start, end) => {
                    let year = end.year().to_string();
                    let month = end.month().and_then(|m| m.value());
                    let day = end.day().and_then(|d| d.value());

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
                Edtf::IntervalFrom(_start, _terminal) => None, // Open-ended
                Edtf::IntervalTo(_terminal, end) => {
                    let year = end.year().to_string();
                    Some(year)
                }
                _ => None,
            },
            RefDate::Literal(_) => None,
        }
    }

    /// Check if the range is open-ended (ends with "..").
    pub fn is_open_range(&self) -> bool {
        matches!(self.parse(), RefDate::Edtf(Edtf::IntervalFrom(_, _)))
    }
}

impl fmt::Display for EdtfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
