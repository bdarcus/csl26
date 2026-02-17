use crate::processor::document::{djot::DjotParser, CitationParser, DocumentFormat};
use crate::processor::Processor;
use crate::reference::{Bibliography, Reference};
use crate::render::plain::PlainText;
use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
use csln_core::Style;

fn make_test_bib() -> Bibliography {
    let mut bib = Bibliography::new();
    bib.insert(
        "item1".to_string(),
        Reference::from(LegacyReference {
            id: "item1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Doe", "John")]),
            title: Some("Book One".to_string()),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "item2".to_string(),
        Reference::from(LegacyReference {
            id: "item2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "Jane")]),
            title: Some("Book Two".to_string()),
            issued: Some(DateVariable::year(2010)),
            ..Default::default()
        }),
    );
    bib
}

#[test]
fn test_bibliography_grouping() {
    use csln_core::{
        template::{
            ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
            TemplateContributor, TemplateDate, WrapPunctuation,
        },
        BibliographySpec, CitationSpec,
    };
    let style = Style {
        citation: Some(CitationSpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            wrap: Some(WrapPunctuation::Parentheses),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        prefix: Some(" (".to_string()),
                        suffix: Some(")".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib = make_test_bib();
    let processor = Processor::new(style, bib);
    let parser = DjotParser;

    let content = "Visible citation: [@item1].";
    let result =
        processor.process_document::<_, PlainText>(content, &parser, DocumentFormat::Plain);

    // Check text output
    assert!(result.contains("Visible citation: (Doe, 2020)"));

    // Check bibliography
    assert!(result.contains("# Bibliography"));
    assert!(result.contains("John Doe (2020)"));
}

#[test]
fn test_visible_wins_over_silent() {
    use csln_core::{
        template::{
            ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
            TemplateContributor, TemplateDate, WrapPunctuation,
        },
        BibliographySpec, CitationSpec,
    };
    let style = Style {
        citation: Some(CitationSpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            wrap: Some(WrapPunctuation::Parentheses),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        prefix: Some(" (".to_string()),
                        suffix: Some(")".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib = make_test_bib();
    let processor = Processor::new(style, bib);
    let parser = DjotParser;

    // Item 2 is cited visibly
    let content = "Visible: [@item2].";
    let result =
        processor.process_document::<_, PlainText>(content, &parser, DocumentFormat::Plain);

    // Smith should be in text as (Smith, 2010)
    assert!(result.contains("Visible: (Smith, 2010)"));

    // Smith should be in the main bibliography
    assert!(result.contains("# Bibliography"));
    assert!(result.contains("Jane Smith (2010)"));

    // Additional Reading should be empty/absent
    assert!(!result.contains("# Additional Reading"));
}

#[test]
fn test_repro_djot_parsing() {
    use csln_core::citation::{CitationMode, ItemVisibility};
    let parser = DjotParser;

    // Bracketed citations (currently supported)
    let content = "Test [+@item1] and [-@item2]";
    let citations = parser.parse_citations(content);
    assert_eq!(citations.len(), 2);

    assert_eq!(citations[0].2.mode, CitationMode::Integral);
    assert_eq!(
        citations[1].2.items[0].visibility,
        ItemVisibility::SuppressAuthor
    );

    // Non-bracketed citations (SHOULD NOT be supported)
    let content2 = "Test @item1 and +@item2 and -@item3 and !@item4";
    let citations2 = parser.parse_citations(content2);
    assert_eq!(
        citations2.len(),
        0,
        "Should NOT support non-bracketed citations"
    );
}

#[test]
fn test_repro_djot_rendering() {
    use csln_core::{
        template::{
            ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
            TemplateContributor, TemplateDate, TemplateList, WrapPunctuation,
        },
        CitationSpec,
    };
    let style = Style {
        citation: Some(CitationSpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    ..Default::default()
                }),
            ]),
            delimiter: Some(", ".to_string()),
            wrap: Some(WrapPunctuation::Parentheses),
            integral: Some(Box::new(csln_core::CitationSpec {
                template: Some(vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author,
                        form: ContributorForm::Short,
                        ..Default::default()
                    }),
                    TemplateComponent::List(TemplateList {
                        items: vec![TemplateComponent::Date(TemplateDate {
                            date: DateVariable::Issued,
                            form: DateForm::Year,
                            ..Default::default()
                        })],
                        rendering: Rendering {
                            wrap: Some(WrapPunctuation::Parentheses),
                            ..Default::default()
                        },
                        delimiter: None,
                        overrides: None,
                        custom: None,
                    }),
                ]),
                delimiter: Some(" ".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib = make_test_bib();
    let processor = Processor::new(style, bib);
    let parser = DjotParser;

    let content = "Integral: [+@item1]. SuppressAuthor: [-@item1].";
    let result =
        processor.process_document::<_, PlainText>(content, &parser, DocumentFormat::Plain);

    println!("RESULT: {}", result);
    assert!(result.contains("Integral: Doe (2020)"));
    assert!(result.contains("SuppressAuthor: (2020)"));
}
