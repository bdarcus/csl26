/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Regression coverage for a defect the [gb7714-bench] benchmark surfaced
//! (bean `csl26-6eoi`): CSL-JSON's `container-title` carries citeproc-js's
//! literal HTML rich-text convention (`<span class="nocase">`) just like
//! `title` does, but it bypassed the `title`-only conversion bean
//! `csl26-zaqk` added, so it leaked verbatim into rendered bibliography
//! output instead of being interpreted as Djot case protection.
//!
//! This is the exact `gbt7714.8.6.1:5` entry from the benchmark's data
//! source (`typst-doc-cn/bib-csl-dev-data`), rendered end to end through
//! the embedded `gb-t-7714-2025-numeric` style, so the defect cannot
//! silently return.
//!
//! [gb7714-bench]: https://gb7714.zhtyp.art/entry/gbt7714.8.6.1-5/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in test code."
)]

mod common;
use common::announce_behavior;

use citum_engine::Processor;
use citum_schema::reference::InputReference;
use indexmap::IndexMap;

/// Load the embedded zh-CN locale for GB/T fixtures — see
/// `date_annotations.rs::zh_cn_locale` for why this must match the style's
/// `info.default-locale` rather than `Processor::new`'s plain `en_us()`.
fn zh_cn_locale() -> citum_schema::Locale {
    let bytes = citum_schema::embedded::get_locale_bytes("zh-CN").expect("zh-CN must be embedded");
    citum_schema::Locale::from_yaml_str(std::str::from_utf8(bytes).expect("valid UTF-8"))
        .expect("zh-CN locale should parse")
}

#[test]
fn gbt7714_8_6_1_5_container_title_nocase_spans_render_as_plain_case_protected_text() {
    announce_behavior(
        "CSL-JSON container-title with citeproc-js nocase spans renders clean, not raw HTML",
    );

    // Verbatim from the benchmark's source data
    // (typst-doc-cn/bib-csl-dev-data, GB-T_7714—2025.builtin.json).
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_str(
        r#"{
            "id": "gbt7714.8.6.1:5",
            "type": "paper-conference",
            "author": [{"family": "Fourney", "given": "M. E."}],
            "editor": [{"family": "Gottenberg", "given": "W. G."}],
            "issued": {"date-parts": [["1971"]]},
            "language": "en-US",
            "page": "17-38",
            "publisher": "ASME",
            "publisher-place": "New York",
            "title": "Advances in holographic photoelasticity",
            "container-title": "<span class=\"nocase\">Symposium on Applications of Holography in Mechanics</span>, August 23-25, 1971, <span class=\"nocase\">University of Southern California</span>, <span class=\"nocase\">Los Angeles, California</span>"
        }"#,
    )
    .expect("benchmark fixture should parse as CSL-JSON");

    let reference: InputReference = legacy.into();
    let bibliography = IndexMap::from([("gbt7714.8.6.1:5".to_string(), reference)]);

    let style = citum_schema::embedded::get_embedded_style("gb-t-7714-2025-numeric")
        .expect("gb-t-7714-2025-numeric should be embedded")
        .expect("gb-t-7714-2025-numeric should parse")
        .into_resolved();

    let rendered =
        Processor::with_locale(style, bibliography, zh_cn_locale()).render_bibliography();

    assert_eq!(
        rendered,
        "[1]Fourney M E. Advances in holographic photoelasticity[C]//Gottenberg W G. \
         Symposium on Applications of Holography in Mechanics, August 23-25, 1971, \
         University of Southern California, Los Angeles, California. New York：ASME，1971：17-38."
    );
}
