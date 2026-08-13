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
    reason = "Panicking is acceptable and often desired in test code."
)]

use citum_migrate::options_extractor::contributors::extract_citation_contributor_overrides;
use citum_schema::options::DelimiterPrecedesLast;
use csl_legacy::parser::parse_style;
use roxmltree::Document;

#[test]
fn explicit_csl_delimiter_rule_does_not_infer_citum_two_name_policy() {
    let xml = r#"<style>
        <citation>
            <layout>
                <names variable="author">
                    <name and="text" delimiter-precedes-last="always"/>
                </names>
            </layout>
        </citation>
    </style>"#;
    let document = Document::parse(xml).expect("CSL should parse as XML");
    let style = parse_style(document.root_element()).expect("legacy style should parse");

    let contributors =
        extract_citation_contributor_overrides(&style).expect("contributor options should extract");

    assert_eq!(
        contributors.delimiter_precedes_last,
        Some(DelimiterPrecedesLast::Always)
    );
    assert_eq!(contributors.two_name_delimiter_policy, None);
}
