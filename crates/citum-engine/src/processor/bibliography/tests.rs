/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Parallel-vs-sequential equality tests for bibliography rendering.
//!
//! Only meaningful under the `parallel` feature: without it there is no
//! parallel path (`Processor::render_numbered_refs_parallel`) to compare
//! against, so this whole module is feature-gated.

#![cfg(feature = "parallel")]

use super::*;
use crate::reference::Bibliography;
use crate::render::plain::PlainText;
use citum_schema::options::{
    AndOptions, BibliographyOptions, Config, ContributorConfig, Processing,
};
use citum_schema::reference::{
    Contributor, DateValue, Monograph, MonographType, MultilingualString, StructuredName, Title,
};
use citum_schema::template::{
    ContributorForm, ContributorRole, Rendering, TemplateComponent, TemplateContributor,
    TemplateTitle, TitleType,
};
use citum_schema::{BibliographySpec, Style, StyleInfo};

/// A minimal monograph fixture: one author, a per-id title, and an issued year.
fn make_ref(id: &str, family: &str, given: &str, year: i32) -> Reference {
    Reference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(format!("Title {id}"))),
        author: Some(Contributor::StructuredName(StructuredName {
            family: MultilingualString::Simple(family.to_string()),
            given: MultilingualString::Simple(given.to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        })),
        issued: DateValue::new(year.to_string()),
        ..Default::default()
    }))
}

/// Build a bibliography of `n` references in shared-author runs of three, so
/// that once sorted by author, subsequent-author substitution triggers
/// repeatedly (author N's 2nd and 3rd entries substitute; the 4th entry,
/// with a new author, does not).
fn make_bibliography(n: usize) -> Bibliography {
    let mut bib = Bibliography::new();
    for i in 0..n {
        let run = i / 3;
        let family = format!("Author{run:03}");
        let id = format!("ref{i:03}");
        bib.insert(
            id.clone(),
            make_ref(
                &id,
                &family,
                "Test",
                2000 + i32::try_from(i % 20).unwrap_or(0),
            ),
        );
    }
    bib
}

/// An author-date style with subsequent-author substitution enabled: author
/// (long form) followed by the title.
fn author_date_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Parallel Bibliography Equality Test".to_string()),
            id: Some("parallel-bibliography-equality-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            contributors: Some(ContributorConfig {
                and: Some(AndOptions::Text),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                subsequent_author_substitute: Some("———".to_string()),
                ..Default::default()
            }),
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author.into(),
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    rendering: Rendering {
                        prefix: Some(", ".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// A numeric style built from [`author_date_style`], declaring a numeric
/// reference marker so `begin_run` pre-assigns `citation_numbers`
/// (`setup.rs::initialize_numeric_bibliography_numbers`) instead of the
/// lazy, render-time fallback author-date styles use.
fn numeric_style() -> Style {
    let mut style = author_date_style();
    style.options.as_mut().unwrap().processing = Some(Processing::Numeric);
    let Some(bibliography) = style.bibliography.as_mut() else {
        unreachable!("author_date_style always sets bibliography");
    };
    let options = bibliography.options.get_or_insert_with(Default::default);
    options.label_mode = Some(citum_schema::options::BibliographyLabelMode::Numeric);
    options.label_wrap = Some(citum_schema::options::BibliographyLabelWrap::Brackets);
    options.label_separator = Some(" ".to_string());
    bibliography.template = Some(vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            ..Default::default()
        }),
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            rendering: Rendering {
                prefix: Some(", ".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]);
    style
}

#[test]
fn given_large_author_date_bibliography_when_rendered_sequential_or_parallel_then_entries_match() {
    // 48 references, well above PARALLEL_MIN_ENTRIES (32), in runs of three
    // shared authors so subsequent-author substitution is exercised.
    let processor = Processor::new(author_date_style(), make_bibliography(48));
    let run = processor.begin_run().finalize();

    let sorted_refs = processor.sort_references(processor.bibliography.values().collect());
    let numbered_refs = number_sorted_refs(sorted_refs.into_iter(), &run);

    let ctx = processor.flat_render_context(&run);
    let sequential_rendered =
        processor.render_numbered_refs_sequential::<PlainText>(&numbered_refs, &ctx);
    let parallel_rendered =
        processor.render_numbered_refs_parallel::<PlainText>(&numbered_refs, &ctx);

    let substitute = ctx
        .bibliography_config
        .subsequent_author_substitute
        .as_ref();

    let sequential_entries =
        processor.apply_substitution_post_pass::<PlainText>(sequential_rendered, substitute, &ctx);
    let parallel_entries =
        processor.apply_substitution_post_pass::<PlainText>(parallel_rendered, substitute, &ctx);

    assert_eq!(sequential_entries.len(), 48);
    assert_eq!(sequential_entries, parallel_entries);

    // Sanity check: substitution actually triggered somewhere. Without this,
    // the equality assertion above could pass trivially on a bibliography
    // where the post-pass's substitution branch never runs.
    let substituted_count = sequential_entries
        .iter()
        .filter(|entry| {
            entry
                .template
                .iter()
                .any(|component| component.value == "———")
        })
        .count();
    assert!(
        substituted_count > 0,
        "expected subsequent-author substitution to trigger at least once \
         across a 48-entry bibliography with three-entry author runs"
    );
}

#[test]
fn given_numeric_bibliography_when_rendered_sequential_or_parallel_then_entries_match() {
    // Numeric styles pre-assign citation_numbers at begin_run (see
    // `setup.rs::initialize_numeric_bibliography_numbers`), which is the
    // determinism precondition parallel bibliography rendering relies on —
    // exercise that path too, not just author-date's lazy fallback numbering.
    let processor = Processor::new(numeric_style(), make_bibliography(40));
    let run = processor.begin_run().finalize();

    let sorted_refs = processor.sort_references(processor.bibliography.values().collect());
    let numbered_refs = number_sorted_refs(sorted_refs.into_iter(), &run);

    let ctx = processor.flat_render_context(&run);
    let sequential_rendered =
        processor.render_numbered_refs_sequential::<PlainText>(&numbered_refs, &ctx);
    let parallel_rendered =
        processor.render_numbered_refs_parallel::<PlainText>(&numbered_refs, &ctx);

    let substitute = ctx
        .bibliography_config
        .subsequent_author_substitute
        .as_ref();

    let sequential_entries =
        processor.apply_substitution_post_pass::<PlainText>(sequential_rendered, substitute, &ctx);
    let parallel_entries =
        processor.apply_substitution_post_pass::<PlainText>(parallel_rendered, substitute, &ctx);

    assert_eq!(sequential_entries.len(), 40);
    assert_eq!(sequential_entries, parallel_entries);
}

#[test]
fn given_large_bibliography_group_when_rendered_sequential_or_parallel_then_entries_match() {
    // Exercises the grouped-bibliography render path, which builds a fresh
    // `Renderer` per task in the parallel branch instead of sharing one
    // `Renderer` across threads (see `EntryRenderContext`'s docs).
    let processor = Processor::new(author_date_style(), make_bibliography(48));
    let run = processor.begin_run().finalize();

    let sorted_refs = processor.sort_references(processor.bibliography.values().collect());
    let numbered_refs = number_sorted_refs(sorted_refs.into_iter(), &run);

    let ctx = EntryRenderContext {
        style: &processor.style,
        hints: &processor.hints,
        config: Arc::new(processor.get_bibliography_config().into_owned()),
        bibliography_config: Arc::new(processor.get_bibliography_options().into_owned()),
        run: &run,
    };

    let sequential_rendered =
        processor.render_numbered_refs_sequential::<PlainText>(&numbered_refs, &ctx);
    let parallel_rendered =
        processor.render_numbered_refs_parallel::<PlainText>(&numbered_refs, &ctx);

    let substitute = ctx
        .bibliography_config
        .subsequent_author_substitute
        .as_ref();
    let sequential_entries =
        processor.apply_substitution_post_pass::<PlainText>(sequential_rendered, substitute, &ctx);
    let parallel_entries =
        processor.apply_substitution_post_pass::<PlainText>(parallel_rendered, substitute, &ctx);

    assert_eq!(sequential_entries.len(), 48);
    assert_eq!(sequential_entries, parallel_entries);
}

#[test]
fn given_bibliography_size_when_dispatching_then_selected_path_matches_sequential_output() {
    for size in [8, 48] {
        let processor = Processor::new(author_date_style(), make_bibliography(size));
        let run = processor.begin_run().finalize();
        let sorted_refs = processor.sort_references(processor.bibliography.values().collect());
        let numbered_refs = number_sorted_refs(sorted_refs.into_iter(), &run);
        let ctx = processor.flat_render_context(&run);

        let dispatched = processor.render_numbered_refs::<PlainText>(&numbered_refs, &ctx);
        let sequential =
            processor.render_numbered_refs_sequential::<PlainText>(&numbered_refs, &ctx);

        assert_eq!(
            dispatched, sequential,
            "dispatch changed output for {size} entries"
        );

        let expected = processor.apply_substitution_post_pass::<PlainText>(
            sequential,
            ctx.bibliography_config
                .subsequent_author_substitute
                .as_ref(),
            &ctx,
        );
        let actual = processor
            .process_references_with_format::<PlainText>(&run)
            .bibliography;
        assert_eq!(
            actual, expected,
            "public rendering changed output for {size} entries"
        );
    }
}
