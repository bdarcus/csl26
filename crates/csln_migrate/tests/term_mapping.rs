use csl_legacy::model::{CslNode, Formatting, Text};
use csln_core::CslnNode;
use csln_core::locale::{GeneralTerm, TermForm};
use csln_migrate::Upsampler;

#[test]
fn test_upsample_term() {
    let legacy_node = CslNode::Text(Text {
        term: Some("in".to_string()),
        formatting: Formatting::default(),
        value: None,
        variable: None,
        macro_name: None,
        form: None,
        prefix: None,
        suffix: None,
        quotes: None,
        text_case: None,
        strip_periods: None,
        plural: None,
        macro_call_order: None,
    });

    let upsampler = Upsampler::new();
    let csln_nodes = upsampler.upsample_nodes(&[legacy_node]);

    assert_eq!(csln_nodes.len(), 1);
    match &csln_nodes[0] {
        CslnNode::Term(term_block) => {
            assert_eq!(term_block.term, GeneralTerm::In);
            assert_eq!(term_block.form, TermForm::Long);
        }
        _ => panic!("Expected CslnNode::Term, got {:?}", csln_nodes[0]),
    }
}

#[test]
fn test_upsample_term_with_form() {
    let legacy_node = CslNode::Text(Text {
        term: Some("editor".to_string()),
        form: Some("short".to_string()),
        formatting: Formatting::default(),
        value: None,
        variable: None,
        macro_name: None,
        prefix: None,
        suffix: None,
        quotes: None,
        text_case: None,
        strip_periods: None,
        plural: None,
        macro_call_order: None,
    });

    let upsampler = Upsampler::new();
    let csln_nodes = upsampler.upsample_nodes(&[legacy_node]);

    assert_eq!(csln_nodes.len(), 1);
    match &csln_nodes[0] {
        CslnNode::Text { value } => {
            assert_eq!(value, "editor");
        }
        _ => panic!(
            "Expected CslnNode::Text for unknown term, got {:?}",
            csln_nodes[0]
        ),
    }
}
