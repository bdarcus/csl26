use super::*;
use crate::reference::Reference;
use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
use csln_core::locale::{GeneralTerm, Locale, TermForm};
use csln_core::options::*;
use csln_core::reference::FlatName;
use csln_core::template::DateVariable as TemplateDateVar;
use csln_core::template::*;

fn make_config() -> Config {
    Config {
        processing: Some(csln_core::options::Processing::AuthorDate),
        contributors: Some(ContributorConfig {
            shorten: Some(ShortenListOptions {
                min: 3,
                use_first: 1,
                ..Default::default()
            }),
            and: Some(AndOptions::Symbol),
            display_as_sort: Some(DisplayAsSort::First),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_locale() -> Locale {
    Locale::en_us()
}

fn make_reference() -> Reference {
    Reference::from(LegacyReference {
        id: "kuhn1962".to_string(),
        ref_type: "book".to_string(),
        author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
        title: Some("The Structure of Scientific Revolutions".to_string()),
        issued: Some(DateVariable::year(1962)),
        publisher: Some("University of Chicago Press".to_string()),
        ..Default::default()
    })
}

#[test]
fn test_contributor_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        overrides: None,
        _extra: Default::default(),
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "Kuhn");
}

#[test]
fn test_date_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        rendering: Default::default(),
        overrides: None,
        _extra: Default::default(),
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "1962");
}

#[test]
fn test_et_al() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("LeCun", "Yann"),
            Name::new("Bengio", "Yoshua"),
            Name::new("Hinton", "Geoffrey"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        overrides: None,
        _extra: Default::default(),
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "LeCun et al.");
}

#[test]
fn test_format_page_range_expanded() {
    use csln_core::options::PageRangeFormat;
    assert_eq!(
        number::format_page_range("321-328", Some(&PageRangeFormat::Expanded)),
        "321–328"
    );
    assert_eq!(
        number::format_page_range("42-45", Some(&PageRangeFormat::Expanded)),
        "42–45"
    );
}

#[test]
fn test_format_page_range_minimal() {
    use csln_core::options::PageRangeFormat;
    // minimal: keep only differing digits
    assert_eq!(
        number::format_page_range("321-328", Some(&PageRangeFormat::Minimal)),
        "321–8"
    );
    assert_eq!(
        number::format_page_range("42-45", Some(&PageRangeFormat::Minimal)),
        "42–5"
    );
    assert_eq!(
        number::format_page_range("12-17", Some(&PageRangeFormat::Minimal)),
        "12–7"
    );
}

#[test]
fn test_format_page_range_minimal_two() {
    use csln_core::options::PageRangeFormat;
    // minimal-two: at least 2 digits
    assert_eq!(
        number::format_page_range("321-328", Some(&PageRangeFormat::MinimalTwo)),
        "321–28"
    );
    assert_eq!(
        number::format_page_range("42-45", Some(&PageRangeFormat::MinimalTwo)),
        "42–45"
    );
}

#[test]
fn test_format_page_range_chicago() {
    use csln_core::options::PageRangeFormat;
    // Chicago: special rules for under 100 and same hundreds
    assert_eq!(
        number::format_page_range("71-72", Some(&PageRangeFormat::Chicago)),
        "71–72"
    );
    assert_eq!(
        number::format_page_range("321-328", Some(&PageRangeFormat::Chicago)),
        "321–28"
    );
    assert_eq!(
        number::format_page_range("1536-1538", Some(&PageRangeFormat::Chicago)),
        "1536–38"
    );
}

#[test]
fn test_format_page_range_no_format() {
    // No format specified: just convert hyphen to en-dash
    assert_eq!(number::format_page_range("321-328", None), "321–328");
}

#[test]
fn test_et_al_delimiter_never() {
    use csln_core::options::DelimiterPrecedesLast;

    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 2,
            use_first: 1,
            ..Default::default()
        });
        contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Never);
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        overrides: None,
        _extra: Default::default(),
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // With "never", no comma before et al.
    assert_eq!(values.value, "Smith et al.");
}

#[test]
fn test_et_al_delimiter_always() {
    use csln_core::options::DelimiterPrecedesLast;

    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 2,
            use_first: 1,
            ..Default::default()
        });
        contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Always);
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        overrides: None,
        _extra: Default::default(),
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // With "always", comma before et al.
    assert_eq!(values.value, "Smith, et al.");
}

#[test]
fn test_demote_non_dropping_particle() {
    use csln_core::options::DemoteNonDroppingParticle;

    // Name: Ludwig van Beethoven
    let name = FlatName {
        family: Some("Beethoven".to_string()),
        given: Some("Ludwig".to_string()),
        non_dropping_particle: Some("van".to_string()),
        ..Default::default()
    };

    // Case 1: Never demote (default CSL behavior for display)
    // Inverted: "van Beethoven, Ludwig"
    let res_never = contributor::format_single_name(
        &name,
        &ContributorForm::Long,
        0,
        &Some(DisplayAsSort::All), // Force inverted
        None,
        None,
        None, // initialize_with_hyphen
        Some(&DemoteNonDroppingParticle::Never),
        None, // sort_separator
        false,
    );
    assert_eq!(res_never, "van Beethoven, Ludwig");

    // Case 2: Display-and-sort (demote)
    // Inverted: "Beethoven, Ludwig van"
    let res_demote = contributor::format_single_name(
        &name,
        &ContributorForm::Long,
        0,
        &Some(DisplayAsSort::All), // Force inverted
        None,
        None,
        None, // initialize_with_hyphen
        Some(&DemoteNonDroppingParticle::DisplayAndSort),
        None, // sort_separator
        false,
    );
    assert_eq!(res_demote, "Beethoven, Ludwig van");

    // Case 3: Sort-only (same as Never for display)
    // Inverted: "van Beethoven, Ludwig"
    let res_sort_only = contributor::format_single_name(
        &name,
        &ContributorForm::Long,
        0,
        &Some(DisplayAsSort::All), // Force inverted
        None,
        None,
        None, // initialize_with_hyphen
        Some(&DemoteNonDroppingParticle::SortOnly),
        None, // sort_separator
        false,
    );
    assert_eq!(res_sort_only, "van Beethoven, Ludwig");

    // Case 4: Not inverted (should be same for all)
    // "Ludwig van Beethoven"
    let res_straight = contributor::format_single_name(
        &name,
        &ContributorForm::Long,
        0,
        &Some(DisplayAsSort::None), // Not inverted
        None,
        None,
        None, // initialize_with_hyphen
        Some(&DemoteNonDroppingParticle::DisplayAndSort),
        None, // sort_separator
        false,
    );
    assert_eq!(res_straight, "Ludwig van Beethoven");
}

#[test]
fn test_template_list_suppression() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateList {
        items: vec![
            TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Doi,
                ..Default::default()
            }),
            TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Url,
                ..Default::default()
            }),
        ],
        delimiter: Some(DelimiterPunctuation::Comma),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options);
    assert!(values.is_none());
}

#[test]
fn test_et_al_use_last() {
    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 3,
            use_first: 1,
            use_last: Some(1),
            ..Default::default()
        });
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("LeCun", "Yann"),
            Name::new("Bengio", "Yoshua"),
            Name::new("Hinton", "Geoffrey"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // first name (LeCun) + ellipsis + last name (Hinton)
    assert_eq!(values.value, "LeCun … Hinton");
}

#[test]
fn test_et_al_use_last_overlap() {
    // Edge case: use_first + use_last >= names.len() should show all names
    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 3,
            use_first: 2,
            use_last: Some(2),
            ..Default::default()
        });
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "overlap".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("Alpha", "A."),
            Name::new("Beta", "B."),
            Name::new("Gamma", "C."),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // use_first(2) + use_last(2) = 4 >= 3 names, so show first 2 + ellipsis + last 1
    // Alpha & Beta … Gamma (skip=max(2, 3-2)=2, so last 1 name)
    assert_eq!(values.value, "Alpha & Beta … Gamma");
}

#[test]
fn test_title_hyperlink() {
    use csln_core::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "kuhn1962".to_string(),
        title: Some("The Structure of Scientific Revolutions".to_string()),
        doi: Some("10.1001/example".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        links: Some(LinksConfig {
            doi: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(
        values.url,
        Some("https://doi.org/10.1001/example".to_string())
    );
}

#[test]
fn test_title_hyperlink_url_fallback() {
    use csln_core::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Citation,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    // Reference with URL but no DOI
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        title: Some("A Web Resource".to_string()),
        url: Some("https://example.com/resource".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        links: Some(LinksConfig {
            doi: Some(true),
            url: Some(true),
        }),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // Falls back to URL when DOI is absent
    assert_eq!(values.url, Some("https://example.com/resource".to_string()));
}

#[test]
fn test_variable_hyperlink() {
    use csln_core::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "pub2024".to_string(),
        publisher: Some("MIT Press".to_string()),
        doi: Some("10.1234/pub".to_string()),
        ..Default::default()
    });

    let component = TemplateVariable {
        variable: SimpleVariable::Publisher,
        links: Some(LinksConfig {
            doi: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "MIT Press");
    assert_eq!(values.url, Some("https://doi.org/10.1234/pub".to_string()));
}

#[test]
fn test_editor_label_format() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "editor-test".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Doe", "John")]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Editor,
        form: ContributorForm::Long,
        ..Default::default()
    };

    // Test VerbPrefix
    if let Some(ref mut contributors) = config.contributors {
        contributors.editor_label_format = Some(EditorLabelFormat::VerbPrefix);
    }
    {
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
        };
        let values = component.values(&reference, &hints, &options).unwrap();
        // Assuming locale for "editor" verb is "edited by"
        assert_eq!(values.prefix, Some("edited by ".to_string()));
    }

    // Test ShortSuffix
    if let Some(ref mut contributors) = config.contributors {
        contributors.editor_label_format = Some(EditorLabelFormat::ShortSuffix);
    }
    {
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
        };
        let values = component.values(&reference, &hints, &options).unwrap();
        // Assuming locale for "editor" short is "Ed."
        assert_eq!(values.suffix, Some(" (Ed.)".to_string()));
    }

    // Test LongSuffix
    if let Some(ref mut contributors) = config.contributors {
        contributors.editor_label_format = Some(EditorLabelFormat::LongSuffix);
    }
    {
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
        };
        let values = component.values(&reference, &hints, &options).unwrap();
        // Assuming locale for "editor" long is "editor"
        assert_eq!(values.suffix, Some(", editor".to_string()));
    }
}

#[test]
fn test_term_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateTerm {
        term: GeneralTerm::In,
        form: Some(TermForm::Long),
        overrides: None,
        _extra: Default::default(),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "in");
}

#[test]
fn test_template_list_term_suppression() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
    };
    // Reference with no editor
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateList {
        items: vec![
            TemplateComponent::Term(TemplateTerm {
                term: GeneralTerm::In,
                overrides: None,
                _extra: Default::default(),
                ..Default::default()
            }),
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Editor,
                ..Default::default()
            }),
        ],
        delimiter: Some(DelimiterPunctuation::Space),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options);
    // Should be None because only the term "In" would render, and it's suppressed if no content-bearing items are present
    assert!(values.is_none());
}
