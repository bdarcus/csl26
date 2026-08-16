/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Integration tests for `options.multilingual.term-locale` — the per-item
//! term localization opt-in (biblatex `autolang` analogue). See
//! `docs/specs/PER_ITEM_TERM_LOCALE.md`.

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
use common::announce_behavior;

use citum_engine::render::plain::PlainText;
use citum_engine::{Citation, CitationItem, Processor};
use citum_schema::locale::GeneralTerm;
use citum_schema::{
    BibliographySpec, CitationSpec, LocalizedTemplateSpec, Style, StyleInfo,
    options::{BibliographyOptions, MultilingualConfig, TermLocale},
    reference::{DateValue, InputReference, Monograph, MonographType, Title},
    template::{DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate, TemplateTerm},
};
use indexmap::IndexMap;

/// Build a minimal book reference with an optional top-level `language` tag
/// and an EDTF `issued` date, using Citum's native reference data structures
/// directly (no CSL-JSON round-trip).
fn term_locale_test_book(
    id: &str,
    title: &str,
    language: Option<&str>,
    date: &str,
) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        issued: DateValue::new(date.to_string()),
        language: language.map(Into::into),
        ..Default::default()
    }))
}

/// A template that renders a quoted title (typography probe), the `and`
/// general term (word-surface probe: "and"/"und"/"et"), and the full date
/// form (month-name probe).
fn term_locale_probe_template() -> Vec<TemplateComponent> {
    vec![
        citum_schema::tc_title!(Primary, quote = true),
        TemplateComponent::Term(TemplateTerm {
            term: GeneralTerm::And,
            form: None,
            gender: None,
            rendering: Rendering {
                prefix: Some(" ".into()),
                ..Default::default()
            },
            custom: None,
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Full,
            suppress_note: None,
            suppress_disamb_suffix: None,
            rendering: Rendering {
                prefix: Some(" ".into()),
                ..Default::default()
            },
            links: None,
            custom: None,
        }),
    ]
}

/// Style with `term-locale: item` in bibliography options and no
/// locale-scoped layout branches.
fn style_with_bibliography_term_locale_item() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Term Locale Item Test".to_string()),
            id: Some("term-locale-item-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                multilingual: Some(MultilingualConfig {
                    term_locale: TermLocale::Item,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            template: Some(term_locale_probe_template().into()),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(term_locale_probe_template().into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn bibliography_term_locale_item_switches_words_but_keeps_style_typography() {
    announce_behavior(
        "term-locale: item switches an item's role/term/date words to its own \
         effective language while typography (quote marks) stays with the style locale.",
    );
    let style = style_with_bibliography_term_locale_item();
    let bibliography = IndexMap::from([
        (
            "german-book".to_string(),
            term_locale_test_book("german-book", "Berliner Mauer", Some("de"), "1990-01-15"),
        ),
        (
            "english-book".to_string(),
            term_locale_test_book("english-book", "American History", Some("en"), "1990-01-15"),
        ),
    ]);
    let processor = Processor::new(style, bibliography);

    let german_entry = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(vec![
            "german-book".to_string(),
        ]);
    let english_entry = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(vec![
            "english-book".to_string(),
        ]);

    // German words ("und", "Januar") but en-US style typography ("smart"
    // curly quotes, not German „low-high“ quotes).
    assert_eq!(german_entry, "“Berliner Mauer” und Januar 15, 1990");
    // English item under the same item-mode config: words and typography
    // both already match the style locale.
    assert_eq!(english_entry, "“American History” and January 15, 1990");
}

#[test]
fn term_locale_item_scoped_to_bibliography_does_not_affect_citation_rendering() {
    announce_behavior(
        "term-locale: item set only in bibliography options does not affect \
         citation rendering — the two scopes resolve independently.",
    );
    let style = style_with_bibliography_term_locale_item();
    let bibliography = IndexMap::from([(
        "german-book".to_string(),
        term_locale_test_book("german-book", "Berliner Mauer", Some("de"), "1990-01-15"),
    )]);
    let processor = Processor::new(style, bibliography);

    let citation = processor
        .process_citation(&Citation {
            items: vec![CitationItem {
                id: "german-book".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .expect("citation should render");

    // Citation scope has no term-locale override, so it stays style-locale
    // (English) even though bibliography scope is item mode. The leading
    // "Berliner Mauer, " is the engine's author-substitute-title behavior
    // for a reference with no author, not part of what this test covers.
    // Default processor locale is en-US, whose `punctuation-in-quote: true`
    // grammar option now supplies the style default (csl26-8e75) and moves
    // the trailing comma inside the closing quote, matching citeproc-js.
    assert_eq!(
        citation,
        "Berliner Mauer, “Berliner Mauer,”  and,  January 15, 1990"
    );
}

#[test]
fn untagged_item_under_term_locale_item_renders_style_terms_with_no_warning() {
    announce_behavior(
        "An untagged item under term-locale: item renders style-locale terms and \
         emits no fallback warning — the positive-evidence rule.",
    );
    let style = style_with_bibliography_term_locale_item();
    let bibliography = IndexMap::from([(
        "untagged-book".to_string(),
        term_locale_test_book("untagged-book", "Untitled Work", None, "1990-01-15"),
    )]);
    let processor = Processor::new(style, bibliography);

    let entry =
        processor.render_selected_bibliography_with_format_standalone::<PlainText, _>(vec![
            "untagged-book".to_string(),
        ]);
    assert_eq!(entry, "“Untitled Work” and January 15, 1990");

    let warnings = citum_engine::api::term_locale_fallback_warnings(&processor);
    assert!(
        warnings.is_empty(),
        "Untagged items must never produce a term-locale fallback warning: {warnings:?}"
    );
}

#[test]
fn tagged_item_with_no_loaded_locale_falls_back_to_style_locale_and_warns() {
    announce_behavior(
        "A tagged item whose language has no loaded locale falls back to style-locale \
         terms silently at render time but is surfaced via a fallback warning.",
    );
    let style = style_with_bibliography_term_locale_item();
    // "it" (Italian) is tagged but not among the embedded locale IDs
    // (en-US, ar-AR, de-DE, es-ES, eu-ES, fr-FR, tr-TR, zh-CN, ja-JP, ko-KR,
    // ru-RU), so neither an exact nor a primary-subtag match exists.
    let bibliography = IndexMap::from([(
        "italian-book".to_string(),
        term_locale_test_book("italian-book", "Storia Italiana", Some("it"), "1990-01-15"),
    )]);
    let processor = Processor::new(style, bibliography);

    let entry =
        processor.render_selected_bibliography_with_format_standalone::<PlainText, _>(vec![
            "italian-book".to_string(),
        ]);
    assert_eq!(entry, "“Storia Italiana” and January 15, 1990");

    let warnings = citum_engine::api::term_locale_fallback_warnings(&processor);
    assert_eq!(
        warnings.len(),
        1,
        "Exactly one fallback warning should be emitted for the unavailable 'it' locale: {warnings:?}"
    );
    assert_eq!(warnings[0].code, "term_locale_unavailable");
    assert_eq!(warnings[0].ref_id.as_deref(), Some("italian-book"));
}

#[test]
fn matched_locales_branch_wins_over_term_locale_item_including_typography() {
    announce_behavior(
        "A matched bibliography.locales[] branch is authoritative: it renders with its \
         own (full) locale, including typography, regardless of term-locale — csl26-838l §5.",
    );
    let mut style = style_with_bibliography_term_locale_item();
    let Some(bib) = style.bibliography.as_mut() else {
        panic!("style should have a bibliography section");
    };
    bib.locales = Some(vec![LocalizedTemplateSpec {
        locale: Some(vec!["de".to_string()]),
        template: term_locale_probe_template(),
        ..Default::default()
    }]);

    let bibliography = IndexMap::from([(
        "german-book".to_string(),
        term_locale_test_book("german-book", "Berliner Mauer", Some("de"), "1990-01-15"),
    )]);
    let processor = Processor::new(style, bibliography);

    let entry =
        processor.render_selected_bibliography_with_format_standalone::<PlainText, _>(vec![
            "german-book".to_string(),
        ]);

    // A matched branch swaps the *entire* rendering locale, so German
    // typography (the „low-high“ quote pair) applies here — unlike the
    // term-only hybrid, which keeps style typography (see the first test
    // in this file, which renders the same content as
    // "“Berliner Mauer” und Januar 15, 1990" — smart quotes, not „ “).
    assert_eq!(entry, "„Berliner Mauer“ und Januar 15, 1990");
}

#[test]
fn bibliography_order_is_unchanged_between_term_locale_style_and_item() {
    announce_behavior(
        "term-locale: item is rendering-only: it must never change bibliography ordering \
         — csl26-838l §6. sort_partitioning.rs, sorting.rs, matching.rs, and \
         disambiguation.rs never consume RenderOptions/the hybridized locale, so this is \
         also guaranteed structurally, not just by this fixture.",
    );
    let bibliography = IndexMap::from([
        (
            "german-book".to_string(),
            term_locale_test_book("german-book", "Aachen Studies", Some("de"), "1990-01-15"),
        ),
        (
            "english-book".to_string(),
            term_locale_test_book(
                "english-book",
                "Zurich Chronicles",
                Some("en"),
                "1990-01-15",
            ),
        ),
    ]);

    let mut style_locale_style = style_with_bibliography_term_locale_item();
    if let Some(bib) = style_locale_style.bibliography.as_mut() {
        bib.options = Some(BibliographyOptions {
            multilingual: Some(MultilingualConfig {
                term_locale: TermLocale::Style,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    let item_locale_style = style_with_bibliography_term_locale_item();

    let style_rendered =
        Processor::new(style_locale_style, bibliography.clone()).render_bibliography();
    let item_rendered = Processor::new(item_locale_style, bibliography).render_bibliography();

    let style_order = (style_rendered.find("Aachen"), style_rendered.find("Zurich"));
    let item_order = (item_rendered.find("Aachen"), item_rendered.find("Zurich"));
    assert!(
        style_order.0.is_some() && style_order.1.is_some(),
        "Both titles should appear in the style-locale rendering: {style_rendered}"
    );
    assert_eq!(
        style_order.0 < style_order.1,
        item_order.0 < item_order.1,
        "Relative bibliography order must be identical between term-locale: style and \
         term-locale: item.\nstyle: {style_rendered}\nitem:  {item_rendered}"
    );
}

#[test]
fn omitted_term_locale_is_byte_identical_to_explicit_style() {
    announce_behavior(
        "term-locale absent defaults to style — today's behavior, byte for byte — \
         csl26-838l acceptance criterion 1.",
    );
    let mut explicit_style_style = style_with_bibliography_term_locale_item();
    if let Some(bib) = explicit_style_style.bibliography.as_mut() {
        bib.options = Some(BibliographyOptions {
            multilingual: Some(MultilingualConfig {
                term_locale: TermLocale::Style,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    let mut unset_style = style_with_bibliography_term_locale_item();
    if let Some(bib) = unset_style.bibliography.as_mut() {
        bib.options = None;
    }

    let bibliography = IndexMap::from([(
        "german-book".to_string(),
        term_locale_test_book("german-book", "Berliner Mauer", Some("de"), "1990-01-15"),
    )]);

    let explicit_rendered =
        Processor::new(explicit_style_style, bibliography.clone()).render_bibliography();
    let unset_rendered = Processor::new(unset_style, bibliography).render_bibliography();

    assert_eq!(
        explicit_rendered, unset_rendered,
        "term-locale: style must be byte-identical to omitting options.multilingual entirely."
    );
}
