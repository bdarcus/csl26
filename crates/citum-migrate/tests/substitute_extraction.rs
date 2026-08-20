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

use citum_migrate::options_extractor::OptionsExtractor;
use citum_schema::options::SubstituteKey;
use csl_legacy::model::{
    Choose, ChooseBranch, Citation, CslNode, Formatting, Info, Layout, Names, Style, Substitute,
    Text,
};

fn announce_behavior(summary: &str) {
    tracing::debug!("behavior: {summary}");
}

#[allow(
    clippy::too_many_lines,
    reason = "test functions naturally exceed 100 lines"
)]
#[test]
fn test_extract_type_conditional_substitute() {
    announce_behavior(
        "Type-conditional substitute branches keep the shared fallback template while preserving the type-specific override.",
    );
    // 1. Setup CSL with type-conditional substitute
    // <names variable="author">
    //   <substitute>
    //     <choose>
    //       <if type="classic">
    //         <text variable="title"/>
    //       </if>
    //     </choose>
    //     <names variable="editor"/>
    //   </substitute>
    // </names>

    let if_branch = ChooseBranch {
        type_: Some("classic".to_string()),
        children: vec![CslNode::Text(Text {
            variable: Some("title".to_string()),
            value: None,
            macro_name: None,
            term: None,
            form: None,
            prefix: None,
            suffix: None,
            quotes: None,
            text_case: None,
            strip_periods: None,
            plural: None,
            macro_call_order: None,
            formatting: Formatting::default(),
        })],
        match_mode: None,
        variable: None,
        is_numeric: None,
        is_uncertain_date: None,
        locator: None,
        position: None,
    };

    let choose_node = CslNode::Choose(Choose {
        if_branch,
        else_if_branches: vec![],
        else_branch: None,
    });

    let editor_names = CslNode::Names(Names {
        variable: "editor".to_string(),
        delimiter: None,
        delimiter_precedes_et_al: None,
        et_al_min: None,
        et_al_use_first: None,
        et_al_subsequent_min: None,
        et_al_subsequent_use_first: None,
        prefix: None,
        suffix: None,
        children: vec![],
        macro_call_order: None,
        formatting: Formatting::default(),
    });

    let substitute = Substitute {
        children: vec![choose_node, editor_names],
    };

    let author_names = CslNode::Names(Names {
        variable: "author".to_string(),
        delimiter: None,
        delimiter_precedes_et_al: None,
        et_al_min: None,
        et_al_use_first: None,
        et_al_subsequent_min: None,
        et_al_subsequent_use_first: None,
        prefix: None,
        suffix: None,
        children: vec![CslNode::Substitute(substitute)],
        macro_call_order: None,
        formatting: Formatting::default(),
    });

    let style = Style {
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
                children: vec![author_names],
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

    // 2. Extract config
    let config = OptionsExtractor::extract(&style);

    // 3. Verify
    assert!(config.substitute.is_some());
    let sub = config.substitute.unwrap().resolve();

    // Default template should have editor (extracted from after choose)
    assert!(sub.candidates().contains(&SubstituteKey::Editor));
    assert!(!sub.candidates().contains(&SubstituteKey::Title)); // Title is conditional

    // Overrides should have classic -> title
    assert!(sub.overrides.contains_key("classic"));
    assert_eq!(
        sub.overrides.get("classic").unwrap().as_slice(),
        &[SubstituteKey::Title]
    );
}
