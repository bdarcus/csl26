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

use citum_migrate::options_extractor::contributors::extract_contributor_config;
use citum_schema::options::NameForm;
use citum_schema::presets::ContributorPreset;
use csl_legacy::model::{Citation, Info, Layout, Style};

#[test]
fn test_style_global_initialize_with_co_emits_name_form_initials() {
    // Regression test for csl26-q4k2:
    // When a style-global initialize-with attribute is set (e.g., <style initialize-with=". ">),
    // the migrator should co-emit name_form: Initials to decouple the semantic meaning:
    // name-form controls how names are formatted, initialize-with only controls the separator.

    let style = Style {
        version: "1.0".to_string(),
        xmlns: "http://purl.org/net/xbiblio/csl".to_string(),
        class: "in-text".to_string(),
        default_locale: None,
        // Set style-global initialize-with
        initialize_with: Some(". ".to_string()),
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
                children: vec![],
            },
            localized_layouts: Vec::new(),
            sort: None,
            collapse: None,
            cite_group_delimiter: None,
            year_suffix_delimiter: None,
            et_al_min: None,
            et_al_use_first: None,
            disambiguate_add_year_suffix: None,
            disambiguate_add_names: None,
            disambiguate_add_givenname: None,
            disambiguate_givenname_rule: None,
        },
        bibliography: None,
    };

    let config = extract_contributor_config(&style);

    assert!(
        config.is_some(),
        "Extraction should succeed when initialize-with is present"
    );

    let config = config.unwrap();
    assert_eq!(
        config.initialize_with,
        Some(". ".to_string()),
        "initialize-with should be set"
    );
    assert_eq!(
        config.name_form,
        Some(NameForm::Initials),
        "name_form should be co-emitted as Initials when initialize-with is present"
    );
}

#[test]
fn test_apa_preset_config_includes_name_form_initials() {
    // Regression: apply_preset_extractions replaces options.contributors with
    // preset.config(), so the preset itself must include name_form: Initials
    // for styles whose initialize-with triggers preset detection.

    let config = ContributorPreset::Apa.config();
    assert_eq!(
        config.name_form,
        Some(NameForm::Initials),
        "APA preset config must include name_form: Initials"
    );
    assert!(
        config.initialize_with.is_some(),
        "APA preset config must include initialize_with"
    );
}
