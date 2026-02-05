use super::*;
use csl_legacy::parser::parse_style;
use csln_core::options::{Processing, SubstituteConfig, SubstituteKey};
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
