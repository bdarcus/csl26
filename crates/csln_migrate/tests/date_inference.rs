use csl_legacy::model::{Citation, CslNode, Date, DatePart, Formatting, Info, Layout, Style};
use csln_core::options::MonthFormat;
use csln_migrate::options_extractor::OptionsExtractor;

fn make_style_with_date(form: Option<String>) -> Style {
    let date_part = DatePart {
        name: "month".to_string(),
        form,
        prefix: None,
        suffix: None,
    };

    let date_node = CslNode::Date(Date {
        variable: "issued".to_string(),
        form: None,
        prefix: None,
        suffix: None,
        delimiter: None,
        date_parts: None,
        text_case: None,
        parts: vec![date_part],
        macro_call_order: None,
        formatting: Formatting::default(),
    });

    Style {
        version: "1.0".to_string(),
        xmlns: "http://purl.org/net/xbiblio/csl".to_string(),
        class: "in-text".to_string(),
        default_locale: None,
        initialize_with: None,
        initialize_with_hyphen: None,
        names_delimiter: None,
        name_as_sort_order: None,
        sort_separator: None,
        delimiter_precedes_last: None,
        delimiter_precedes_et_al: None,
        demote_non_dropping_particle: None,
        and: None,
        page_range_format: None,
        info: Info::default(),
        locale: vec![],
        macros: vec![],
        citation: Citation {
            layout: Layout {
                prefix: None,
                suffix: None,
                delimiter: None,
                children: vec![date_node],
            },
            sort: None,
            et_al_min: None,
            et_al_use_first: None,
            disambiguate_add_year_suffix: None,
            disambiguate_add_names: None,
            disambiguate_add_givenname: None,
        },
        bibliography: None,
    }
}

#[test]
fn test_infer_short_month() {
    let style = make_style_with_date(Some("short".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Short);
}

#[test]
fn test_infer_long_month() {
    let style = make_style_with_date(Some("long".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Long);
}

#[test]
fn test_infer_numeric_month() {
    let style = make_style_with_date(Some("numeric".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Numeric);
}

#[test]
fn test_infer_default_month() {
    let style = make_style_with_date(None); // Default is usually long
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Long);
}
