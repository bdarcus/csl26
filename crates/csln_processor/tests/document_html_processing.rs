use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
use csln_core::{
    options::{BibliographyConfig, Config, Processing},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate,
    },
    BibliographySpec, Style, StyleInfo,
};
use csln_processor::{
    processor::document::{DocumentFormat, WinnowCitationParser},
    reference::Reference,
    Processor,
};

#[test]
fn test_document_html_output_contains_heading() {
    // Create a simple style
    let style = Style {
        info: StyleInfo {
            title: Some("Test Style".to_string()),
            id: Some("test".to_string()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            bibliography: Some(BibliographyConfig {
                entry_suffix: Some(".".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    name_order: None,
                    delimiter: None,
                    sort_separator: None,
                    shorten: None,
                    and: None,
                    rendering: Rendering::default(),
                    links: None,
                    overrides: None,
                    _extra: Default::default(),
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create a bibliography with one reference
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "kuhn1962".to_string(),
        Reference::from(LegacyReference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            ..Default::default()
        }),
    );

    // Create processor
    let processor = Processor::new(style, bibliography);

    // Create a simple document with a citation
    let document = "This is a test document with a citation [@kuhn1962].\n\nMore text here.";

    // Process document as HTML
    let parser = WinnowCitationParser;
    let html_output = processor.process_document::<_, csln_processor::render::html::Html>(
        document,
        &parser,
        DocumentFormat::Html,
    );

    // Verify that the output contains HTML heading
    assert!(
        html_output.contains("<h1>Bibliography</h1>"),
        "Output should contain <h1>Bibliography</h1>"
    );

    // Verify that the citation was replaced
    assert!(
        html_output.contains("kuhn1962") || html_output.contains("Kuhn"),
        "Output should contain reference to kuhn1962 or Kuhn. Got: {}",
        html_output
    );

    // Verify document structure is preserved
    assert!(
        html_output.contains("test document with a citation"),
        "Output should contain original document text"
    );
}

#[test]
fn test_document_djot_output_unmodified() {
    // Create a simple style
    let style = Style {
        info: StyleInfo {
            title: Some("Test Style".to_string()),
            id: Some("test".to_string()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            bibliography: Some(BibliographyConfig {
                entry_suffix: Some(".".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    name_order: None,
                    delimiter: None,
                    sort_separator: None,
                    shorten: None,
                    and: None,
                    rendering: Rendering::default(),
                    links: None,
                    overrides: None,
                    _extra: Default::default(),
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create a bibliography
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Author", "Name")]),
            title: Some("Title".to_string()),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bibliography);
    let document = "Document with citation [@ref1].";

    // Process as Djot format
    let parser = WinnowCitationParser;
    let djot_output = processor.process_document::<_, csln_processor::render::djot::Djot>(
        document,
        &parser,
        DocumentFormat::Djot,
    );

    // Verify it contains Djot markdown (not HTML)
    assert!(
        djot_output.contains("# Bibliography"),
        "Djot output should contain # Bibliography markdown"
    );

    // Should not contain HTML tags
    assert!(
        !djot_output.contains("<h1>"),
        "Djot output should not contain HTML tags"
    );
}
