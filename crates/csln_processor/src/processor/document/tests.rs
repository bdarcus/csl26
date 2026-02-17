use crate::processor::document::{djot::DjotParser, DocumentFormat};
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
                        prefix: Some(" ".to_string()),
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

    let content = "Visible citation: [@item1]. Silent: [!@item2]";
    let result =
        processor.process_document::<_, PlainText>(content, &parser, DocumentFormat::Plain);

    // Check text output
    assert!(result.contains("Visible citation: (Doe,  2020)"));
    assert!(result.contains("Silent: "));

    // Smith should be hidden in the main body text (before the bibliography)
    let body_text = &result[..result.find("# Bibliography").unwrap()];
    assert!(!body_text.contains("Smith"));

    // Check bibliography grouping
    assert!(result.contains("# Bibliography"));
    assert!(result.contains("John Doe (2020)"));

    assert!(result.contains("# Additional Reading"));
    assert!(result.contains("Jane Smith (2010)"));
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
                        prefix: Some(" ".to_string()),
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

    // Item 2 is cited both visibly and silently
    let content = "Visible: [@item2]. Silent: [!@item2]";
    let result =
        processor.process_document::<_, PlainText>(content, &parser, DocumentFormat::Plain);

    // Smith should be in text as (Smith,  2010)
    assert!(result.contains("Visible: (Smith,  2010)"));

    // Smith should be in the main bibliography
    assert!(result.contains("# Bibliography"));
    assert!(result.contains("Jane Smith (2010)"));

    // Additional Reading should be empty/absent
    assert!(!result.contains("# Additional Reading"));
}
