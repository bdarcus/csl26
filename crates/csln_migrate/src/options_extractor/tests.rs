use super::*;
use csl_legacy::parser::parse_style;
use csln_core::grouping::SortKey as GroupSortKey;
use csln_core::options::{Processing, SortKey, SubstituteConfig, SubstituteKey};
use roxmltree::Document;

fn parse_csl(xml: &str) -> Result<Style, String> {
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;
    parse_style(doc.root_element()).map_err(|e| e.to_string())
}

#[test]
fn test_extract_author_date_processing() {
    let xml = r#"<style class="in-text"><citation><layout><text macro="year"/></layout></citation><bibliography><layout><text variable="title"/></layout></bibliography></style>"#;
    let style = parse_csl(xml).unwrap();
    let config = OptionsExtractor::extract(&style);

    assert!(matches!(config.processing, Some(Processing::Custom(_))));
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

    let sort = custom.sort.unwrap();
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
    assert_eq!(sort.template.len(), 3);
    assert!(matches!(sort.template[0].key, GroupSortKey::Author));
    assert!(matches!(sort.template[1].key, GroupSortKey::Title));
    assert!(matches!(sort.template[2].key, GroupSortKey::Issued));
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
