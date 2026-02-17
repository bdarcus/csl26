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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Author,
        form: ContributorForm::Short,
        label: None,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: None,
        rendering: Default::default(),
        links: None,
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        label: None,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        label: None,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        label: None,
        name_order: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        links: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
        links: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
            target: Some(LinkTarget::Doi),
            anchor: Some(LinkAnchor::Title),
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
            target: Some(LinkTarget::UrlOrDoi),
            anchor: Some(LinkAnchor::Title),
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
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
            target: Some(LinkTarget::Doi),
            anchor: Some(LinkAnchor::Component),
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
        links: None,
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
            visibility: csln_core::citation::ItemVisibility::Default,
            locator: None,
            locator_label: None,
            infix: None,
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
            visibility: csln_core::citation::ItemVisibility::Default,
            locator: None,
            locator_label: None,
            infix: None,
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
            visibility: csln_core::citation::ItemVisibility::Default,
            locator: None,
            locator_label: None,
            infix: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateTerm {
        term: GeneralTerm::In,
        form: Some(TermForm::Long),
        overrides: None,
        custom: None,
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
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    // Reference with no editor
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateList {
        items: vec![
            TemplateComponent::Term(TemplateTerm {
                term: GeneralTerm::In,
                overrides: None,
                custom: None,
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

#[test]
fn test_date_fallback() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    // Reference with NO issued date
    let reference = Reference::from(LegacyReference {
        id: "no-date".to_string(),
        ref_type: "book".to_string(),
        author: Some(vec![Name::new("Aristotle", "Ancient")]),
        title: Some("Poetics".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: Some(vec![TemplateComponent::Term(TemplateTerm {
            term: GeneralTerm::NoDate,
            form: Some(TermForm::Short),
            ..Default::default()
        })]),
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    assert_eq!(values.value, "n.d.");
}

#[test]
fn test_strip_periods_global_config() {
    let mut config = make_config();
    config.strip_periods = Some(true);
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };

    let component = TemplateContributor {
        contributor: ContributorRole::Editor,
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // Should have "(Ed)" instead of "(Ed.)" due to strip_periods
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (Ed)");
}

#[test]
fn test_strip_periods_component_override() {
    let mut config = make_config();
    config.strip_periods = Some(false); // Global is false
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };

    // Component overrides global setting
    let component = TemplateContributor {
        contributor: ContributorRole::Editor,
        form: ContributorForm::Long,
        rendering: Rendering {
            strip_periods: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // Should strip periods because component overrides global
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (Ed)");
}

#[test]
fn test_strip_periods_no_strip_by_default() {
    let config = make_config();
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };

    let component = TemplateContributor {
        contributor: ContributorRole::Editor,
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component.values(&reference, &hints, &options).unwrap();
    // Should preserve periods by default
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (Ed.)");
}

#[test]
fn test_strip_trailing_periods() {
    assert_eq!(strip_trailing_periods("test."), "test");
    assert_eq!(strip_trailing_periods("test"), "test");
    assert_eq!(strip_trailing_periods("Ph.D."), "Ph.D");
    assert_eq!(strip_trailing_periods("A.B.C."), "A.B.C");
    assert_eq!(strip_trailing_periods("..."), "");
}

#[test]
fn test_should_strip_periods_precedence() {
    let config = Config {
        strip_periods: Some(true),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: &config,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };

    // Component override takes precedence
    let rendering_override_true = Rendering {
        strip_periods: Some(true),
        ..Default::default()
    };
    assert!(should_strip_periods(&rendering_override_true, &options));

    let rendering_override_false = Rendering {
        strip_periods: Some(false),
        ..Default::default()
    };
    assert!(!should_strip_periods(&rendering_override_false, &options));

    // Falls back to config when component has None
    let rendering_default = Rendering::default();
    assert!(should_strip_periods(&rendering_default, &options));

    // Defaults to false when both are None
    let config_none = Config::default();
    let options_none = RenderOptions {
        config: &config_none,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: csln_core::citation::CitationMode::NonIntegral,
        visibility: csln_core::citation::ItemVisibility::Default,
        locator: None,
        locator_label: None,
        infix: None,
    };
    assert!(!should_strip_periods(&rendering_default, &options_none));
}
