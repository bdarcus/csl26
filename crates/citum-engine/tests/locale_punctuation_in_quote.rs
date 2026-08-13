/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Integration coverage for locale-supplied `punctuation-in-quote` defaults
//! (`Processor::resolve_punctuation_defaults`, `docs/specs/PUNCTUATION_NORMALIZATION.md`).
//! A style that leaves `punctuation-in-quote` unset inherits it from the
//! active locale's `grammar-options`; a locale substituted for one that
//! could not be resolved does not supply it. See `csl26-8e75`.

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in test, benchmark, and example code."
)]

mod common;
use common::make_book;

use citum_engine::Processor;
use citum_schema::locale::Locale;
use citum_schema::template::{DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate};
use citum_schema::{BibliographySpec, Style, StyleInfo, tc_title};
use rstest::rstest;

/// A minimal bibliography template: quoted title, then the issued year with
/// a comma prefix — the exact shape (movable mark immediately after a
/// closing quote) that `punctuation-in-quote` governs.
fn quoted_title_then_year_template() -> Vec<TemplateComponent> {
    vec![
        tc_title!(Primary, quote = true),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                prefix: Some(", ".into()),
                suffix: Some(".".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Style that leaves `punctuation-in-quote` unset at every scope, so the
/// rendered comma placement is entirely locale-driven.
fn style_with_unset_punctuation_in_quote() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Locale Punctuation-In-Quote Probe".to_string()),
            id: Some("locale-punctuation-in-quote-probe".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(quoted_title_then_year_template().into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// A locale flagged as substituted for one that could not be resolved,
/// carrying en-US's own `punctuation-in-quote: true` grammar option — proof
/// that the guard, not the option's value, is what suppresses resolution.
fn fallback_flagged_en_us() -> Locale {
    let mut locale = Locale::en_us();
    locale.resolved_by_fallback = true;
    locale
}

#[rstest]
#[case::authoritative_en_us_moves_comma_inside_the_closing_quote(
    Locale::en_us(),
    "\u{201C}Structure Is Not Destiny,\u{201D} 1962."
)]
#[case::fallback_flagged_locale_leaves_comma_outside(
    fallback_flagged_en_us(),
    "\u{201C}Structure Is Not Destiny\u{201D}, 1962."
)]
fn given_unset_style_option_when_bibliography_renders_then_locale_authority_governs_punctuation_placement(
    #[case] locale: Locale,
    #[case] expected: &str,
) {
    let style = style_with_unset_punctuation_in_quote();
    let bibliography = indexmap::indexmap! { "kuhn1962".to_string() => make_book("kuhn1962", "Kuhn", "Thomas S.", 1962, "Structure Is Not Destiny") };

    let processor = Processor::with_locale(style, bibliography, locale);
    let rendered = processor.render_bibliography();

    assert_eq!(
        rendered.trim(),
        expected,
        "rendered bibliography entry:\n{rendered}"
    );
}
