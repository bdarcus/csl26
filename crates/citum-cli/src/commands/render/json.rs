/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! JSON reference and citation rendering.

use super::RenderContext;
use super::human::render_citation_file_entries;
use citum_engine::{Citation, CitationItem};
use std::collections::HashSet;
use std::error::Error;

/// Core JSON renderer for references and citations.
///
/// Returns a pretty-printed JSON object with `style`, `items`, and optionally
/// `citations` and `bibliography` keys.
#[allow(
    clippy::too_many_lines,
    reason = "JSON output construction is vertically long"
)]
pub(super) fn print_json_with_format<F>(
    ctx: &RenderContext<'_>,
    show_cite: bool,
    show_bib: bool,
    citations: Option<Vec<Citation>>,
) -> Result<String, Box<dyn Error>>
where
    F: citum_engine::render::format::OutputFormat<Output = String>,
{
    use serde_json::json;

    let mut result = json!({
        "style": ctx.style_name,
        "items": ctx.item_ids.len()
    });

    if show_cite {
        if let Some(cite_list) = citations {
            let rendered: Vec<_> = cite_list
                .iter()
                .zip(render_citation_file_entries::<F>(ctx.processor, &cite_list))
                .map(|(citation, text)| {
                    json!({
                        "id": citation.id,
                        "text": text
                    })
                })
                .collect();
            #[allow(clippy::indexing_slicing, reason = "JSON object insertion")]
            {
                result["citations"] = json!(rendered);
            }
        } else {
            let non_integral: Vec<_> = ctx
                .item_ids
                .iter()
                .map(|id| {
                    let citation = Citation {
                        id: Some(id.clone()),
                        items: vec![CitationItem {
                            id: id.clone(),
                            ..Default::default()
                        }],
                        mode: citum_schema::citation::CitationMode::NonIntegral,
                        ..Default::default()
                    };
                    let mut run = ctx.processor.begin_run();
                    json!({
                        "id": id,
                        "text": ctx.processor
                            .process_citation_with_format::<F>(&citation, &mut run)
                            .unwrap_or_else(|e| e.to_string())
                    })
                })
                .collect();

            let integral: Vec<_> = ctx
                .item_ids
                .iter()
                .map(|id| {
                    let citation = Citation {
                        id: Some(id.clone()),
                        items: vec![CitationItem {
                            id: id.clone(),
                            ..Default::default()
                        }],
                        mode: citum_schema::citation::CitationMode::Integral,
                        ..Default::default()
                    };
                    let mut run = ctx.processor.begin_run();
                    json!({
                        "id": id,
                        "text": ctx.processor
                            .process_citation_with_format::<F>(&citation, &mut run)
                            .unwrap_or_else(|e| e.to_string())
                    })
                })
                .collect();

            #[allow(clippy::indexing_slicing, reason = "JSON object insertion")]
            {
                result["citations"] = json!({
                    "non-integral": non_integral,
                    "integral": integral
                });
            }
        }
    }

    if show_bib {
        let filter: HashSet<&str> = ctx
            .item_ids
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let processed = ctx.processor.process_references();
        let entries: Vec<_> = processed
            .bibliography
            .into_iter()
            .filter(|entry| filter.contains(entry.id.as_str()))
            .map(|entry| {
                let text = citum_engine::render::refs_to_string_slice_with_format::<F>(
                    std::slice::from_ref(&entry),
                    ctx.annotations,
                    Some(ctx.annotation_style),
                );
                json!({
                    "id": entry.id,
                    "text": text.trim()
                })
            })
            .collect();

        #[allow(clippy::indexing_slicing, reason = "JSON object insertion")]
        {
            result["bibliography"] = json!({ "entries": entries });
        }
    }

    Ok(serde_json::to_string_pretty(&result)?)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests"
)]
mod tests {
    use super::*;
    use crate::style_resolver::create_processor;
    use citum_engine::render::plain::PlainText;
    use citum_engine::{Bibliography, Processor};
    use citum_io::{AnnotationStyle, LoadedBibliography, load_merged_bibliography};
    use citum_schema::Style;
    use citum_schema::citation::CitationMode;
    use citum_schema::grouping::{GroupSort, GroupSortEntry, GroupSortKey, SortKey};
    use citum_schema::options::{Config, Processing};
    use citum_schema::template::WrapPunctuation;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_print_json_batches_numeric_citation_files() {
        let _ = create_processor; // silence unused-import in this test fn
        let style = Style {
            citation: Some(citum_schema::CitationSpec {
                options: Some(citum_schema::CitationOptions {
                    label_mode: Some(citum_schema::options::CitationLabelMode::Numeric),
                    ..Default::default()
                }),
                wrap: Some(WrapPunctuation::Brackets.into()),
                ..Default::default()
            }),
            bibliography: Some(citum_schema::BibliographySpec {
                sort: Some(GroupSortEntry::Explicit(GroupSort {
                    template: vec![GroupSortKey {
                        key: SortKey::Author,
                        ascending: true,
                        order: None,
                        sort_order: None,
                    }],
                })),
                ..Default::default()
            }),
            options: Some(Config {
                processing: Some(Processing::Numeric),
                ..Default::default()
            }),
            ..Default::default()
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("citum-batch-citations-{now}"));
        std::fs::create_dir_all(&base).expect("temp dir should be created");
        let bib_path = base.join("refs.yaml");
        std::fs::write(
            &bib_path,
            r#"
references:
  - class: monograph
    id: smith2020
    type: book
    title: Smith Book
    author:
      - family: Smith
        given: Jane
    issued: "2020"
  - class: monograph
    id: adams2021
    type: book
    title: Adams Book
    author:
      - family: Adams
        given: Amy
    issued: "2021"
"#,
        )
        .expect("fixture should write");

        let _ = Bibliography::new;
        let _ = LoadedBibliography {
            references: Bibliography::new(),
            sets: None,
        };
        let loaded = load_merged_bibliography(std::slice::from_ref(&bib_path))
            .expect("bibliography should load");
        let processor = Processor::new(style, loaded.references);
        let citations = vec![Citation {
            id: Some("c1".into()),
            items: vec![CitationItem {
                id: "smith2020".to_string(),
                ..Default::default()
            }],
            mode: CitationMode::NonIntegral,
            ..Default::default()
        }];

        let item_ids = vec!["smith2020".to_string(), "adams2021".to_string()];
        let annotation_style = AnnotationStyle::default();
        let render_ctx = RenderContext {
            processor: &processor,
            style_name: "numeric-test",
            item_ids: &item_ids,
            annotations: None,
            annotation_style: &annotation_style,
        };
        let output = print_json_with_format::<PlainText>(&render_ctx, true, false, Some(citations))
            .expect("json rendering should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("output should be valid JSON");

        assert_eq!(parsed["citations"][0]["text"], "[2]");

        let _ = std::fs::remove_file(bib_path);
        let _ = std::fs::remove_dir(base);
    }
}
