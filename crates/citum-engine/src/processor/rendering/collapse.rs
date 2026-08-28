/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citation number and compound subentry collapsing.

use super::{CitationChunk, Renderer};
use crate::processor::rendering::marker::MarkerValue;
use crate::values::number::format_page_range;
use crate::values::range::{ConsecutiveSegment, consecutive_segments};
#[cfg(test)]
use std::sync::Arc;

impl Renderer<'_> {
    /// Collapse consecutive numeric citation chunks into ranges (e.g., "1–3").
    ///
    /// Only applies when:
    /// - The chunk contains exactly one reference ID.
    /// - The chunk is semantically a bare numeric label.
    /// - The citation numbers are consecutive.
    #[allow(clippy::indexing_slicing, reason = "loop-guaranteed indices")]
    pub(super) fn collapse_numeric_citation_chunks(
        &self,
        chunks: Vec<CitationChunk>,
    ) -> Vec<CitationChunk> {
        let mut collapsed = Vec::new();
        let mut i = 0;

        while i < chunks.len() {
            let Some(_ref_id) = chunks[i].ids.first() else {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            };
            if chunks[i].ids.len() != 1 {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            }
            let Some(citation_number) = chunks[i]
                .marker
                .as_ref()
                .and_then(|marker| marker.value.collapsible_number())
            else {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            };

            let mut j = i;
            let mut block_ids = Vec::new();
            let mut end_number = citation_number;

            while j < chunks.len() {
                let Some(candidate_id) = chunks[j].ids.first() else {
                    break;
                };
                if chunks[j].ids.len() != 1 {
                    break;
                }
                let Some(candidate_number) = chunks[j]
                    .marker
                    .as_ref()
                    .and_then(|marker| marker.value.collapsible_number())
                else {
                    break;
                };
                if !block_ids.is_empty() && candidate_number != end_number + 1 {
                    break;
                }

                block_ids.push(candidate_id.clone());
                end_number = candidate_number;
                j += 1;
            }

            if block_ids.len() < 2 {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            }

            let range_format = self
                .config
                .citation_numbers
                .as_ref()
                .and_then(|cfg| cfg.range_format.as_ref());
            let range = format_page_range(
                &format!("{citation_number}-{end_number}"),
                range_format,
                self.identifier_range_delimiter(),
            );
            collapsed.push(CitationChunk {
                ids: block_ids,
                content: String::new(),
                marker: chunks[i].marker.clone().map(|mut marker| {
                    marker.value = MarkerValue::Range(range);
                    marker
                }),
            });
            i = j;
        }

        collapsed
    }

    /// Collapse consecutive compound sub-labels into ranges (e.g., "1a–c").
    ///
    /// Only applies for alphabetic sub-labels when:
    /// - The chunks belong to the same numeric citation (same number).
    /// - The chunks belong to the same compound set.
    /// - The sub-labels are consecutive.
    #[allow(clippy::indexing_slicing, reason = "loop-guaranteed indices")]
    pub(super) fn collapse_compound_citation_chunks(
        &self,
        chunks: Vec<CitationChunk>,
    ) -> Vec<CitationChunk> {
        let Some(compound) = self
            .bibliography_config
            .as_ref()
            .and_then(|b| b.compound_numeric.as_ref())
        else {
            return chunks;
        };

        if !matches!(
            compound.sub_label,
            citum_schema::options::bibliography::SubLabelStyle::Alphabetic
        ) {
            return chunks;
        }

        let mut collapsed = Vec::new();
        let mut i = 0;

        while i < chunks.len() {
            let Some(ref_id) = chunks[i].ids.first() else {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            };
            let Some(group_id) = self.compound_set_by_ref.get(ref_id) else {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            };
            let Some((citation_number, _)) = chunks[i]
                .marker
                .as_ref()
                .and_then(|marker| marker.value.number_parts())
            else {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            };

            let mut j = i;
            let mut block_ids = Vec::new();
            let mut member_ordinals = Vec::new();

            while j < chunks.len() {
                let Some(candidate_id) = chunks[j].ids.first() else {
                    break;
                };
                if chunks[j].ids.len() != 1
                    || self.compound_set_by_ref.get(candidate_id) != Some(group_id)
                    || chunks[j]
                        .marker
                        .as_ref()
                        .and_then(|marker| marker.value.number_parts())
                        .is_none_or(|(number, _)| number != citation_number)
                {
                    break;
                }

                let Some(member_index) = self.compound_member_index.get(candidate_id).copied()
                else {
                    break;
                };
                let Some((_, Some(_))) = chunks[j]
                    .marker
                    .as_ref()
                    .and_then(|marker| marker.value.number_parts())
                else {
                    break;
                };

                block_ids.push(candidate_id.clone());
                member_ordinals.push((member_index + 1) as u32);
                j += 1;
            }

            if block_ids.len() < 2 {
                collapsed.push(chunks[i].clone());
                i += 1;
                continue;
            }

            let range_delimiter = self.identifier_range_delimiter();
            let labels = consecutive_segments(&member_ordinals)
                .into_iter()
                .map(|segment| format_compound_sub_label_segment(segment, range_delimiter))
                .collect::<Vec<_>>()
                .join(",");

            collapsed.push(CitationChunk {
                ids: block_ids,
                content: String::new(),
                marker: chunks[i].marker.clone().map(|mut marker| {
                    marker.value = MarkerValue::Number {
                        number: citation_number,
                        sub_label: Some(labels),
                    };
                    marker
                }),
            });
            i = j;
        }

        collapsed
    }
}

fn format_compound_sub_label_segment(segment: ConsecutiveSegment, range_delimiter: &str) -> String {
    match segment {
        ConsecutiveSegment::Single(value) => {
            crate::values::int_to_letter(value).unwrap_or_default()
        }
        ConsecutiveSegment::Range { start, end } => {
            let start_label = crate::values::int_to_letter(start).unwrap_or_default();
            let end_label = crate::values::int_to_letter(end).unwrap_or_default();
            format!("{start_label}{range_delimiter}{end_label}")
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::reference::Bibliography;
    use crate::values::ProcHints;
    use citum_schema::locale::Locale;
    use citum_schema::options::Config;
    use citum_schema::options::bibliography::BibliographyConfig;
    use indexmap::IndexMap;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[allow(clippy::too_many_arguments, reason = "test helper")]
    fn make_renderer<'a>(
        style: &'a citum_schema::Style,
        bib: &'a Bibliography,
        loc: &'a Locale,
        cfg: Arc<Config>,
        hints: &'a HashMap<String, ProcHints>,
        citation_numbers: &'a RwLock<HashMap<String, usize>>,
        compound_set_by_ref: &'a HashMap<String, String>,
        compound_member_index: &'a HashMap<String, usize>,
        compound_sets: &'a IndexMap<String, Vec<String>>,
        bibliography_config: Option<Arc<BibliographyConfig>>,
    ) -> Renderer<'a> {
        Renderer {
            style,
            bibliography: bib,
            locale: loc,
            config: cfg,
            bibliography_config,
            hints,
            citation_numbers,
            compound_set_by_ref,
            compound_member_index,
            compound_sets,
            show_semantics: false,
            inject_ast_indices: false,
            filtered_to_original_index: RefCell::new(None),
            abbreviation_map: None,
            first_note_by_id: None,
        }
    }

    #[test]
    fn test_collapse_numeric() {
        let style = citum_schema::Style::default();
        let bib = Bibliography::default();
        let loc = Locale::default();
        let cfg = Arc::new(Config::default());
        let hints = HashMap::new();

        let mut nums = HashMap::new();
        nums.insert("A".to_string(), 1);
        nums.insert("B".to_string(), 2);
        nums.insert("C".to_string(), 3);
        nums.insert("D".to_string(), 4);
        let citation_numbers = RwLock::new(nums);
        let empty_map_string = HashMap::new();
        let empty_map_usize = HashMap::new();
        let empty_index = IndexMap::new();

        let renderer = make_renderer(
            &style,
            &bib,
            &loc,
            cfg,
            &hints,
            &citation_numbers,
            &empty_map_string,
            &empty_map_usize,
            &empty_index,
            None,
        );

        let cases = [
            (
                vec![
                    (vec!["A".to_string()], "1".to_string()),
                    (vec!["B".to_string()], "2".to_string()),
                    (vec!["C".to_string()], "3".to_string()),
                ],
                vec![(
                    vec!["A".to_string(), "B".to_string(), "C".to_string()],
                    "1–3".to_string(),
                )],
            ),
            (
                vec![
                    (vec!["A".to_string()], "1".to_string()),
                    (vec!["B".to_string()], "2".to_string()),
                    (vec!["D".to_string()], "4".to_string()),
                ],
                vec![
                    (vec!["A".to_string(), "B".to_string()], "1–2".to_string()),
                    (vec!["D".to_string()], "4".to_string()),
                ],
            ),
        ];

        for (chunks, expected) in cases {
            let chunks = chunks
                .into_iter()
                .map(|(ids, _content)| CitationChunk {
                    marker: ids.first().and_then(|id| {
                        citation_numbers.read().unwrap().get(id).copied().map(
                            |number| crate::processor::rendering::ResolvedMarker {
                                value: MarkerValue::Number {
                                    number,
                                    sub_label: None,
                                },
                                spec: crate::processor::rendering::marker::CitationMarkerSpec {
                                    kind: crate::processor::rendering::marker::MarkerKind::Numeric,
                                    label_wrap: None,
                                    item_wrap: None,
                                    placement: crate::processor::rendering::marker::MarkerPlacement::Leading,
                                },
                            },
                        )
                    }),
                    ids,
                    content: String::new(),
                })
                .collect();
            let actual = renderer.collapse_numeric_citation_chunks(chunks);
            let actual = actual
                .into_iter()
                .map(|chunk| {
                    let text = chunk
                        .marker
                        .as_ref()
                        .map(|marker| marker.value.as_text())
                        .unwrap_or(chunk.content);
                    (chunk.ids, text)
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn given_citation_numbers_range_format_when_collapsing_then_it_abbreviates() {
        // given a style that opts citation-number ranges into Chicago
        // abbreviation, independent of the style-wide range-format default
        let style = citum_schema::Style::default();
        let bib = Bibliography::default();
        let loc = Locale::default();
        let config = Config {
            identifier_range_delimiter: Some("~".to_string()),
            citation_numbers: Some(citum_schema::options::CitationNumberConfig {
                range_format: Some(citum_schema::options::RangeFormat::Chicago),
            }),
            ..Default::default()
        };
        let cfg = Arc::new(config);
        let hints = HashMap::new();

        let mut nums = HashMap::new();
        nums.insert("A".to_string(), 1002);
        nums.insert("B".to_string(), 1003);
        nums.insert("C".to_string(), 1004);
        nums.insert("D".to_string(), 1005);
        nums.insert("E".to_string(), 1006);
        let citation_numbers = RwLock::new(nums);
        let empty_map_string = HashMap::new();
        let empty_map_usize = HashMap::new();
        let empty_index = IndexMap::new();

        let renderer = make_renderer(
            &style,
            &bib,
            &loc,
            cfg,
            &hints,
            &citation_numbers,
            &empty_map_string,
            &empty_map_usize,
            &empty_index,
            None,
        );

        let chunks = ["A", "B", "C", "D", "E"]
            .into_iter()
            .map(|id| CitationChunk {
                marker: citation_numbers
                    .read()
                    .unwrap()
                    .get(id)
                    .copied()
                    .map(|number| crate::processor::rendering::ResolvedMarker {
                        value: MarkerValue::Number {
                            number,
                            sub_label: None,
                        },
                        spec: crate::processor::rendering::marker::CitationMarkerSpec {
                            kind: crate::processor::rendering::marker::MarkerKind::Numeric,
                            label_wrap: None,
                            item_wrap: None,
                            placement:
                                crate::processor::rendering::marker::MarkerPlacement::Leading,
                        },
                    }),
                ids: vec![id.to_string()],
                content: String::new(),
            })
            .collect();

        // when collapsed
        let actual = renderer.collapse_numeric_citation_chunks(chunks);

        // then the range abbreviates per Chicago rules, matching the page
        // variable's own algorithm (values/number.rs::format_page_range)
        assert_eq!(actual.len(), 1);
        assert_eq!(
            actual[0]
                .marker
                .as_ref()
                .map(|marker| marker.value.as_text()),
            Some("1002~6".to_string())
        );
    }
}
