/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::*;
use citum_schema::grouping::SortKey as GroupSortKey;
use citum_schema::options::{
    ArticleJournalNoPageFallback, GivennameRule, Processing, SortKey, SubstituteConfig,
    SubstituteKey,
};
use citum_schema::presets::{SortPreset, SubstitutePreset};
use csl_legacy::parser::parse_style;
use roxmltree::Document;

fn parse_csl(xml: &str) -> Result<Style, String> {
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;
    parse_style(doc.root_element()).map_err(|e| e.clone())
}

#[test]
fn test_extract_author_date_processing() {
    let xml = r#"<style class="in-text"><citation><layout><text macro="year"/></layout></citation><bibliography><layout><text variable="title"/></layout></bibliography></style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDate));
}

#[test]
fn test_extract_author_date_processing_from_nested_macro() {
    let xml = r#"<style class="in-text">
        <macro name="issued-year">
            <date variable="issued"><date-part name="year"/></date>
        </macro>
        <macro name="author-date">
            <names variable="author"><name/></names>
            <text macro="issued-year" prefix=" "/>
        </macro>
        <citation><layout><text macro="author-date"/></layout></citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDate));
}

#[test]
fn test_extract_et_al_from_citation() {
    let xml = r#"<style class="in-text">
        <citation><layout>
            <names variable="author" et-al-min="3" et-al-use-first="1"><name/></names>
        </layout></citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let contributors = config.contributors.unwrap();
    let shorten = contributors.shorten.unwrap();
    assert_eq!(shorten.min, 3);
    assert_eq!(shorten.use_first, 1);
}

#[test]
fn test_extract_substitute_pattern() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography><layout>
            <names variable="author">
                <name/>
                <substitute>
                    <names variable="editor"/>
                    <text variable="title"/>
                </substitute>
            </names>
        </layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    if let Some(SubstituteConfig::Explicit(sub)) = config.substitute {
        assert_eq!(sub.template.len(), 2);
        assert_eq!(sub.template[0], SubstituteKey::Editor);
        assert_eq!(sub.template[1], SubstituteKey::Title);
    } else {
        panic!("Substitute pattern not extracted");
    }
}

#[test]
fn test_extract_migration_options_folds_standard_substitute_preset() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography><layout>
            <names variable="author">
                <name/>
                <substitute>
                    <names variable="editor"/>
                    <text variable="title"/>
                    <names variable="translator"/>
                </substitute>
            </names>
        </layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let extracted = OptionsExtractor::extract_migration_options(&style);

    assert_eq!(
        extracted.options.substitute,
        Some(SubstituteConfig::Preset(SubstitutePreset::Standard))
    );
}

#[test]
fn test_extract_migration_options_does_not_fold_substitute_with_overrides() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography><layout>
            <names variable="author">
                <name/>
                <substitute>
                    <choose>
                        <if type="classic">
                            <text variable="title"/>
                        </if>
                    </choose>
                    <names variable="editor"/>
                    <text variable="title"/>
                    <names variable="translator"/>
                </substitute>
            </names>
        </layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let extracted = OptionsExtractor::extract_migration_options(&style);

    assert!(matches!(
        extracted.options.substitute,
        Some(SubstituteConfig::Explicit(_))
    ));
}

#[test]
fn test_extract_processing_sort_and_disambiguation() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-year-suffix="false" disambiguate-add-names="true" disambiguate-add-givenname="true">
            <sort>
                <key macro="author"/>
                <key variable="issued"/>
                <key variable="title" sort="descending"/>
            </sort>
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let Processing::Custom(custom) = config.processing.unwrap() else {
        panic!("expected custom processing mode");
    };

    let disamb = custom.disambiguate.unwrap();
    assert!(!disamb.year_suffix);
    assert!(disamb.names);
    assert!(disamb.add_givenname);

    let sort = custom.sort.unwrap().resolve();
    assert_eq!(sort.template.len(), 3);
    assert_eq!(sort.template[0].key, SortKey::Author);
    assert_eq!(sort.template[1].key, SortKey::Year);
    assert_eq!(sort.template[2].key, SortKey::Title);
    assert!(sort.template[0].ascending);
    assert!(sort.template[1].ascending);
    assert!(!sort.template[2].ascending);

    let group = custom.group.unwrap();
    assert_eq!(
        group.template,
        vec![SortKey::Author, SortKey::Year, SortKey::Title]
    );
}

#[test]
fn test_extract_processing_sort_uses_author_date_title_preset_for_duplicate_macro_keys() {
    let xml = r#"<style class="in-text">
        <citation>
            <sort>
                <key macro="author-sort"/>
                <key macro="date-sort-group"/>
                <key macro="date-sort"/>
            </sort>
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    // Duplicate date macros deduplicate to [Author, Year], which maps to the
    // AuthorDateTitle preset — so the style folds to the named variant, not Custom.
    assert_eq!(config.processing, Some(Processing::AuthorDate));
    let config_custom = config.processing.unwrap().config();
    assert!(matches!(
        config_custom.sort,
        Some(citum_schema::options::SortEntry::Preset(
            SortPreset::AuthorDateTitle
        ))
    ));
}

#[test]
fn test_extract_processing_sort_keeps_conflicting_duplicate_direction_explicit() {
    let xml = r#"<style class="in-text">
        <citation>
            <sort>
                <key macro="author-sort"/>
                <key macro="date-sort-group"/>
                <key macro="date-sort" sort="descending"/>
            </sort>
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let Processing::Custom(custom) = config.processing.unwrap() else {
        panic!("expected custom processing mode");
    };
    let Some(citum_schema::options::SortEntry::Explicit(sort)) = custom.sort else {
        panic!("expected conflicting duplicate sort directions to stay explicit");
    };

    assert_eq!(sort.template.len(), 2);
    assert_eq!(sort.template[0].key, SortKey::Author);
    assert!(sort.template[0].ascending);
    assert_eq!(sort.template[1].key, SortKey::Year);
    assert!(!sort.template[1].ascending);
}

#[test]
fn test_extract_processing_disambiguation_defaults() {
    let xml = r#"<style class="in-text">
        <citation>
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDate));

    let disamb = config.processing.unwrap().config().disambiguate.unwrap();
    assert!(!disamb.names);
    assert!(!disamb.add_givenname);
    assert!(disamb.year_suffix);
}

#[test]
fn test_extract_processing_disambiguation_variant_givenname() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-givenname="true">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDateGivenname));
}

#[test]
fn test_extract_processing_disambiguation_variant_names() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-names="true">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDateNames));
}

/// names + givenname with no explicit `givenname-disambiguation-rule` still
/// folds to `author-date-full`, per the folding table in
/// `docs/reference/PROCESSING_MIGRATION.md`: the named preset is chosen from
/// the names/givenname signal alone, and its own canonical `primary-name`
/// rule applies without needing to be restated in the CSL.
#[test]
fn test_extract_full_disambiguation_default_rule_folds_to_full() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-names="true" disambiguate-add-givenname="true">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDateFull));
}

/// names + givenname + explicit `primary-name` matches the `author-date-full`
/// guide profile, so it folds to the named preset.
#[test]
fn test_extract_full_disambiguation_primary_name_folds_to_full() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-names="true" disambiguate-add-givenname="true" givenname-disambiguation-rule="primary-name">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert_eq!(config.processing, Some(Processing::AuthorDateFull));
}

#[test]
fn test_extract_processing_disambiguation_year_suffix_false_stays_custom() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-year-suffix="false">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let Processing::Custom(custom) = config.processing.unwrap() else {
        panic!("expected explicit year-suffix=false to stay custom");
    };
    let disamb = custom.disambiguate.unwrap();
    assert!(!disamb.names);
    assert!(!disamb.add_givenname);
    assert!(!disamb.year_suffix);
}

#[test]
fn test_extract_scoped_contributor_shorten_overrides() {
    let xml = r#"<style class="in-text">
        <citation et-al-min="3" et-al-use-first="1">
            <layout><names variable="author"><name/></names></layout>
        </citation>
        <bibliography et-al-min="6" et-al-use-first="3">
            <layout><names variable="author"><name/></names></layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let global_shorten = config
        .contributors
        .as_ref()
        .and_then(|c| c.shorten.as_ref())
        .expect("global contributor shorten should be extracted");
    assert_eq!(global_shorten.min, 6);
    assert_eq!(global_shorten.use_first, 3);

    let citation_scope = super::contributors::extract_citation_contributor_overrides(&style)
        .expect("citation scope overrides should be extracted");
    let citation_shorten = citation_scope.shorten.expect("citation shorten missing");
    assert_eq!(citation_shorten.min, 3);
    assert_eq!(citation_shorten.use_first, 1);

    let bibliography_scope =
        super::contributors::extract_bibliography_contributor_overrides(&style)
            .expect("bibliography scope overrides should be extracted");
    let bibliography_shorten = bibliography_scope
        .shorten
        .expect("bibliography shorten missing");
    assert_eq!(bibliography_shorten.min, 6);
    assert_eq!(bibliography_shorten.use_first, 3);
}

#[test]
fn test_extract_note_processing_mode() {
    let xml = r#"<style class="note">
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);
    assert!(matches!(config.processing, Some(Processing::Note)));
}

#[test]
fn test_extract_group_sort_from_bibliography_macros() {
    let xml = r#"<style class="in-text">
        <citation><layout><text variable="citation-number"/></layout></citation>
        <bibliography>
            <sort>
                <key macro="author"/>
                <key macro="title"/>
                <key variable="issued"/>
            </sort>
            <layout><text variable="title"/></layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let legacy_sort = style
        .bibliography
        .as_ref()
        .and_then(|b| b.sort.as_ref())
        .expect("legacy bibliography sort should exist");

    let sort = super::bibliography::extract_group_sort_from_bibliography(legacy_sort)
        .expect("group sort should be extracted");
    let sort = sort.resolve();
    assert_eq!(sort.template.len(), 3);
    assert!(matches!(sort.template[0].key, GroupSortKey::Author));
    assert!(matches!(sort.template[1].key, GroupSortKey::Title));
    assert!(matches!(sort.template[2].key, GroupSortKey::Issued));
}

#[test]
fn test_extract_group_sort_uses_author_date_title_preset_for_duplicate_macro_keys() {
    let xml = r#"<style class="in-text">
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <sort>
                <key macro="author-sort"/>
                <key macro="date-sort-group"/>
                <key macro="date-sort"/>
                <key macro="title"/>
                <key variable="event-date"/>
                <key variable="original-date"/>
            </sort>
            <layout><text variable="title"/></layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let legacy_sort = style
        .bibliography
        .as_ref()
        .and_then(|b| b.sort.as_ref())
        .expect("legacy bibliography sort should exist");

    let sort = super::bibliography::extract_group_sort_from_bibliography(legacy_sort)
        .expect("group sort should be extracted");

    assert!(matches!(
        sort,
        citum_schema::grouping::GroupSortEntry::Preset(SortPreset::AuthorDateTitle)
    ));
}

#[test]
fn test_extract_group_sort_keeps_conflicting_duplicate_direction_explicit() {
    let xml = r#"<style class="in-text">
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <sort>
                <key macro="author-sort"/>
                <key macro="date-sort-group"/>
                <key macro="date-sort" sort="descending"/>
                <key macro="title"/>
            </sort>
            <layout><text variable="title"/></layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let legacy_sort = style
        .bibliography
        .as_ref()
        .and_then(|b| b.sort.as_ref())
        .expect("legacy bibliography sort should exist");

    let sort = super::bibliography::extract_group_sort_from_bibliography(legacy_sort)
        .expect("group sort should be extracted");
    let citum_schema::grouping::GroupSortEntry::Explicit(sort) = sort else {
        panic!("expected conflicting duplicate sort directions to stay explicit");
    };

    assert_eq!(sort.template.len(), 3);
    assert_eq!(sort.template[0].key, GroupSortKey::Author);
    assert!(sort.template[0].ascending);
    assert_eq!(sort.template[1].key, GroupSortKey::Issued);
    assert!(!sort.template[1].ascending);
    assert_eq!(sort.template[2].key, GroupSortKey::Title);
    assert!(sort.template[2].ascending);
}

#[test]
fn test_extract_group_sort_ignores_citation_number_only() {
    let xml = r#"<style class="in-text">
        <citation><layout><text variable="citation-number"/></layout></citation>
        <bibliography>
            <sort>
                <key variable="citation-number"/>
            </sort>
            <layout><text variable="title"/></layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let legacy_sort = style
        .bibliography
        .as_ref()
        .and_then(|b| b.sort.as_ref())
        .expect("legacy bibliography sort should exist");

    let sort = super::bibliography::extract_group_sort_from_bibliography(legacy_sort);
    assert!(sort.is_none());
}

#[test]
fn test_extract_article_journal_no_page_doi_fallback() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <layout>
                <choose>
                    <if type="article-journal">
                        <group delimiter=", ">
                            <text variable="container-title"/>
                            <choose>
                                <if variable="page">
                                    <date variable="issued"><date-part name="year"/></date>
                                    <text variable="volume"/>
                                    <text variable="page"/>
                                </if>
                                <else>
                                    <text variable="DOI" prefix="DOI:"/>
                                </else>
                            </choose>
                        </group>
                    </if>
                </choose>
            </layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let bibliography = super::bibliography::extract_bibliography_config(&style);

    assert_eq!(
        bibliography
            .and_then(|bibliography| bibliography.article_journal)
            .and_then(|article_journal| article_journal.no_page_fallback),
        Some(ArticleJournalNoPageFallback::Doi)
    );
}

#[test]
fn test_extract_locator_strip_periods_from_citation_layout() {
    let xml = r#"<style class="in-text">
        <citation>
            <layout>
                <text variable="citation-number"/>
                <group prefix="(" suffix=")">
                    <label form="short" strip-periods="true" variable="locator"/>
                    <text variable="locator"/>
                </group>
            </layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let locators = config
        .locators
        .expect("locator config should be extracted from citation layout");
    assert_eq!(locators.strip_label_periods, Some(true));
}

#[test]
fn test_extract_article_journal_no_page_doi_fallback_ignores_additive_doi_patterns() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <layout>
                <choose>
                    <if type="article-journal">
                        <group delimiter=", ">
                            <text variable="container-title"/>
                            <date variable="issued"><date-part name="year"/></date>
                            <text variable="volume"/>
                            <choose>
                                <if variable="page">
                                    <text variable="page"/>
                                </if>
                            </choose>
                            <text variable="DOI" prefix="DOI:"/>
                        </group>
                    </if>
                </choose>
            </layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let bibliography = super::bibliography::extract_bibliography_config(&style);

    assert!(
        bibliography
            .and_then(|bibliography| bibliography.article_journal)
            .and_then(|article_journal| article_journal.no_page_fallback)
            .is_none()
    );
}

/// Given a CSL `<citation>` with `givenname-disambiguation-rule="primary-name"`,
/// when processing options are extracted,
/// then `Disambiguation.givenname_rule` is `GivennameRule::PrimaryName`.
#[test]
fn test_extract_givenname_rule_primary_name() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-givenname="true" givenname-disambiguation-rule="primary-name">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let Processing::Custom(custom) = config.processing.unwrap() else {
        panic!("expected custom processing mode");
    };
    let disamb = custom.disambiguate.unwrap();
    assert!(disamb.add_givenname);
    assert_eq!(
        disamb.givenname_rule,
        GivennameRule::PrimaryName,
        "givenname-disambiguation-rule=primary-name must map to GivennameRule::PrimaryName"
    );
}

/// Given a CSL `<citation>` with no `givenname-disambiguation-rule` attribute,
/// when processing options are extracted,
/// then `Disambiguation.givenname_rule` defaults to `GivennameRule::ByCite`.
#[test]
fn test_extract_givenname_rule_defaults_to_by_cite() {
    let xml = r#"<style class="in-text">
        <citation disambiguate-add-givenname="true">
            <layout><text macro="year"/></layout>
        </citation>
        <bibliography><layout><text variable="title"/></layout></bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    let processing = config.processing.unwrap();
    assert_eq!(processing, Processing::AuthorDateGivenname);

    let disamb = processing.config().disambiguate.unwrap();
    assert_eq!(
        disamb.givenname_rule,
        GivennameRule::ByCite,
        "absent givenname-disambiguation-rule must default to GivennameRule::ByCite"
    );
}

/// Remaining givenname-disambiguation-rule values round-trip through the migrator.
///
/// Given CSL citations with each of the three non-default, non-primary-name attribute
/// values, when processing options are extracted, then each maps to the correct
/// GivennameRule variant.
#[test]
fn test_extract_givenname_rule_remaining_values() {
    let parse_rule = |attr_value: &str| {
        let xml = format!(
            r#"<style class="in-text">
            <citation disambiguate-add-givenname="true" givenname-disambiguation-rule="{attr_value}">
                <layout><text macro="year"/></layout>
            </citation>
            <bibliography><layout><text variable="title"/></layout></bibliography>
        </style>"#
        );
        let style = parse_csl(&xml).unwrap();
        let config = OptionsExtractor::extract(&style);
        let Processing::Custom(custom) = config.processing.unwrap() else {
            panic!("expected custom processing");
        };
        custom.disambiguate.unwrap().givenname_rule
    };

    assert_eq!(parse_rule("all-names"), GivennameRule::AllNames);
    assert_eq!(
        parse_rule("all-names-with-initials"),
        GivennameRule::AllNamesWithInitials
    );
    assert_eq!(
        parse_rule("primary-name-with-initials"),
        GivennameRule::PrimaryNameWithInitials
    );
}

#[test]
fn test_extract_bibliography_separator_stays_literal_not_semantic() {
    // A CSL 1.0 group delimiter of "," is recognized by `from_csl_string` as
    // the semantic `Comma` mark, but migrated output must stay literal and
    // script-invariant like every other migrated style -- it must not start
    // realizing full-width if the migrated style later declares
    // `realization-default: cjk` (docs/specs/PUNCTUATION_REALIZATION.md §7).
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <layout>
                <group delimiter=", ">
                    <text variable="title"/>
                    <text variable="container-title"/>
                </group>
            </layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();

    let bibliography = super::bibliography::extract_bibliography_config(&style);

    assert_eq!(
        bibliography.and_then(|bibliography| bibliography.separator),
        Some(citum_schema::template::DelimiterPunctuation::Custom(
            ", ".to_string()
        )),
        "extracted separator should be a literal Custom(\", \"), not the semantic Comma mark"
    );
}

#[test]
fn test_extract_bibliography_entry_suffix_stays_literal() {
    // The layout suffix attribute is always a raw CSL string, never
    // classified via `from_csl_string`, so this should already be literal --
    // regression guard against a future change accidentally reclassifying it.
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <layout suffix=".">
                <text variable="title"/>
            </layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).unwrap();

    let bibliography = super::bibliography::extract_bibliography_config(&style);

    assert_eq!(
        bibliography.and_then(|bibliography| bibliography.entry_suffix),
        Some(citum_schema::template::DelimiterPunctuation::Custom(
            ".".to_string()
        ))
    );
}
