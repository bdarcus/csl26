use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
use csln_core::{
    options::{BibliographyConfig, Config, ContributorConfig, Processing},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate,
    },
    BibliographySpec, Style, StyleInfo,
};
use csln_processor::reference::Reference;
use csln_processor::Processor;

fn make_style_with_substitute(substitute: Option<String>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Subsequent Author Substitute Test".to_string()),
            id: Some("sub-test".to_string()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            bibliography: Some(BibliographyConfig {
                subsequent_author_substitute: substitute,
                entry_suffix: Some(".".to_string()),
                ..Default::default()
            }),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(csln_core::options::DisplayAsSort::First),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: None,
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
    }
}

#[test]
fn test_subsequent_author_substitute() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            issued: Some(DateVariable::year(2021)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // ref1 comes first (2020), then ref2 (2021). ref2 should have substituted author.
    // Note: Implicit separator ". " + Implicit suffix "."
    let expected = "Smith, John. 2020.\n\n———. 2021.";
    assert_eq!(result, expected);
}

#[test]
fn test_no_substitute_if_different() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Doe", "Jane")]),
            issued: Some(DateVariable::year(2021)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Doe comes before Smith alphabetically
    let expected = "Doe, Jane. 2021.\n\nSmith, John. 2020.";
    assert_eq!(result, expected);
}
