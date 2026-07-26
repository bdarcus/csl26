/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::Locale;
use super::message::MessageArgs;
use super::types::MessageSyntax;

impl Locale {
    /// Resolve a `pattern.date-*` message with locale-specific year/month/day
    /// components.
    ///
    /// Returns `Some(rendered)` only when the locale carries an MF2 message
    /// at `message_id` and the evaluator produces output. Callers fall back
    /// to the engine's hardcoded English assembly on `None`.
    ///
    /// A component is forwarded to the evaluator only when non-empty; an
    /// authored pattern that references `{$day}` therefore yields `None` if
    /// the input date carries no day, letting the caller pick a shorter form.
    ///
    /// The day argument is taken as `Option<u32>` rather than a pre-formatted
    /// string so the digit-to-string allocation is deferred until after the
    /// message lookup succeeds - the common case for legacy locales (`en-US`,
    /// every v1 file) is the lookup miss, which now incurs zero allocation.
    pub fn resolve_date_pattern(
        &self,
        message_id: &str,
        year: Option<&str>,
        month: Option<&str>,
        day: Option<u32>,
    ) -> Option<String> {
        let message = self.messages.get(message_id)?;
        if self.evaluation.message_syntax == MessageSyntax::Static {
            return None;
        }

        let day_str = day.map(|d| d.to_string());
        let args = MessageArgs {
            year: year.filter(|s| !s.is_empty()),
            month: month.filter(|s| !s.is_empty()),
            day: day_str.as_deref(),
            ..MessageArgs::default()
        };
        self.evaluator.evaluate(message, &args)
    }

    /// Resolve a shared-year `pattern.date-range-*` message with pre-formatted
    /// endpoint fragments and their common year.
    ///
    /// The message is evaluated only for MF2 locales. Callers fall back to
    /// their established date-form assembly when a locale has not authored
    /// the requested interval pattern.
    pub fn resolve_date_range_pattern(
        &self,
        message_id: &str,
        start: &str,
        end: &str,
        year: Option<&str>,
    ) -> Option<String> {
        let message = self.messages.get(message_id)?;
        if self.evaluation.message_syntax == MessageSyntax::Static {
            return None;
        }

        let args = MessageArgs {
            start: (!start.is_empty()).then_some(start),
            end: (!end.is_empty()).then_some(end),
            year: year.filter(|value| !value.is_empty()),
            ..MessageArgs::default()
        };
        self.evaluator.evaluate(message, &args)
    }
}
