/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

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
use common::*;

use citum_engine::Processor;
use citum_engine::processor::document::{
    CitationParser, DocumentFormat, djot::DjotParser, markdown::MarkdownParser,
};
use citum_io::load_bibliography;
use citum_schema::{
    BibliographySpec, CitationSpec, Locale, Style, StyleInfo,
    options::{
        BibliographyOptions, BibliographyPartitionKind, BibliographyPartitionMode,
        BibliographySortPartitioning, Config, Disambiguation, LocatorPreset,
        OrgAbbreviationMemoryConfig, Processing, ProcessingCustom,
    },
    reference::{
        Contributor, DateValue, InputReference as Reference, Monograph, MonographType, SimpleName,
        Title,
    },
};

fn assert_output_includes(output: &str, expected: &str, context: &str) {
    assert_ne!(
        output.find(expected),
        None,
        "{context}\nexpected fragment: {expected:?}\noutput:\n{output}"
    );
}

fn assert_output_excludes(output: &str, unexpected: &str, context: &str) {
    assert_eq!(
        output.find(unexpected),
        None,
        "{context}\nunexpected fragment: {unexpected:?}\noutput:\n{output}"
    );
}

fn assert_output_has_line(output: &str, expected: &str, context: &str) {
    assert!(
        output.lines().any(|line| line == expected),
        "{context}\nexpected line: {expected:?}\noutput:\n{output}"
    );
}

fn assert_output_lacks_line(output: &str, unexpected: &str, context: &str) {
    assert!(
        output.lines().all(|line| line != unexpected),
        "{context}\nunexpected line: {unexpected:?}\noutput:\n{output}"
    );
}

fn assert_output_includes_all(output: &str, expected: &[&str], context: &str) {
    for fragment in expected {
        assert_output_includes(output, fragment, context);
    }
}

fn assert_output_excludes_all(output: &str, unexpected: &[&str], context: &str) {
    for fragment in unexpected {
        assert_output_excludes(output, fragment, context);
    }
}

fn assert_output_in_order(output: &str, first: &str, second: &str, context: &str) {
    let first_position = output.find(first).unwrap_or_else(|| {
        panic!("{context}\nmissing first fragment: {first:?}\noutput:\n{output}")
    });
    let second_position = output.find(second).unwrap_or_else(|| {
        panic!("{context}\nmissing second fragment: {second:?}\noutput:\n{output}")
    });

    assert!(
        first_position < second_position,
        "{context}\nexpected {first:?} before {second:?}\noutput:\n{output}"
    );
}

// --- Document Rendering Scenarios ---

fn given_simple_author_date_document_when_rendered_as_html_then_a_bibliography_heading_is_appended()
{
    // Create a simple style
    let style = Style {
        info: StyleInfo {
            title: Some("Test Style".to_string()),
            id: Some("test".into()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                ..Default::default()
            }),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_date!(Issued, Year),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create a bibliography with one reference
    let mut bibliography = indexmap::IndexMap::new();
    let kuhn = make_book(
        "kuhn1962",
        "Kuhn",
        "Thomas S.",
        1962,
        "The Structure of Scientific Revolutions",
    );
    bibliography.insert("kuhn1962".to_string(), kuhn);

    // Create processor
    let processor = Processor::new(style, bibliography);

    // Create a simple document with a citation
    let document = "This is a test document with a citation [@kuhn1962].\n\nMore text here.";

    // Process document as HTML
    let parser = DjotParser;
    let html_output = processor
        .process_document::<_, citum_engine::render::html::Html>(
            document,
            &parser,
            DocumentFormat::Html,
        )
        .expect("document should render");

    assert_output_includes(
        &html_output,
        "<h1>Bibliography</h1>",
        "HTML output should append a bibliography heading",
    );
    assert_output_includes(
        &html_output,
        "Kuhn",
        "HTML output should replace the citation with rendered reference text",
    );
    assert_output_includes(
        &html_output,
        "test document with a citation",
        "HTML output should preserve original document text",
    );
}

fn given_simple_author_date_document_when_rendered_as_djot_then_html_tags_are_not_emitted() {
    // Create a simple style
    let style = Style {
        info: StyleInfo {
            title: Some("Test Style".to_string()),
            id: Some("test".into()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                ..Default::default()
            }),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_date!(Issued, Year),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create a bibliography
    let mut bibliography = indexmap::IndexMap::new();
    let ref1 = make_book("ref1", "Author", "Name", 2020, "Title");
    bibliography.insert("ref1".to_string(), ref1);

    let processor = Processor::new(style, bibliography);
    let document = "Document with citation [@ref1].";

    // Process as Djot format
    let parser = DjotParser;
    let djot_output = processor
        .process_document::<_, citum_engine::render::djot::Djot>(
            document,
            &parser,
            DocumentFormat::Djot,
        )
        .expect("document should render");

    assert_output_has_line(
        &djot_output,
        "# Bibliography",
        "Djot output should contain a markdown bibliography heading",
    );
    assert_output_lacks_line(
        &djot_output,
        "<h1>Bibliography</h1>",
        "Djot output should not contain HTML heading tags",
    );
}

fn given_example_mla_document_when_rendered_as_html_then_citation_markup_is_not_escaped() {
    let processor = example_document_processor("styles/embedded/modern-language-association.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document.djot");

    let html_output = processor
        .process_document::<_, citum_engine::render::html::Html>(
            &document,
            &parser,
            DocumentFormat::Html,
        )
        .expect("document should render");

    assert_output_includes(
        &html_output,
        r#"<span class="citum-citation" data-ref="smith2010">"#,
        "citation markup should be real HTML",
    );
    assert_output_includes(
        &html_output,
        r#"<div class="citum-bibliography">"#,
        "bibliography markup should be real HTML",
    );
    assert_output_excludes(
        &html_output,
        "&lt;span class=",
        "citation markup should not be escaped",
    );
    assert_output_excludes(
        &html_output,
        "&lt;div class=",
        "bibliography markup should not be escaped",
    );
}

fn given_example_mla_document_when_rendered_as_plain_text_then_integral_name_memory_is_visible() {
    let processor = example_document_processor("styles/embedded/modern-language-association.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document.djot");

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            &document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_eq!(
        output,
        "# Chapter One\n\nThis is a test document using the proposed Djot citation syntax.\nThis example overrides the MLA default `document` scope to `chapter`\nso the narrative-name reset is visible in one short sample.\n\n## Parenthetical Citations\n\nMulti-cite with locator: (Kuhn; Watson and Crick, chap. 2).\n\nStructured locator: (Kuhn, sec. 5).\n\nSimple parenthetical: (Watson and Crick).\n\n## Integral Citations\n\nIntroductory note[^narrative-note].\n\nFirst narrative mention: Anthony D. Smith (10) surveys the broader literature.\n\nLater in the same chapter: Smith (12) narrows the argument.\n\nIntegral with locator: Thomas S. Kuhn (10) argues...\n\n## Visibility Modifiers\n\nSuppress author with locator: (10).\n\n[^narrative-note]: Before the prose introduces him, Anthony D. Smith (3) already appears in a note.\n\n# Chapter Two\n\nThe chapter boundary resets the narrative name memory, so Anthony D. Smith (14)\nappears in full again here.\n\n## In-Document Bibliography Grouping\n\nCitum supports `::: bibliography :::` fenced divs to place and filter\nbibliography sections inline. Entries are assigned to blocks in order;\nlater blocks exclude entries already placed by earlier ones.\nThe default appended bibliography is suppressed when any block is present.\n\n### Filtered by type\n\n## Journal Articles\n\nDoe, Jane. “Silent Paper.” _Journal of Silence_, 2020.\n\nKuhn, Thomas S. “The Structure of Scientific Revolutions.” _International Encyclopedia of Unified Science_, vol. 2, no. 2. University of Chicago Press, 1962, https://doi.org/10.1234/example\n\nWatson, James D., and Francis H. C. Crick. “Molecular Structure of Nucleic Acids: A Structure for Deoxyribose Nucleic Acid.” _Nature_, vol. 171, no. 4356, 25 Apr. 1953, pp. 737–38.\n\n## Books\n\nBird, Arthur. _Ornithology_. Nature Press, 1987.\n\nBrown, Dorothy. _Methods of Surveying and Measuring Vegetation_. Commonwealth Agricultural Bureaux, 1954.\n\nSmith, Anthony D. _Nationalism: Theory, Ideology, History_. Polity, 2010.\n\n### Remaining references\n\n\n"
    );
}

fn given_two_authors_with_same_surname_when_both_cited_integrally_then_each_gets_first_form() {
    let mut style = load_style("styles/embedded/modern-language-association.yaml");
    style
        .options
        .get_or_insert_with(Config::default)
        .integral_name_memory = Some(citum_schema::options::IntegralNameMemoryConfig {
        contexts: Some(citum_schema::options::IntegralNameContexts::BodyOnly),
        subsequent_form: Some(citum_schema::options::SubsequentNameForm::Short),
        ..Default::default()
    });

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "john-smith".to_string(),
        Reference::Monograph(Box::new(Monograph {
            id: Some("john-smith".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Book One".to_string())),
            author: Some(Contributor::StructuredName(
                citum_schema::reference::StructuredName {
                    family: citum_schema::reference::MultilingualString::Simple(
                        "Smith".to_string(),
                    ),
                    given: citum_schema::reference::MultilingualString::Simple("John".to_string()),
                    suffix: None,
                    dropping_particle: None,
                    non_dropping_particle: None,
                },
            )),
            issued: DateValue::new("2010".to_string()),
            ..Default::default()
        })),
    );
    bib.insert(
        "jane-smith".to_string(),
        Reference::Monograph(Box::new(Monograph {
            id: Some("jane-smith".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Book Two".to_string())),
            author: Some(Contributor::StructuredName(
                citum_schema::reference::StructuredName {
                    family: citum_schema::reference::MultilingualString::Simple(
                        "Smith".to_string(),
                    ),
                    given: citum_schema::reference::MultilingualString::Simple("Jane".to_string()),
                    suffix: None,
                    dropping_particle: None,
                    non_dropping_particle: None,
                },
            )),
            issued: DateValue::new("2015".to_string()),
            ..Default::default()
        })),
    );

    let processor = citum_engine::Processor::new(style, bib);
    let parser = DjotParser;
    let doc = "[+@john-smith] wrote the first book. [+@jane-smith] wrote the second.";

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            doc,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // Both authors share surname "Smith" but are different people, so each
    // person's first integral mention should use the full given+family form.
    // A family-name-only tracking key would conflate Jane with John's prior
    // mention and render Jane as just "Smith".
    assert_output_includes(
        &output,
        "John Smith",
        "John Smith's first integral mention should show full given+family name",
    );
    assert_output_includes(
        &output,
        "Jane Smith",
        "Jane Smith's first integral mention should show full given+family name",
    );
}

fn given_org_with_short_name_when_org_abbreviation_memory_configured_and_cited_integrally_twice_then_first_shows_full_then_subsequent_shows_short()
 {
    let mut style = load_style("styles/embedded/modern-language-association.yaml");
    style
        .options
        .get_or_insert_with(Config::default)
        .org_abbreviation_memory = Some(OrgAbbreviationMemoryConfig {
        contexts: Some(citum_schema::options::IntegralNameContexts::BodyOnly),
        ..Default::default()
    });

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "who2020".to_string(),
        Reference::Monograph(Box::new(Monograph {
            id: Some("who2020".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("World Health Report".to_string())),
            author: Some(Contributor::SimpleName(SimpleName {
                name: citum_schema::reference::MultilingualString::Simple(
                    "World Health Organization".to_string(),
                ),
                short_name: Some("WHO".to_string()),
                location: None,
            })),
            issued: DateValue::new("2020".to_string()),
            ..Default::default()
        })),
    );

    let processor = citum_engine::Processor::new(style, bib);
    let parser = DjotParser;
    // Two integral citations to the same org.
    let doc = "[+@who2020] released a report. Later, [+@who2020] followed up.";

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            doc,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // First integral mention: full name + abbreviation in parens.
    assert_output_includes(
        &output,
        "World Health Organization (WHO)",
        "first mention should show full name then abbreviation",
    );
    // Subsequent mention: abbreviation only.
    assert_output_in_order(
        &output,
        "World Health Organization (WHO)",
        "WHO followed",
        "second mention should use short form after the first full mention",
    );
}

fn given_example_apa_document_when_rendered_as_plain_text_then_integral_citations_include_locators()
{
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document.djot");

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            &document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // APA abbreviates given names: "A. D. Smith" (Long form, first integral mention)
    assert_output_has_line(
        &output,
        "First narrative mention: A. D. Smith (2010, p. 10) surveys the broader literature.",
        "APA output should keep the first integral locator",
    );
    assert_output_has_line(
        &output,
        "Later in the same chapter: Smith (2010, p. 12) narrows the argument.",
        "APA output should keep the subsequent integral locator",
    );
    assert_output_has_line(
        &output,
        "Integral with locator: T. S. Kuhn (1962, p. 10) argues...",
        "APA output should keep Kuhn's integral locator",
    );
    assert_output_has_line(
        &output,
        "[^narrative-note]: Before the prose introduces him, A. D. Smith (2010, p. 3) already appears in a note.",
        "APA output should keep the manual note locator",
    );
    assert_output_has_line(
        &output,
        "Suppress author with locator: (1962, p. 10).",
        "APA output should keep the suppress-author locator",
    );
}

fn given_example_chicago_note_document_when_rendered_as_plain_text_then_integral_mentions_keep_their_note_anchor()
 {
    let processor =
        example_document_processor("styles/embedded/chicago-shortened-notes-bibliography.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document.djot");

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            &document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // Body anchors and note bodies are distinct rendered surfaces. Smith is
    // introduced first in the manual note, so later body anchors use the
    // surname anchor; Kuhn has no prior integral-name mention, so its body
    // anchor uses the full note-style long form.
    assert_output_has_line(
        &output,
        "First narrative mention: Smith[^citum-auto-5] surveys the broader literature.",
        "Chicago note output should keep Smith's first narrative anchor",
    );
    assert_output_has_line(
        &output,
        "Later in the same chapter: Smith[^citum-auto-6] narrows the argument.",
        "Chicago note output should keep Smith's subsequent narrative anchor",
    );
    assert_output_has_line(
        &output,
        "Integral with locator: Thomas S. Kuhn[^citum-auto-7] argues...",
        "Chicago note output should keep Kuhn's narrative anchor",
    );

    // Manual note content is checked separately from body anchors: it should
    // preserve the authored anchor and reduced note content with its locator.
    assert_output_includes(
        &output,
        "[^narrative-note]: Before the prose introduces him, Smith, Anthony D.",
        "manual note should preserve the authored anchor",
    );
    assert_output_includes(
        &output,
        "_Nationalism: Theory, Ideology, History_, 3",
        "manual note should preserve the reduced note content and locator",
    );
    assert_eq!(
        output.matches("[^narrative-note]:").count(),
        1,
        "manual note should not be duplicated: {output}"
    );
    assert_output_excludes(&output, "[+@smith2010]", "raw citation should not leak");
}

// --- Note Flow Scenarios ---

fn given_chicago_note_flow_document_when_authored_ibid_appears_in_notes_then_anchor_and_reduced_form_stay_separate()
 {
    let processor = example_document_processor("styles/embedded/chicago-notes-18th.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document-citation-flow.djot");

    let output = processor
        .process_document::<_, citum_engine::render::djot::Djot>(
            &document,
            &parser,
            DocumentFormat::Djot,
        )
        .expect("document should render");

    let lowercase_output = output.to_lowercase();
    assert_output_includes(
        &lowercase_output,
        "ibid",
        "note style should render ibid in this scenario",
    );
    assert_output_excludes(
        &output,
        "KuhnIbid",
        "ibid should not concatenate with Kuhn author token",
    );
    assert_output_excludes(
        &output,
        "SmithIbid",
        "ibid should not concatenate with Smith author token",
    );
    assert_output_includes(
        &output,
        "Brown (",
        "authored note ibid should preserve the Brown narrative anchor",
    );
    assert_output_excludes(
        &lowercase_output,
        "brown ibid",
        "authored note ibid should not concatenate the anchor and reduced form",
    );
    assert_output_excludes(
        &lowercase_output,
        "ibid also argues that...",
        "authored note ibid should not replace the narrative anchor",
    );
}

fn given_chicago_note_locator_repeat_when_integral_ibid_is_rendered_then_anchor_and_locator_are_preserved()
 {
    let processor = example_document_processor("styles/embedded/chicago-notes-18th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "Text.[^n1]\n\n",
        "[^n1]:\n\n",
        "  - [+@brown1954, p. 10] argues that...\n",
        "  - [+@brown1954, p. 12] also argues that...\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Brown (",
        "integral ibid-with-locator should preserve the authored anchor",
    );
    assert_output_includes(
        &output.to_lowercase(),
        "ibid",
        "integral ibid-with-locator should remain reduced",
    );
    assert_output_includes(
        &output,
        "Brown (ibid., 12)",
        "integral ibid-with-locator should preserve the reduced locator fragment",
    );
}

fn given_page_labels_are_configured_when_integral_ibid_is_rendered_then_the_labeled_page_locator_is_preserved()
 {
    let mut style = load_style("styles/embedded/chicago-notes-18th.yaml").into_resolved();
    style.options.get_or_insert_with(Default::default).locators =
        Some(LocatorPreset::AuthorDate.config());
    let processor = Processor::new(style, load_example_bibliography());
    let parser = DjotParser;
    let document = concat!(
        "Text.[^n1]\n\n",
        "[^n1]:\n\n",
        "  - [+@brown1954, p. 10] argues that...\n",
        "  - [+@brown1954, p. 12] also argues that...\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Brown (",
        "integral ibid-with-locator should preserve the authored anchor",
    );
    assert_output_includes(
        &output.to_lowercase(),
        "ibid",
        "integral ibid-with-locator should remain reduced",
    );
    assert_output_includes(
        &output,
        "p. 12",
        "configured page labels should be preserved in reduced manual notes",
    );
}

fn given_chapter_locator_repeat_when_integral_ibid_is_rendered_then_the_labeled_chapter_locator_is_preserved()
 {
    let processor = example_document_processor("styles/embedded/chicago-notes-18th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "Text.[^n1]\n\n",
        "[^n1]:\n\n",
        "  - [+@brown1954, chap. 2] argues that...\n",
        "  - [+@brown1954, chap. 3] also argues that...\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Brown (",
        "integral ibid-with-locator should preserve the authored anchor",
    );
    assert_output_includes(
        &output.to_lowercase(),
        "ibid",
        "integral ibid-with-locator should remain reduced",
    );
    assert_output_includes(
        &output,
        "chap. 3",
        "chapter locators should retain their localized label in reduced manual notes",
    );
}

fn given_locale_specific_ibid_term_when_the_style_has_no_ibid_override_then_the_locale_term_is_used_without_base_suffix_punctuation()
 {
    let mut style = load_style("styles/embedded/chicago-notes-18th.yaml").into_resolved();
    if let Some(citation) = style.citation.as_mut() {
        citation.suffix = Some(".".into());
        citation.ibid = None;
    }

    let mut locale = Locale::en_us();
    locale.terms.ibid = Some("ibid".to_string());

    let processor = Processor::with_locale(style, load_example_bibliography(), locale);
    let parser = DjotParser;
    let document = load_example_document("examples/document-citation-flow.djot");

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            &document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_has_line(
        &output,
        "   - Brown (ibid) also argues that...",
        "ibid term should come from locale data and preserve locale punctuation",
    );
    assert_output_excludes(
        &output,
        "Brown (.) also argues that...",
        "ibid fallback must not use base citation suffix",
    );
}

fn given_explicit_style_ibid_suffix_when_locale_also_defines_ibid_then_the_style_suffix_wins() {
    let mut style = load_style("styles/embedded/chicago-notes-18th.yaml").into_resolved();
    if let Some(citation) = style.citation.as_mut()
        && let Some(ibid) = citation.ibid.as_mut()
    {
        ibid.suffix = Some("IBIDX".into());
    }

    let mut locale = Locale::en_us();
    locale.terms.ibid = Some("ibid".to_string());

    let processor = Processor::with_locale(style, load_example_bibliography(), locale);
    let parser = DjotParser;
    let document = load_example_document("examples/document-citation-flow.djot");

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            &document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_has_line(
        &output,
        "   - Brown (IBIDX) also argues that...",
        "explicit style ibid suffix should override locale ibid term",
    );
}

fn given_missing_note_anchor_when_integral_ibid_is_rendered_then_the_reduced_citation_still_appears_without_concatenation()
 {
    let processor = example_document_processor("styles/embedded/chicago-notes-18th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "Text.[^n1]\n\n",
        "[^n1]:\n\n",
        "  - [+@missingref, p. 10] argues that...\n",
        "  - [+@missingref, p. 12] also argues that...\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output.to_lowercase(),
        "ibid",
        "when anchor cannot render, integral ibid should still render reduced citation text",
    );
    assert_output_excludes(
        &output,
        "missingrefIbid",
        "anchor failure path must not concatenate fallback and ibid text",
    );
}

fn given_chicago_note_flow_document_when_no_bibliography_entries_are_needed_then_no_heading_is_emitted()
 {
    let processor = example_document_processor("styles/embedded/chicago-notes-18th.yaml");
    let parser = DjotParser;
    let document = load_example_document("examples/document-citation-flow.djot");

    let output = processor
        .process_document::<_, citum_engine::render::djot::Djot>(
            &document,
            &parser,
            DocumentFormat::Djot,
        )
        .expect("document should render");

    assert_output_lacks_line(
        &output,
        "# Bibliography",
        "empty bibliography should not emit Djot heading",
    );

    let html_output = processor
        .process_document::<_, citum_engine::render::html::Html>(
            &document,
            &parser,
            DocumentFormat::Html,
        )
        .expect("document should render");
    assert_output_excludes(
        &html_output,
        "<h1>Bibliography</h1>",
        "empty bibliography should not emit HTML heading",
    );
}

fn given_a_style_without_a_bibliography_when_rendering_a_document_then_citations_remain_and_bibliography_output_is_empty()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Citation-only test style".to_string()),
            id: Some("citation-only-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_number!(CitationNumber)]),
            wrap: Some(citum_schema::template::WrapPunctuation::Brackets.into()),
            ..Default::default()
        }),
        bibliography: None,
        ..Default::default()
    };
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "kuhn1962".to_string(),
        make_book(
            "kuhn1962",
            "Kuhn",
            "Thomas S.",
            1962,
            "The Structure of Scientific Revolutions",
        ),
    );
    let processor = Processor::new(style, bibliography);
    let parser = DjotParser;

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "A claim [@kuhn1962].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("citation-only document should render");

    assert_eq!(output.trim(), "A claim [1].");
    assert_eq!(processor.render_bibliography(), "");
    assert!(processor.process_references().bibliography.is_empty());
}

fn given_non_note_styles_when_rendering_the_note_flow_example_then_ibid_is_never_emitted() {
    let parser = DjotParser;
    let document = load_example_document("examples/document-citation-flow.djot");

    for style_path in [
        "styles/embedded/apa-7th.yaml",
        "styles/embedded/ieee.yaml",
        "styles/alpha.yaml",
    ] {
        let style = load_style(style_path);
        let processor = Processor::new(style, load_example_bibliography());

        let output = processor
            .process_document::<_, citum_engine::render::djot::Djot>(
                &document,
                &parser,
                DocumentFormat::Djot,
            )
            .expect("document should render");
        assert_output_excludes(
            &output,
            "Ibid",
            &format!("non-note style unexpectedly rendered ibid for {style_path}"),
        );
    }
}

fn given_pandoc_markdown_author_date_syntax_when_rendered_then_integral_and_cluster_citations_are_preserved()
 {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = MarkdownParser;
    let document = concat!(
        "Kuhn argued that @kuhn1962 [p. 10] changed science.\n\n",
        "Later work supports this [see @smith2010, p. 12; @kuhn1962, ch. 3].",
    );

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_has_line(
        &output,
        "Kuhn argued that Kuhn (1962, p. 10) changed science.",
        "integral markdown citation should render in prose",
    );
    assert_output_has_line(
        &output,
        "Later work supports this (Kuhn, 1962, chap. 3; see Smith, 2010, p. 12).",
        "bracketed markdown cite cluster should render with prefix and locator",
    );
}

fn given_markdown_integral_note_citation_when_rendered_with_a_note_style_then_a_generated_note_is_emitted()
 {
    let processor =
        example_document_processor("styles/embedded/chicago-shortened-notes-bibliography.yaml");
    let parser = MarkdownParser;
    let document = "Narrative mention @smith2010 introduces the argument.";

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_has_line(
        &output,
        "Narrative mention Smith[^citum-auto-1] introduces the argument.",
        "note-style markdown integral citation should anchor correctly",
    );
    assert_output_has_line(
        &output,
        "[^citum-auto-1]: Smith, _Nationalism: Theory, Ideology, History_.",
        "generated note should be emitted for markdown citation",
    );
}

fn given_markdown_citation_inside_manual_footnote_when_rendered_with_note_style_then_it_renders_in_place()
 {
    // When the user writes their own [^n]: block containing a citation,
    // the citation should render inline inside that definition (ManualFootnote
    // placement) rather than generating a second auto-footnote.
    let processor =
        example_document_processor("styles/embedded/chicago-shortened-notes-bibliography.yaml");
    let parser = MarkdownParser;
    let document = "See note[^1].\n\n[^1]: Early work [@kuhn1962] supports this.";

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // The manual footnote anchor must appear in prose.
    assert_output_includes(
        &output,
        "[^1]",
        "manual footnote reference should appear in prose",
    );
    // The rendered citation should appear inside the footnote definition, not
    // as a separate auto-generated note.
    assert_output_includes(
        &output,
        "[^1]: Early work",
        "footnote definition body should be preserved",
    );
    assert_output_includes(
        &output,
        "Kuhn",
        "citation inside manual footnote should be rendered in place",
    );
    // No auto-generated note should be created for this citation.
    assert_output_excludes(
        &output,
        "[^citum-auto-",
        "no auto-footnote should be generated for a ManualFootnote citation",
    );
}

// --- Grouped Bibliography Scenarios ---

fn given_grouped_primary_and_secondary_sources_when_rendered_then_both_group_headings_and_entries_appear()
 {
    let style = load_style("styles/embedded/chicago-author-date-18th.yaml");
    let bibliography =
        load_bibliography(&project_root().join("tests/fixtures/grouping/primary-secondary.json"))
            .expect("grouping fixture should parse");

    let processor = Processor::new(style, bibliography);
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "Grouping check [@interview-1978; @ms-archive-1901; @journal-2021].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_has_line(
        &output,
        "## Primary Sources",
        "grouped bibliography should include the primary heading",
    );
    assert_output_has_line(
        &output,
        "## Secondary Sources",
        "grouped bibliography should include the secondary heading",
    );
    assert_output_includes(
        &output,
        "Field Notes from the Delta Survey",
        "grouped bibliography should include the primary-source entry",
    );
    assert_output_includes(
        &output,
        "Trade Networks in the Early Modern Atlantic",
        "grouped bibliography should include the secondary-source entry",
    );
}

fn given_group_local_disambiguation_when_rendering_multilingual_groups_then_year_suffixes_restart_within_each_group()
 {
    let mut style = load_style("styles/experimental/multilingual-academic.yaml");
    style
        .options
        .get_or_insert_with(Default::default)
        .processing = Some(Processing::Custom(ProcessingCustom {
        base: None,
        disambiguate: Some(Disambiguation {
            year_suffix: false,
            ..Default::default()
        }),
        ..Default::default()
    }));
    style
        .bibliography
        .get_or_insert_with(Default::default)
        .options
        .get_or_insert_with(Default::default)
        .processing = Some(Processing::Custom(ProcessingCustom {
        base: None,
        disambiguate: Some(Disambiguation {
            year_suffix: true,
            ..Default::default()
        }),
        ..Default::default()
    }));

    let bibliography =
        load_bibliography(&project_root().join("tests/fixtures/grouping/multilingual-groups.json"))
            .expect("multilingual grouping fixture should parse");
    let processor = Processor::new(style, bibliography);
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "Disambiguation check [@vi-kuhn-a; @vi-kuhn-b; @en-kuhn-a; @en-kuhn-b].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // Split on the known group headings to get per-group bibliography text.
    let vi_block = output
        .split("## Vietnamese Sources")
        .nth(1)
        .and_then(|s| s.split("## Western Sources").next())
        .unwrap_or_else(|| panic!("Vietnamese Sources section missing: {output}"));
    let en_block = output
        .split("## Western Sources")
        .nth(1)
        .unwrap_or_else(|| panic!("Western Sources section missing: {output}"));

    // Each group must have both 2020a and 2020b — suffixes restart at 'a' per group.
    assert_output_includes_all(
        vi_block,
        &["2020a", "2020b"],
        "Vietnamese group should restart suffixes at 2020a and 2020b",
    );
    assert_output_includes_all(
        en_block,
        &["2020a", "2020b"],
        "Western group should restart suffixes at 2020a and 2020b",
    );

    // Suffixes must not bleed across the boundary: neither group should contain
    // a suffix that belongs to the other (i.e. no 2020c or 2020d anywhere).
    assert_output_excludes_all(
        &output,
        &["2020c", "2020d"],
        "cross-group suffixes should not leak",
    );
}

fn given_juris_m_legal_grouping_when_rendered_then_headings_follow_the_expected_legal_hierarchy() {
    let style = load_style("styles/experimental/jm-chicago-legal.yaml");
    let bibliography =
        load_bibliography(&project_root().join("tests/fixtures/grouping/legal-hierarchy.json"))
            .expect("legal hierarchy fixture should parse");

    let processor = Processor::new(style, bibliography);
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "Legal grouping [@brown1954; @civilrights1964; @versailles1919; @hart1994].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    let cases = output
        .find("## Cases")
        .expect("missing cases heading in grouped bibliography");
    let statutes = output
        .find("## Statutes")
        .expect("missing statutes heading in grouped bibliography");
    let treaties = output
        .find("## Treaties and International Agreements")
        .expect("missing treaties heading in grouped bibliography");
    let secondary = output
        .find("## Secondary Sources")
        .expect("missing secondary heading in grouped bibliography");

    assert!(cases < statutes, "expected Cases before Statutes: {output}");
    assert!(
        statutes < treaties,
        "expected Statutes before Treaties: {output}"
    );
    assert!(
        treaties < secondary,
        "expected Treaties before Secondary: {output}"
    );
}

fn given_an_english_locale_variant_when_group_headings_are_localized_then_the_language_tag_fallback_is_used()
 {
    let style = load_style("styles/embedded/chicago-author-date-18th.yaml");
    let bibliography =
        load_bibliography(&project_root().join("tests/fixtures/grouping/primary-secondary.json"))
            .expect("grouping fixture should parse");

    let mut locale = Locale::en_us();
    locale.locale = "en-GB".to_string();

    let processor = Processor::with_locale(style, bibliography, locale);
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "Locale fallback check [@interview-1978; @journal-2021].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // chicago-author-date headings are localized with en-US + en.
    // en-GB should fall back to the language tag (en).
    assert_output_has_line(
        &output,
        "## Primary Sources",
        "English locale fallback should resolve the primary heading",
    );
    assert_output_has_line(
        &output,
        "## Secondary Sources",
        "English locale fallback should resolve the secondary heading",
    );
}

mod rendering_formats {
    use super::announce_behavior;

    #[test]
    fn simple_author_date_html_appends_a_bibliography_heading() {
        announce_behavior(
            "Rendering a simple author-date document as HTML should append a Bibliography heading and preserve the prose.",
        );
        super::given_simple_author_date_document_when_rendered_as_html_then_a_bibliography_heading_is_appended();
    }

    #[test]
    fn simple_author_date_djot_does_not_emit_html_tags() {
        announce_behavior(
            "Rendering the same document as Djot should produce markdown headings rather than HTML tags.",
        );
        super::given_simple_author_date_document_when_rendered_as_djot_then_html_tags_are_not_emitted();
    }
}

mod example_documents {
    use super::announce_behavior;

    #[test]
    fn mla_html_keeps_citation_markup_unescaped() {
        announce_behavior(
            "The MLA example document should emit real citation and bibliography HTML instead of escaped markup.",
        );
        super::given_example_mla_document_when_rendered_as_html_then_citation_markup_is_not_escaped(
        );
    }

    #[test]
    fn mla_plain_text_shows_integral_name_memory() {
        announce_behavior(
            "The MLA plain-text example should shorten repeated narrative citations after the first integral mention.",
        );
        super::given_example_mla_document_when_rendered_as_plain_text_then_integral_name_memory_is_visible();
    }

    #[test]
    fn two_authors_with_same_surname_both_get_first_form() {
        announce_behavior(
            "Two different integral authors sharing a family name must each render in full (First) form on their own first mention.",
        );
        super::given_two_authors_with_same_surname_when_both_cited_integrally_then_each_gets_first_form();
    }

    #[test]
    fn org_abbreviation_memory_renders_full_then_short_on_first_and_short_on_subsequent() {
        announce_behavior(
            "With org-abbreviation-memory configured, the first integral mention of an org shows full name + abbreviation; subsequent shows abbreviation only.",
        );
        super::given_org_with_short_name_when_org_abbreviation_memory_configured_and_cited_integrally_twice_then_first_shows_full_then_subsequent_shows_short();
    }

    #[test]
    fn apa_plain_text_integral_citations_keep_locators() {
        announce_behavior(
            "The APA plain-text example should keep locators inside integral citations throughout the document.",
        );
        super::given_example_apa_document_when_rendered_as_plain_text_then_integral_citations_include_locators();
    }

    #[test]
    fn chicago_note_plain_text_keeps_integral_note_anchors() {
        announce_behavior(
            "The Chicago note example should preserve narrative note anchors and keep manual-note content intact.",
        );
        super::given_example_chicago_note_document_when_rendered_as_plain_text_then_integral_mentions_keep_their_note_anchor();
    }
}

mod note_flow {
    use super::announce_behavior;

    #[test]
    fn chicago_note_flow_preserves_authored_ibid_anchor_and_reduced_form() {
        announce_behavior(
            "A Chicago note-flow authored note should keep the narrative anchor separate from the generated ibid text.",
        );
        super::given_chicago_note_flow_document_when_authored_ibid_appears_in_notes_then_anchor_and_reduced_form_stay_separate();
    }

    #[test]
    fn locator_repeats_keep_the_anchor_and_locator() {
        announce_behavior(
            "A repeated note citation with a locator should keep both the narrative anchor and the locator.",
        );
        super::given_chicago_note_locator_repeat_when_integral_ibid_is_rendered_then_anchor_and_locator_are_preserved();
    }

    #[test]
    fn configured_page_labels_are_preserved_in_manual_note_ibid_with_locator() {
        announce_behavior(
            "If locator rendering is configured to show page labels, a repeated manual note should keep the labeled page locator in the reduced ibid text.",
        );
        super::given_page_labels_are_configured_when_integral_ibid_is_rendered_then_the_labeled_page_locator_is_preserved();
    }

    #[test]
    fn chapter_locator_repeats_keep_the_labeled_locator() {
        announce_behavior(
            "A repeated manual note with a chapter locator should keep the chapter label in the reduced ibid text.",
        );
        super::given_chapter_locator_repeat_when_integral_ibid_is_rendered_then_the_labeled_chapter_locator_is_preserved();
    }

    #[test]
    fn locale_ibid_term_is_used_when_the_style_has_no_override() {
        announce_behavior(
            "If a note style does not override ibid, the localized term should be used without extra base punctuation.",
        );
        super::given_locale_specific_ibid_term_when_the_style_has_no_ibid_override_then_the_locale_term_is_used_without_base_suffix_punctuation();
    }

    #[test]
    fn explicit_style_ibid_suffix_overrides_the_locale_term() {
        announce_behavior(
            "If the style defines its own ibid suffix, that style-specific suffix should override the locale term.",
        );
        super::given_explicit_style_ibid_suffix_when_locale_also_defines_ibid_then_the_style_suffix_wins();
    }

    #[test]
    fn missing_note_anchor_falls_back_to_reduced_citation_text() {
        announce_behavior(
            "If a repeated note cite has no reusable anchor, the reduced citation text should still appear cleanly.",
        );
        super::given_missing_note_anchor_when_integral_ibid_is_rendered_then_the_reduced_citation_still_appears_without_concatenation();
    }

    #[test]
    fn empty_note_flow_does_not_emit_a_bibliography_heading() {
        announce_behavior(
            "A note-flow document with no bibliography entries should not emit an empty bibliography heading.",
        );
        super::given_chicago_note_flow_document_when_no_bibliography_entries_are_needed_then_no_heading_is_emitted();
    }

    #[test]
    fn citation_only_style_skips_bibliography_output_without_skipping_citations() {
        announce_behavior(
            "A resolved style without a bibliography should still render citations while emitting no bibliography entries, content, or heading.",
        );
        super::given_a_style_without_a_bibliography_when_rendering_a_document_then_citations_remain_and_bibliography_output_is_empty();
    }

    #[test]
    fn non_note_styles_never_emit_ibid_in_the_note_flow_example() {
        announce_behavior(
            "Running the note-flow example under non-note styles should never emit ibid.",
        );
        super::given_non_note_styles_when_rendering_the_note_flow_example_then_ibid_is_never_emitted();
    }
}

fn given_markdown_document_with_pipe_table_when_rendered_as_markdown_then_body_passes_through_verbatim()
 {
    // A GFM pipe table, a fenced code block, and a citation in prose — body
    // markup must survive verbatim; only the [@key] marker is replaced.
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = MarkdownParser;
    let pipe_table = "| Column A | Column B |\n|----------|----------|\n| cell 1   | cell 2   |";
    let code_block = "```rust\nfn hello() {}\n```";
    let document =
        format!("# Introduction\n\nAs argued in [@kuhn1962].\n\n{pipe_table}\n\n{code_block}\n");

    let output = processor
        .process_document::<_, citum_engine::render::markdown::Markdown>(
            &document,
            &parser,
            DocumentFormat::Markdown,
        )
        .expect("document should render");

    // Pipe table lines must be unchanged.
    assert_output_includes_all(
        &output,
        &[
            "| Column A | Column B |",
            "|----------|----------|",
            "| cell 1   | cell 2   |",
        ],
        "pipe table should pass through unchanged",
    );

    // Fenced code block must be unchanged.
    assert_output_includes(
        &output,
        "```rust\nfn hello() {}\n```",
        "fenced code block should pass through unchanged",
    );

    // Citation marker replaced with rendered inline text.
    assert_output_excludes(
        &output,
        "[@kuhn1962]",
        "raw citation marker should be replaced",
    );
    assert_output_has_line(
        &output,
        "As argued in (Kuhn, 1962).",
        "rendered citation should replace the marker with the full APA cite",
    );

    // Bibliography heading present.
    assert_output_has_line(
        &output,
        "# Bibliography",
        "bibliography heading should be present",
    );
}

fn given_note_style_markdown_document_when_rendered_as_markdown_then_commonmark_footnote_syntax_is_emitted()
 {
    // Note styles emit [^label] anchors in prose and [^label]: … definitions
    // at the end — the CommonMark+footnotes extension used by Pandoc/GFM.
    let processor =
        example_document_processor("styles/embedded/chicago-shortened-notes-bibliography.yaml");
    let parser = MarkdownParser;
    let document = "First claim [@kuhn1962]. Second claim [@smith2010].";

    let output = processor
        .process_document::<_, citum_engine::render::markdown::Markdown>(
            document,
            &parser,
            DocumentFormat::Markdown,
        )
        .expect("document should render");

    // Footnote anchors in prose.
    assert_output_has_line(
        &output,
        "First claim.[^citum-auto-1] Second claim.[^citum-auto-2]",
        "footnote anchors should be emitted in prose",
    );

    // Footnote definitions with rendered content (CommonMark emphasis).
    assert_output_includes_all(
        &output,
        &["[^citum-auto-1]:", "[^citum-auto-2]:"],
        "footnote definitions should be emitted",
    );

    // No raw citation markers remain.
    assert_output_excludes_all(
        &output,
        &["[@kuhn1962]", "[@smith2010]"],
        "raw citation markers should be replaced",
    );
}

mod markdown_documents {
    use super::announce_behavior;

    #[test]
    fn pandoc_author_date_syntax_preserves_integral_and_cluster_citations() {
        announce_behavior(
            "Pandoc markdown citations should preserve both integral citations and citation clusters through rendering.",
        );
        super::given_pandoc_markdown_author_date_syntax_when_rendered_then_integral_and_cluster_citations_are_preserved();
    }

    #[test]
    fn note_style_markdown_integral_citations_emit_generated_notes() {
        announce_behavior(
            "Markdown integral citations rendered with a note style should generate note content instead of inline prose cites.",
        );
        super::given_markdown_integral_note_citation_when_rendered_with_a_note_style_then_a_generated_note_is_emitted();
    }

    #[test]
    fn markdown_output_passes_pipe_table_and_code_block_through_verbatim() {
        announce_behavior(
            "Markdown output format must pass block-level markup (pipe tables, fenced code blocks) through verbatim while replacing citation markers with rendered inline text.",
        );
        super::given_markdown_document_with_pipe_table_when_rendered_as_markdown_then_body_passes_through_verbatim();
    }

    #[test]
    fn note_style_markdown_output_emits_commonmark_footnote_syntax() {
        announce_behavior(
            "A note-style document rendered as Markdown output should emit [^n] anchors in prose and [^n]: … definitions, following the CommonMark+footnotes extension used by Pandoc and GFM.",
        );
        super::given_note_style_markdown_document_when_rendered_as_markdown_then_commonmark_footnote_syntax_is_emitted();
    }

    #[test]
    fn citation_inside_manual_footnote_renders_in_place_not_as_auto_note() {
        announce_behavior(
            "A citation found inside a user-authored [^n]: footnote definition should render inline within that definition rather than generating a separate auto-footnote.",
        );
        super::given_markdown_citation_inside_manual_footnote_when_rendered_with_note_style_then_it_renders_in_place();
    }

    #[test]
    fn chicago_author_date_markdown_citation_has_no_spurious_space() {
        announce_behavior(
            "Chicago author-date Markdown citation with locator renders as '(Kuhn 1962, 5)'.",
        );
        super::given_chicago_author_date_markdown_citation_when_rendered_then_no_spurious_space_before_locator();
    }

    #[test]
    fn chicago_author_date_suppressed_citation_has_no_spurious_space() {
        announce_behavior(
            "Suppress-author Chicago author-date Markdown citation with locator renders as '(1962, 5)'.",
        );
        super::given_chicago_author_date_markdown_suppressed_citation_when_rendered_then_no_spurious_space();
    }
}

// --- Chicago author-date Markdown rendering (#844) ---

fn given_chicago_author_date_markdown_citation_when_rendered_then_no_spurious_space_before_locator()
{
    let processor = example_document_processor("styles/embedded/chicago-author-date-18th.yaml");
    let parser = MarkdownParser;

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "See [@kuhn1962, p. 5].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // Chicago author-date should render the full sentence with author, year,
    // and locator, with no spurious space before the comma.
    assert_output_has_line(
        &output,
        "See (Kuhn 1962, 5).",
        "Chicago author-date locator sentence should be exactly 'See (Kuhn 1962, 5).'",
    );
}

fn given_chicago_author_date_markdown_suppressed_citation_when_rendered_then_no_spurious_space() {
    let processor = example_document_processor("styles/embedded/chicago-author-date-18th.yaml");
    let parser = MarkdownParser;

    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            "See [-@kuhn1962, p. 5].",
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    // Suppress-author form should render the full sentence with year and
    // locator only, with no author or spurious space before the comma.
    assert_output_has_line(
        &output,
        "See (1962, 5).",
        "Chicago author-date suppress-author locator sentence should be exactly 'See (1962, 5).'",
    );
}

// --- Djot Adapter & Pipeline Tests ---

fn djot_parser_extracts_citations_from_simple_document() {
    let document = "A citation [@kuhn1962] appears here.";
    let parser = DjotParser;

    let parsed = parser.parse_document(document, &Locale::en_us());
    assert_eq!(parsed.citations.len(), 1, "Should extract one citation");
    assert_eq!(parsed.citations[0].citation.items[0].id, "kuhn1962");
}

fn djot_parser_respects_manual_footnotes() {
    let document = "Text[^m1].\n\n[^m1]: See [@kuhn1962].";
    let parser = DjotParser;

    let parsed = parser.parse_document(document, &Locale::en_us());
    assert_eq!(
        parsed.manual_note_order.len(),
        1,
        "Should track one manual note"
    );
    assert_eq!(parsed.manual_note_order[0], "m1");
    assert_eq!(
        parsed.citations.len(),
        1,
        "Should extract citation in footnote"
    );
}

fn djot_parsing_handles_multiple_citations() {
    let document = "First [@smith2020] and second [@jones2021] citations.";
    let parser = DjotParser;

    let parsed = parser.parse_document(document, &Locale::en_us());
    assert_eq!(
        parsed.citations.len(),
        2,
        "Should extract two separate citations"
    );
    assert_eq!(parsed.citations[0].citation.items[0].id, "smith2020");
    assert_eq!(parsed.citations[1].citation.items[0].id, "jones2021");
}

fn djot_note_preserves_italic_markup_in_html_bibliography() {
    use citum_engine::render::html::Html;
    use citum_schema::template::{SimpleVariable, TemplateVariable};

    let style = Style {
        info: StyleInfo {
            title: Some("Note Djot Test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(vec![citum_schema::template::TemplateComponent::Variable(
                TemplateVariable {
                    variable: SimpleVariable::Note,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        citum_schema::reference::InputReference::Monograph(Box::new(
            citum_schema::reference::Monograph {
                id: Some("ref1".into()),
                r#type: citum_schema::reference::MonographType::Book,
                title: Some(citum_schema::reference::Title::Single(
                    "Test Book".to_string(),
                )),
                issued: citum_schema::reference::DateValue::new("2024".to_string()),
                note: Some(citum_schema::reference::RichText::Djot {
                    djot: "_italic_".to_string(),
                }),
                ..Default::default()
            },
        )),
    );

    let output = Processor::new(style, bib).render_bibliography_with_format_standalone::<Html>();
    assert_eq!(
        output,
        "<div class=\"citum-bibliography\">\n<div class=\"citum-entry\" id=\"ref-ref1\" data-year=\"2024\" data-title=\"Test Book\"><span class=\"citum-note\"><em>italic</em></span></div>\n</div>",
        "Djot note should preserve italic markup in HTML bibliography"
    );
}

fn djot_note_sentence_case_does_not_restart_across_markup_boundaries() {
    use citum_engine::render::html::Html;
    use citum_schema::options::titles::TextCase;
    use citum_schema::template::{Rendering, SimpleVariable, TemplateComponent, TemplateVariable};

    let style = Style {
        info: StyleInfo {
            title: Some("Note Djot Sentence Case Test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(vec![TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Note,
                rendering: Rendering {
                    text_case: Some(TextCase::Sentence),
                    ..Default::default()
                },
                ..Default::default()
            })]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        citum_schema::reference::InputReference::Monograph(Box::new(
            citum_schema::reference::Monograph {
                id: Some("ref1".into()),
                r#type: citum_schema::reference::MonographType::Book,
                title: Some(citum_schema::reference::Title::Single(
                    "Test Book".to_string(),
                )),
                issued: citum_schema::reference::DateValue::new("2024".to_string()),
                note: Some(citum_schema::reference::RichText::Djot {
                    djot: "foo _BAR_ baz".to_string(),
                }),
                ..Default::default()
            },
        )),
    );

    let output = Processor::new(style, bib).render_bibliography_with_format_standalone::<Html>();
    assert_eq!(
        output,
        "<div class=\"citum-bibliography\">\n<div class=\"citum-entry\" id=\"ref-ref1\" data-year=\"2024\" data-title=\"Test Book\"><span class=\"citum-note\">Foo <em>bar</em> baz</span></div>\n</div>",
        "Djot note sentence case should not restart inside markup"
    );
}

mod djot_adapter {
    use super::announce_behavior;

    #[test]
    fn simple_document_citation_extraction() {
        announce_behavior(
            "The Djot parser adapter should extract citations from simple documents.",
        );
        super::djot_parser_extracts_citations_from_simple_document();
    }

    #[test]
    fn manual_footnotes_are_tracked() {
        announce_behavior(
            "The Djot parser should track manual footnotes and citations within them.",
        );
        super::djot_parser_respects_manual_footnotes();
    }

    #[test]
    fn multiple_citations_extraction() {
        announce_behavior(
            "The Djot parser should extract multiple citations from a single document.",
        );
        super::djot_parsing_handles_multiple_citations();
    }

    #[test]
    fn note_preserves_italic_markup_in_html_bibliography() {
        announce_behavior("Djot note preserves italic markup in HTML bibliography.");
        super::djot_note_preserves_italic_markup_in_html_bibliography();
    }

    #[test]
    fn note_sentence_case_does_not_restart_across_markup_boundaries() {
        announce_behavior("Djot note sentence case does not restart across markup boundaries.");
        super::djot_note_sentence_case_does_not_restart_across_markup_boundaries();
    }
}

mod grouped_bibliography {
    use super::announce_behavior;

    #[test]
    fn primary_and_secondary_sources_render_both_headings_and_entries() {
        announce_behavior(
            "A grouped bibliography should render both primary and secondary headings along with their entries.",
        );
        super::given_grouped_primary_and_secondary_sources_when_rendered_then_both_group_headings_and_entries_appear();
    }

    #[test]
    fn group_local_disambiguation_restarts_year_suffixes_per_group() {
        announce_behavior(
            "Group-local disambiguation should restart year suffixes inside each bibliography group.",
        );
        super::given_group_local_disambiguation_when_rendering_multilingual_groups_then_year_suffixes_restart_within_each_group();
    }

    #[test]
    fn juris_m_legal_grouping_follows_the_expected_hierarchy() {
        announce_behavior(
            "Juris-M legal bibliography grouping should follow the expected legal hierarchy and headings.",
        );
        super::given_juris_m_legal_grouping_when_rendered_then_headings_follow_the_expected_legal_hierarchy();
    }

    #[test]
    fn english_locale_variants_fall_back_to_the_language_tag_for_group_headings() {
        announce_behavior(
            "English locale variants should fall back to their language tag when no localized group heading term exists.",
        );
        super::given_an_english_locale_variant_when_group_headings_are_localized_then_the_language_tag_fallback_is_used();
    }
}

// --- Document bibliography single-pass fast path (csl26-plaz) ---
//
// `Processor::render_document_bibliography` renders each cited reference's
// template exactly once for the flat and sort-partitioned-sections cases
// instead of once for the appended bibliography string and again for the
// per-entry API data. These scenarios exercise that facade through the
// public `process_document` entry point (not the lower-level `Processor`
// methods used directly elsewhere in this file), since that facade is the
// one whose render pass was unified.

/// A book-tagged legacy JSON reference with an explicit `language`, used to
/// drive [`BibliographyPartitionKind::Language`] sectioning.
fn language_tagged_reference(
    id: &str,
    family: &str,
    given: &str,
    title: &str,
    language: &str,
) -> Reference {
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": id,
        "type": "book",
        "title": title,
        "language": language,
        "author": [{ "family": family, "given": given }]
    }))
    .expect("language-tagged fixture should parse");
    legacy.into()
}

fn given_a_large_library_and_a_small_cited_subset_when_rendering_a_flat_document_bibliography_then_only_cited_entries_appear()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Flat Fast Path Test".to_string()),
            id: Some("flat-fast-path-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                ..Default::default()
            }),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_title!(Primary, prefix = ". "),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // A "large library" of never-cited filler references, plus two cited ones.
    let mut bibliography = indexmap::IndexMap::new();
    for i in 0..10 {
        let id = format!("filler{i:02}");
        bibliography.insert(
            id.clone(),
            make_book(
                &id,
                &format!("Filler{i:02}"),
                "F.",
                2000 + i,
                &format!("Filler Title {i:02}"),
            ),
        );
    }
    bibliography.insert(
        "cited-a".to_string(),
        make_book("cited-a", "Cited", "Alice", 2015, "Cited Alpha"),
    );
    bibliography.insert(
        "cited-b".to_string(),
        make_book("cited-b", "Cited", "Zed", 2016, "Cited Beta"),
    );

    let processor = Processor::new(style, bibliography);
    let document = "Alpha work [@cited-a] and beta work [@cited-b].";
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Alice Cited. Cited Alpha.",
        "the first cited entry should render in full",
    );
    assert_output_includes(
        &output,
        "Zed Cited. Cited Beta.",
        "the second cited entry should render in full",
    );
    for i in 0..10 {
        assert_output_excludes(
            &output,
            &format!("Filler Title {i:02}"),
            "uncited references from a large library must not appear in a flat document bibliography",
        );
    }
}

fn given_an_uncited_reference_between_two_same_author_cited_entries_when_rendering_a_flat_document_bibliography_then_the_second_cited_entry_still_substitutes()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Flat Substitution Gap Test".to_string()),
            id: Some("flat-substitution-gap-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                subsequent_author_substitute: Some("———".to_string()),
                ..Default::default()
            }),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_title!(Primary, prefix = ". "),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "smith-a".to_string(),
        make_book("smith-a", "Smith", "Ann", 2018, "Alpha"),
    );
    // Never cited: sits between the two cited Smith entries in sorted order.
    bibliography.insert(
        "smith-b".to_string(),
        make_book("smith-b", "Smith", "Ann", 2019, "Beta"),
    );
    bibliography.insert(
        "smith-c".to_string(),
        make_book("smith-c", "Smith", "Ann", 2020, "Gamma"),
    );

    let processor = Processor::new(style, bibliography);
    let document = "First work [@smith-a] and second work [@smith-c].";
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Ann Smith. Alpha.",
        "the first cited entry should render its author in full",
    );
    assert_output_includes(
        &output,
        "———. Gamma.",
        "the second cited entry should substitute, since the immediately preceding \
         *cited* entry shares its author — the uncited entry between them in the \
         full library must not break the substitution chain",
    );
    assert_output_excludes(&output, "Beta", "the uncited entry must not appear");
}

fn given_sort_partitioned_sections_and_subsequent_author_substitution_when_rendering_a_document_bibliography_then_substitution_resets_at_each_section_boundary()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Partition Substitution Boundary Test".to_string()),
            id: Some("partition-substitution-boundary-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                subsequent_author_substitute: Some("———".to_string()),
                sort_partitioning: Some(BibliographySortPartitioning {
                    by: BibliographyPartitionKind::Language,
                    mode: BibliographyPartitionMode::Sections,
                    order: vec!["ru".to_string(), "en".to_string()],
                    headings: std::collections::HashMap::new(),
                    unknown_fields: std::collections::BTreeMap::new(),
                }),
                ..Default::default()
            }),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_title!(Primary, prefix = ". "),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "ru-a".to_string(),
        language_tagged_reference("ru-a", "Smith", "John", "Alpha", "ru"),
    );
    bibliography.insert(
        "ru-b".to_string(),
        language_tagged_reference("ru-b", "Smith", "John", "Beta", "ru"),
    );
    // Same author as the Russian-section entries, but placed in a different
    // partition section by language.
    bibliography.insert(
        "en-a".to_string(),
        language_tagged_reference("en-a", "Smith", "John", "Gamma", "en"),
    );

    let processor = Processor::new(style, bibliography);
    let document = "[@ru-a] [@ru-b] [@en-a]";
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "John Smith. Alpha.",
        "the first entry in the Russian section should render its author in full",
    );
    assert_output_in_order(
        &output,
        "John Smith. Alpha.",
        "———. Beta.",
        "the second entry in the same (Russian) section should substitute",
    );
    assert_output_includes(
        &output,
        "John Smith. Gamma.",
        "the English-section entry shares an author with the Russian section, \
         but substitution must reset at the section boundary rather than \
         carrying over from the previous section's last entry",
    );
    assert_output_excludes(
        &output,
        "———. Gamma.",
        "substitution must not cross a partition section boundary",
    );
}

fn given_groups_enabled_false_with_groups_still_configured_when_rendering_a_document_bibliography_then_it_renders_flat()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Groups Disabled Fast Path Test".to_string()),
            id: Some("groups-disabled-fast-path-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                entry_suffix: Some(".".into()),
                ..Default::default()
            }),
            groups_enabled: false,
            // Selector matches only the "book" entry, leaving the "article-journal"
            // entry unassigned — this makes the group-vs-flat routing decision
            // observable: taking the (buggy) custom-groups branch would populate
            // exactly one group with a non-empty unassigned remainder, so the
            // group's heading would NOT be suppressed by the single-group
            // heading-suppression rule and would render, unlike the flat path.
            groups: Some(vec![citum_schema::grouping::BibliographyGroup {
                id: "ignored".to_string(),
                heading: Some(citum_schema::grouping::GroupHeading::Literal {
                    literal: "Should Not Appear".to_string(),
                }),
                selector: citum_schema::grouping::GroupSelector {
                    ref_type: Some(citum_schema::template::TypeSelector::Single(
                        "book".to_string(),
                    )),
                    ..Default::default()
                },
                sort: None,
                template: None,
                disambiguate: None,
            }]),
            template: Some(vec![
                citum_schema::tc_contributor!(Author, Long),
                citum_schema::tc_title!(Primary, prefix = ". "),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "alpha".to_string(),
        make_book("alpha", "Adams", "A.", 2020, "Alpha"),
    );
    bibliography.insert(
        "beta".to_string(),
        make_article("beta", "Baker", "B.", 2021, "Beta"),
    );

    let processor = Processor::new(style, bibliography);
    let document = "First [@alpha] and second [@beta].";
    let parser = DjotParser;
    let output = processor
        .process_document::<_, citum_engine::render::plain::PlainText>(
            document,
            &parser,
            DocumentFormat::Plain,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "Alpha",
        "the first cited entry should still render when groups_enabled is false",
    );
    assert_output_includes(
        &output,
        "Beta",
        "the second cited entry should still render when groups_enabled is false",
    );
    assert_output_excludes(
        &output,
        "Should Not Appear",
        "groups_enabled: false must disable groups: configuration entirely — no group \
         heading should render, matching the documented \"render a flat bibliography \
         instead\" contract",
    );
}

// --- Body markup conversion for terminal formats (#824) ---

fn given_markdown_block_quote_when_rendered_as_typst_then_quote_block_is_emitted() {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = MarkdownParser;
    let document = concat!(
        "> This is a block quote with *italic* text,\n",
        "> and **strong** text. So is __this__.\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::typst::Typst>(
            document,
            &parser,
            DocumentFormat::Typst,
        )
        .expect("document should render");

    assert_output_includes_all(
        &output,
        &["#quote(block: true)", "#emph[italic]"],
        "markdown block quote should produce Typst quote and emphasis markup",
    );
    assert!(
        !output.starts_with('>'),
        "raw markdown block-quote syntax should not appear in Typst output, got: {output}"
    );
}

fn given_djot_block_quote_when_rendered_as_typst_then_quote_block_is_emitted() {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "> This is a block quote with _italic_ text,\n",
        "> and *bold*.\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::typst::Typst>(
            document,
            &parser,
            DocumentFormat::Typst,
        )
        .expect("document should render");

    assert_output_includes_all(
        &output,
        &["#quote(block: true)", "#emph[italic]"],
        "Djot block quote should produce Typst quote and emphasis markup",
    );
}

fn given_djot_notice_with_nested_strong_when_rendered_as_typst_then_strong_markup_is_composable() {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "{.citum-demo-notice}\n",
        "**Note**: This notice is outside any footnote.\n\n",
        "---\n\n",
        "**Features illustrated**: source markup, thematic breaks, and citations.\n\n",
        "A later citation still resolves [@kuhn1962].\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::typst::Typst>(
            document,
            &parser,
            DocumentFormat::Typst,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "#strong[#strong[Note]]",
        "Djot nested strong input should render as composable Typst strong markup",
    );
    assert_output_excludes(
        &output,
        "**Note**",
        "raw Djot strong delimiters should not leak into Typst output",
    );
    assert_output_excludes(
        &output,
        "{.citum-demo-notice}",
        "raw Djot paragraph attributes should not leak into Typst output",
    );
    assert_output_includes_all(
        &output,
        &["#link(<ref-kuhn1962>)", "Kuhn"],
        "citation placeholder replacement should still produce Typst links",
    );
}

fn given_djot_notice_with_nested_strong_when_rendered_as_djot_then_source_markup_passes_through() {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = DjotParser;
    let document = concat!(
        "{.citum-demo-notice}\n",
        "**Note**: This notice is outside any footnote.\n\n",
        "---\n\n",
        "A later citation still resolves [@kuhn1962].\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::djot::Djot>(
            document,
            &parser,
            DocumentFormat::Djot,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "**Note**",
        "Djot passthrough output should preserve source body markup",
    );
    assert_output_includes(
        &output,
        "{.citum-demo-notice}",
        "Djot passthrough output should preserve source attributes",
    );
    assert_output_includes(
        &output,
        "Kuhn",
        "citation text should be rendered while body markup passes through",
    );
    assert_output_excludes(
        &output,
        "[@kuhn1962]",
        "raw citation marker should not pass through",
    );
}

fn given_markdown_block_quote_when_rendered_as_latex_then_quote_environment_is_emitted() {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = MarkdownParser;
    let document = concat!(
        "> This is a block quote with *italic* text\n",
        "> and **strong** text.\n",
    );

    let output = processor
        .process_document::<_, citum_engine::render::latex::Latex>(
            document,
            &parser,
            DocumentFormat::Latex,
        )
        .expect("document should render");

    assert_output_includes_all(
        &output,
        &["\\begin{quote}", "\\emph{italic}", "\\textbf{strong}"],
        "markdown block quote should produce LaTeX quote, emph, and strong markup",
    );
}

fn given_markdown_citation_inside_prose_when_rendered_as_typst_then_citation_and_markup_both_appear()
 {
    let processor = example_document_processor("styles/embedded/apa-7th.yaml");
    let parser = MarkdownParser;
    let document = "A paragraph with *emphasis* and a citation [@kuhn1962].";

    let output = processor
        .process_document::<_, citum_engine::render::typst::Typst>(
            document,
            &parser,
            DocumentFormat::Typst,
        )
        .expect("document should render");

    assert_output_includes(
        &output,
        "#emph[emphasis]",
        "markdown emphasis in paragraph should produce Typst emphasis markup",
    );
    assert_output_includes_all(
        &output,
        &["Kuhn", "1962"],
        "citation should render in Typst output",
    );
}

mod flat_and_partitioned_bibliography_fast_path {
    use super::announce_behavior;

    #[test]
    fn large_library_with_small_cited_subset_renders_only_cited_entries() {
        announce_behavior(
            "A flat document bibliography over a large library should render only the cited subset, not every loaded reference.",
        );
        super::given_a_large_library_and_a_small_cited_subset_when_rendering_a_flat_document_bibliography_then_only_cited_entries_appear();
    }

    #[test]
    fn uncited_gap_does_not_break_subsequent_author_substitution() {
        announce_behavior(
            "An uncited reference sitting between two same-author cited entries must not break subsequent-author substitution for the second cited entry.",
        );
        super::given_an_uncited_reference_between_two_same_author_cited_entries_when_rendering_a_flat_document_bibliography_then_the_second_cited_entry_still_substitutes();
    }

    #[test]
    fn partition_sections_reset_subsequent_author_substitution_at_each_boundary() {
        announce_behavior(
            "Subsequent-author substitution in a sort-partitioned document bibliography must reset at each section boundary rather than carrying over from the previous section.",
        );
        super::given_sort_partitioned_sections_and_subsequent_author_substitution_when_rendering_a_document_bibliography_then_substitution_resets_at_each_section_boundary();
    }

    #[test]
    fn groups_enabled_false_disables_groups_configuration_and_renders_flat() {
        announce_behavior(
            "groups_enabled: false must disable groups: configuration entirely and render a flat document bibliography, even when groups: is still present.",
        );
        super::given_groups_enabled_false_with_groups_still_configured_when_rendering_a_document_bibliography_then_it_renders_flat();
    }
}

mod body_markup_terminal_formats {
    use super::announce_behavior;

    #[test]
    fn markdown_block_quote_renders_as_typst_quote_block() {
        announce_behavior(
            "A Markdown block quote rendered to Typst should produce #quote(block: true) with inline emphasis correctly mapped — not raw '>' syntax (fixes #824).",
        );
        super::given_markdown_block_quote_when_rendered_as_typst_then_quote_block_is_emitted();
    }

    #[test]
    fn djot_block_quote_renders_as_typst_quote_block() {
        announce_behavior(
            "A Djot block quote rendered to Typst should produce #quote(block: true) with correctly mapped inline markup.",
        );
        super::given_djot_block_quote_when_rendered_as_typst_then_quote_block_is_emitted();
    }

    #[test]
    fn djot_notice_nested_strong_renders_as_composable_typst() {
        announce_behavior(
            "A Djot notice paragraph using nested strong markup (`**...**`) outside footnotes should render as composable Typst strong markup while preserving citation placeholder replacement.",
        );
        super::given_djot_notice_with_nested_strong_when_rendered_as_typst_then_strong_markup_is_composable();
    }

    #[test]
    fn djot_passthrough_preserves_notice_source_markup() {
        announce_behavior(
            "Djot passthrough output should preserve source body markup while still replacing citation markers.",
        );
        super::given_djot_notice_with_nested_strong_when_rendered_as_djot_then_source_markup_passes_through();
    }

    #[test]
    fn markdown_block_quote_renders_as_latex_quote_environment() {
        announce_behavior(
            "A Markdown block quote rendered to LaTeX should produce a \\begin{quote} environment with \\emph and \\textbf for inline markup.",
        );
        super::given_markdown_block_quote_when_rendered_as_latex_then_quote_environment_is_emitted(
        );
    }

    #[test]
    fn markdown_citation_in_prose_renders_alongside_converted_markup() {
        announce_behavior(
            "A Markdown paragraph with a citation and inline emphasis rendered to Typst should produce both a converted citation and #emph[…] markup.",
        );
        super::given_markdown_citation_inside_prose_when_rendered_as_typst_then_citation_and_markup_both_appear();
    }
}
