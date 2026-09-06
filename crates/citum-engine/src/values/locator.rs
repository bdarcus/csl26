/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Locator rendering logic for citations.
//!
//! Renders citation locators (page numbers, sections, etc.) with configurable
//! labels, range formatting, and compound locator patterns.

use citum_schema::citation::{CitationLocator, LocatorSegment, LocatorType};
use citum_schema::locale::{Locale, TermForm};
use citum_schema::options::{LabelForm, LabelRepeat, LocatorConfig, RangeFormat, TextCase};

/// Render a citation locator to a display string.
///
/// All label, range, and delimiter decisions are driven by `config`.
/// Returns an empty string when the locator is absent.
///
/// # Arguments
/// * `locator` - The citation locator to render.
/// * `ref_type` - The reference type for optional type-class gating.
/// * `config` - The locator configuration.
/// * `locale` - The locale for term lookup.
/// * `style_range_format` - The style-wide `options.range-format` default.
///   Applies to every locator kind (see
///   `docs/specs/RANGE_COLLAPSE_MODEL.md` Decision 2) unless `config` or a
///   per-kind entry overrides it.
/// * `style_range_delimiter` - The style-wide `options.range-delimiter`
///   override, falling back to the locale's page-range delimiter.
#[must_use]
pub fn render_locator(
    locator: &CitationLocator,
    ref_type: &str,
    config: &LocatorConfig,
    locale: &Locale,
    style_range_format: Option<&RangeFormat>,
    style_range_delimiter: Option<&str>,
) -> String {
    let segments = locator.segments();
    let range_delimiter =
        style_range_delimiter.unwrap_or(locale.grammar_options.page_range_delimiter.as_str());

    // Collect the set of locator kinds present in the locator
    let kinds: std::collections::HashSet<LocatorType> =
        segments.iter().map(|seg| seg.label.clone()).collect();
    let pattern = find_matching_pattern(&kinds, ref_type, config);

    if let Some(pattern) = pattern {
        render_with_pattern(
            segments,
            pattern,
            config,
            locale,
            style_range_format,
            range_delimiter,
        )
    } else {
        render_default(
            segments,
            config,
            locale,
            style_range_format,
            range_delimiter,
        )
    }
}

/// Find the first `LocatorPattern` whose `kinds` set is a subset of the
/// locator's active kinds and whose optional `type_class` gate matches
/// `ref_type`. Patterns are tested in declaration order.
///
/// Shared by [`render_locator`] and [`effective_attach`] so both agree on
/// which pattern (if any) governs a given locator.
fn find_matching_pattern<'a>(
    kinds: &std::collections::HashSet<LocatorType>,
    ref_type: &str,
    config: &'a LocatorConfig,
) -> Option<&'a citum_schema::options::LocatorPattern> {
    config.patterns.iter().find(|p| {
        let pattern_kinds: std::collections::HashSet<LocatorType> =
            p.kinds.iter().cloned().collect();
        if !pattern_kinds.is_subset(kinds) {
            return false;
        }
        if let Some(type_class) = p.type_class
            && !crate::values::type_class::matches_type_class(ref_type, type_class)
        {
            return false;
        }
        true
    })
}

/// Resolve the delimiter that should join this locator to its preceding
/// sibling, per `docs/specs/LOCATOR_RENDERING.md` ("Label Case and
/// Attachment"): the `attach` of the first kind in a matched pattern's
/// `order`, or of the locator's first (and, for a `Single` locator, only)
/// segment kind when no pattern matches, falling back to `config.attach`.
///
/// Returns `None` when no `attach` applies at any level — callers must not
/// overwrite an existing template-authored prefix in that case.
#[must_use]
pub fn effective_attach(
    locator: &CitationLocator,
    ref_type: &str,
    config: &LocatorConfig,
) -> Option<citum_schema::template::DelimiterPunctuation> {
    let segments = locator.segments();
    let kinds: std::collections::HashSet<LocatorType> =
        segments.iter().map(|seg| seg.label.clone()).collect();
    let pattern = find_matching_pattern(&kinds, ref_type, config);

    let governing_kind = match pattern {
        Some(pattern) => pattern.order.first().cloned(),
        None => segments.first().map(|seg| seg.label.clone()),
    };

    governing_kind
        .and_then(|kind| config.kinds.get(&kind).and_then(|k| k.attach.clone()))
        .or_else(|| config.attach.clone())
}

/// Render segments using a matched pattern.
fn render_with_pattern(
    segments: &[LocatorSegment],
    pattern: &citum_schema::options::LocatorPattern,
    config: &LocatorConfig,
    locale: &Locale,
    style_range_format: Option<&RangeFormat>,
    range_delimiter: &str,
) -> String {
    let mut rendered = Vec::new();

    for (idx, kind) in pattern.order.iter().enumerate() {
        // Find the segment with this kind
        if let Some(seg) = segments.iter().find(|s| s.label == *kind) {
            let kind_cfg = config.kinds.get(kind);
            let should_label = matches!(pattern.label_repeat, LabelRepeat::All)
                || (matches!(pattern.label_repeat, LabelRepeat::First) && idx == 0);

            let rendered_segment = if should_label {
                let form = kind_cfg
                    .and_then(|cfg| cfg.label_form)
                    .unwrap_or(config.default_label_form);
                render_segment_with_label(
                    seg,
                    kind_cfg,
                    form,
                    config,
                    config.strip_label_periods,
                    config.label_case,
                    locale,
                    style_range_format,
                    range_delimiter,
                )
            } else {
                let range_format = effective_range_format(kind_cfg, config, style_range_format);
                crate::values::number::format_page_range(
                    seg.value.value_str(),
                    Some(&range_format),
                    range_delimiter,
                )
            };

            rendered.push(rendered_segment);
        }
    }

    // Render segments not covered by pattern.order using default rendering
    let covered: std::collections::HashSet<LocatorType> = pattern.order.iter().cloned().collect();
    for seg in segments.iter().filter(|s| !covered.contains(&s.label)) {
        let kind_cfg = config.kinds.get(&seg.label);
        let form = kind_cfg
            .and_then(|cfg| cfg.label_form)
            .unwrap_or(config.default_label_form);
        let range_format = effective_range_format(kind_cfg, config, style_range_format);
        let value_str = crate::values::number::format_page_range(
            seg.value.value_str(),
            Some(&range_format),
            range_delimiter,
        );
        let rendered_segment = if matches!(form, LabelForm::None) {
            value_str
        } else {
            render_segment_with_label_str(
                seg,
                kind_cfg,
                form,
                &value_str,
                config.strip_label_periods,
                config.label_case,
                locale,
            )
        };
        rendered.push(rendered_segment);
    }

    rendered.join(&pattern.delimiter)
}

/// Render segments without a matched pattern (default behavior).
fn render_default(
    segments: &[LocatorSegment],
    config: &LocatorConfig,
    locale: &Locale,
    style_range_format: Option<&RangeFormat>,
    range_delimiter: &str,
) -> String {
    let mut rendered = Vec::new();

    for seg in segments {
        let kind_cfg = config.kinds.get(&seg.label);
        let form = kind_cfg
            .and_then(|cfg| cfg.label_form)
            .unwrap_or(config.default_label_form);

        let rendered_segment = if matches!(form, LabelForm::None) {
            let range_format = effective_range_format(kind_cfg, config, style_range_format);
            crate::values::number::format_page_range(
                seg.value.value_str(),
                Some(&range_format),
                range_delimiter,
            )
        } else {
            render_segment_with_label(
                seg,
                kind_cfg,
                form,
                config,
                config.strip_label_periods,
                config.label_case,
                locale,
                style_range_format,
                range_delimiter,
            )
        };

        rendered.push(rendered_segment);
    }

    rendered.join(&config.fallback_delimiter)
}

/// Render a single segment with label.
#[allow(
    clippy::too_many_arguments,
    reason = "internal helper, callers are the small set above"
)]
fn render_segment_with_label(
    seg: &LocatorSegment,
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    form: LabelForm,
    config: &LocatorConfig,
    global_strip: Option<bool>,
    global_label_case: Option<TextCase>,
    locale: &Locale,
    style_range_format: Option<&RangeFormat>,
    range_delimiter: &str,
) -> String {
    let range_format = effective_range_format(kind_cfg, config, style_range_format);
    let value_str = crate::values::number::format_page_range(
        seg.value.value_str(),
        Some(&range_format),
        range_delimiter,
    );
    render_segment_with_label_str(
        seg,
        kind_cfg,
        form,
        &value_str,
        global_strip,
        global_label_case,
        locale,
    )
}

/// Render a single segment with label, given a pre-computed value string.
#[allow(
    clippy::too_many_arguments,
    reason = "internal helper, callers are the small set above"
)]
fn render_segment_with_label_str(
    seg: &LocatorSegment,
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    form: LabelForm,
    value_str: &str,
    global_strip: Option<bool>,
    global_label_case: Option<TextCase>,
    locale: &Locale,
) -> String {
    let plural = seg.value.is_plural();

    let term_form = match form {
        LabelForm::Short => TermForm::Short,
        LabelForm::Long => TermForm::Long,
        LabelForm::Symbol => TermForm::Symbol,
        LabelForm::None => TermForm::Short, // Shouldn't reach here
    };

    if let Some(term) = locale.resolved_locator_term(&seg.label, plural, &term_form, None) {
        let strip_periods = kind_cfg
            .and_then(|k| k.strip_label_periods)
            .or(global_strip)
            == Some(true);
        let label_case = kind_cfg.and_then(|k| k.label_case).or(global_label_case);
        let (term, glued) = if strip_periods {
            // Stripping the trailing period removes the natural separator,
            // so no additional space is added (e.g. "p." → "p23" not "p 23").
            (crate::values::strip_trailing_periods(&term), true)
        } else {
            (term, false)
        };
        let term = match label_case {
            Some(case) => crate::values::text_case::apply_text_case_with_language(
                &term,
                case,
                Some(locale.locale.as_str()),
            ),
            None => term,
        };
        if glued {
            format!("{term}{value_str}")
        } else {
            format!("{term} {value_str}")
        }
    } else {
        value_str.to_string()
    }
}

/// Resolve the effective range format for a locator segment.
///
/// Chain: per-kind override -> `locators.range-format` override -> the
/// style-wide `options.range-format` default (applies to every locator kind
/// per `docs/specs/RANGE_COLLAPSE_MODEL.md` Decision 2) -> `Expanded`.
fn effective_range_format(
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    config: &LocatorConfig,
    style_range_format: Option<&RangeFormat>,
) -> RangeFormat {
    kind_cfg
        .and_then(|k| k.range_format.clone())
        .or_else(|| config.range_format.clone())
        .or_else(|| style_range_format.cloned())
        .unwrap_or_default()
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
    use citum_schema::citation::LocatorValue;
    use citum_schema::options::{LabelForm, LocatorConfig};
    use rstest::rstest;

    #[test]
    fn test_render_single_page_locator_with_short_label() {
        // given a short-label config and a single page locator
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            ..Default::default()
        };
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("42".to_string()),
        });
        // when rendered with the default locale (which has "p." for page)
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then the output includes the short label and value
        assert!(
            result.contains("42"),
            "should contain the page number: {result}"
        );
    }

    #[test]
    fn test_render_single_page_locator_no_label() {
        // given a none-label config and a single page locator
        let config = LocatorConfig {
            default_label_form: LabelForm::None,
            ..Default::default()
        };
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("42".to_string()),
        });
        // when rendered
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then output is just the bare value
        assert_eq!(result, "42");
    }

    #[test]
    fn test_render_compound_locator_page_line_pattern() {
        use citum_schema::options::{LabelRepeat, LocatorPattern};
        // given a config with a page+line pattern
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            patterns: vec![LocatorPattern {
                kinds: vec![LocatorType::Page, LocatorType::Line],
                type_class: None,
                order: vec![LocatorType::Page, LocatorType::Line],
                delimiter: ", ".to_string(),
                label_repeat: LabelRepeat::First,
                unknown_fields: Default::default(),
            }],
            ..Default::default()
        };
        let locator = CitationLocator::Compound {
            segments: vec![
                LocatorSegment {
                    label: LocatorType::Page,
                    value: LocatorValue::Text("33".to_string()),
                },
                LocatorSegment {
                    label: LocatorType::Line,
                    value: LocatorValue::Text("5".to_string()),
                },
            ],
        };
        // when rendered
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then label appears only on first segment, value present for both
        assert!(result.contains("33"), "should contain page value: {result}");
        assert!(result.contains('5'), "should contain line value: {result}");
    }

    #[test]
    fn test_render_global_strip_label_periods() {
        // given a config with global strip_label_periods = true
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            strip_label_periods: Some(true),
            ..Default::default()
        };
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("42".to_string()),
        });
        // when rendered
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then the label has no trailing period
        assert!(result.contains("42"), "should contain page value: {result}");
        assert!(
            !result.contains("p."),
            "label period should be stripped: {result}"
        );
    }

    #[test]
    fn test_render_global_range_format_applies_to_labeled_and_unlabeled_locators() {
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            range_format: Some(RangeFormat::Chicago),
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  page:
    short:
      singular: "page"
      plural: "page"
"#,
        )
        .expect("custom locale should parse");
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("3-10".to_string()),
        });

        assert_eq!(
            render_locator(&locator, "book", &config, &locale, None, None),
            "page 3–10"
        );

        let unlabeled_config = LocatorConfig {
            default_label_form: LabelForm::None,
            range_format: Some(RangeFormat::Chicago),
            ..Default::default()
        };

        assert_eq!(
            render_locator(&locator, "book", &unlabeled_config, &locale, None, None),
            "3–10"
        );
    }

    #[test]
    fn test_render_style_range_delimiter_applies_to_page_and_non_page_locators() {
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  page:
    short:
      singular: "page"
      plural: "pages"
  chapter:
    short:
      singular: "chapter"
      plural: "chapters"
"#,
        )
        .expect("custom locale should parse");
        let page = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("10-12".to_string()),
        });
        let chapter = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Chapter,
            value: LocatorValue::Text("3-5".to_string()),
        });

        assert_eq!(
            render_locator(&page, "book", &config, &locale, None, Some("~")),
            "pages 10~12"
        );
        assert_eq!(
            render_locator(&chapter, "book", &config, &locale, None, Some("~")),
            "chapters 3~5"
        );
    }

    #[test]
    fn test_render_style_wide_range_format_applies_to_locator_kinds_by_default() {
        // Decision 2 of docs/specs/RANGE_COLLAPSE_MODEL.md: the style-wide
        // `options.range-format` default reaches every locator kind, not
        // just pages, when neither the locator config nor a per-kind entry
        // overrides it.
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  chapter:
    short:
      singular: "chapter"
      plural: "chapters"
"#,
        )
        .expect("custom locale should parse");
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Chapter,
            value: LocatorValue::Text("112-118".to_string()),
        });

        assert_eq!(
            render_locator(
                &locator,
                "book",
                &config,
                &locale,
                Some(&RangeFormat::Chicago),
                None,
            ),
            "chapters 112–18"
        );

        let mut kinds = std::collections::HashMap::new();
        kinds.insert(
            LocatorType::Chapter,
            citum_schema::options::LocatorKindConfig {
                range_format: Some(RangeFormat::Expanded),
                ..Default::default()
            },
        );
        let kind_override = LocatorConfig {
            default_label_form: LabelForm::Short,
            kinds,
            ..Default::default()
        };
        assert_eq!(
            render_locator(
                &locator,
                "book",
                &kind_override,
                &locale,
                Some(&RangeFormat::Chicago),
                None,
            ),
            "chapters 112–118"
        );
    }

    #[test]
    fn test_render_kind_range_override_applies_to_labeled_and_unlabeled_locators() {
        let mut kinds = std::collections::HashMap::new();
        kinds.insert(
            LocatorType::Page,
            citum_schema::options::LocatorKindConfig {
                range_format: Some(RangeFormat::Chicago),
                ..Default::default()
            },
        );
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            range_format: Some(RangeFormat::Expanded),
            kinds,
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  page:
    short:
      singular: "page"
      plural: "page"
"#,
        )
        .expect("custom locale should parse");
        let labeled = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("505-517".to_string()),
        });
        let unlabeled = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("1002-1006".to_string()),
        });

        assert_eq!(
            render_locator(&labeled, "book", &config, &locale, None, None),
            "page 505–17"
        );

        let unlabeled_config = LocatorConfig {
            default_label_form: LabelForm::None,
            range_format: Some(RangeFormat::Expanded),
            kinds: config.kinds.clone(),
            ..Default::default()
        };
        assert_eq!(
            render_locator(&unlabeled, "book", &unlabeled_config, &locale, None, None,),
            "1002–6"
        );
    }

    #[test]
    fn test_render_type_class_gated_pattern() {
        use citum_schema::options::{LabelRepeat, LocatorPattern, TypeClass};
        // given a config with a legal-only pattern
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            patterns: vec![LocatorPattern {
                kinds: vec![LocatorType::Page],
                type_class: Some(TypeClass::Legal),
                order: vec![LocatorType::Page],
                delimiter: ", ".to_string(),
                label_repeat: LabelRepeat::None,
                unknown_fields: Default::default(),
            }],
            ..Default::default()
        };
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Page,
            value: LocatorValue::Text("42".to_string()),
        });
        // when rendered as a non-legal type
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then the legal pattern does NOT apply (default rendering applies instead)
        assert!(result.contains("42"));
    }

    #[test]
    fn test_render_label_repeat_all() {
        use citum_schema::options::{LabelRepeat, LocatorPattern};
        // given a config with LabelRepeat::All on a compound pattern
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            patterns: vec![LocatorPattern {
                kinds: vec![LocatorType::Page, LocatorType::Line],
                type_class: None,
                order: vec![LocatorType::Page, LocatorType::Line],
                delimiter: ", ".to_string(),
                label_repeat: LabelRepeat::All,
                unknown_fields: Default::default(),
            }],
            ..Default::default()
        };
        let locator = CitationLocator::Compound {
            segments: vec![
                LocatorSegment {
                    label: LocatorType::Page,
                    value: LocatorValue::Text("33".to_string()),
                },
                LocatorSegment {
                    label: LocatorType::Line,
                    value: LocatorValue::Text("5".to_string()),
                },
            ],
        };
        // when rendered
        let result = render_locator(&locator, "book", &config, &Locale::default(), None, None);
        // then both segments contain their values
        assert!(result.contains("33"));
        assert!(result.contains('5'));
    }

    #[test]
    fn test_render_custom_locator_with_locale_defined_label() {
        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  reel:
    short:
      singular: "reel"
      plural: "reels"
"#,
        )
        .expect("custom locale should parse");
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Custom("reel".to_string()),
            value: LocatorValue::Text("3".to_string()),
        });

        assert_eq!(
            render_locator(&locator, "book", &config, &locale, None, None),
            "reel 3"
        );
    }

    #[test]
    fn test_render_custom_locator_pattern_matches_custom_kind() {
        use citum_schema::options::{LabelRepeat, LocatorPattern};

        let config = LocatorConfig {
            default_label_form: LabelForm::Short,
            patterns: vec![LocatorPattern {
                kinds: vec![LocatorType::Custom("reel".to_string())],
                type_class: None,
                order: vec![LocatorType::Custom("reel".to_string())],
                delimiter: " | ".to_string(),
                label_repeat: LabelRepeat::All,
                unknown_fields: Default::default(),
            }],
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  reel:
    short:
      singular: "reel"
      plural: "reels"
"#,
        )
        .expect("custom locale should parse");
        let locator = CitationLocator::Single(LocatorSegment {
            label: LocatorType::Custom("reel".to_string()),
            value: LocatorValue::Text("3".to_string()),
        });

        assert_eq!(
            render_locator(&locator, "book", &config, &locale, None, None),
            "reel 3"
        );
    }

    #[rstest]
    #[case::page_opts_out_of_config_level_label_case(LocatorType::Page, "12", "p. 12")]
    #[case::section_takes_config_level_label_case(LocatorType::Section, "12", "Section 12")]
    fn given_a_config_level_label_case_with_a_per_kind_opt_out_when_rendered_then_only_the_opted_out_kind_is_unaffected(
        #[case] kind: LocatorType,
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        // APA (docs/specs/LOCATOR_RENDERING.md, "Label Case and Attachment"):
        // every kind gets a long, capitalized label except page (and
        // paragraph), which stay short and as-is.
        let mut kinds = std::collections::HashMap::new();
        kinds.insert(
            LocatorType::Page,
            citum_schema::options::LocatorKindConfig {
                label_form: Some(LabelForm::Short),
                label_case: Some(TextCase::AsIs),
                ..Default::default()
            },
        );
        let config = LocatorConfig {
            default_label_form: LabelForm::Long,
            label_case: Some(TextCase::CapitalizeFirst),
            kinds,
            ..Default::default()
        };
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  page:
    short:
      singular: "p."
      plural: "pp."
  section:
    long:
      singular: "section"
      plural: "sections"
"#,
        )
        .expect("custom locale should parse");
        let locator = CitationLocator::Single(LocatorSegment {
            label: kind,
            value: LocatorValue::Text(value.to_string()),
        });

        assert_eq!(
            render_locator(&locator, "book", &config, &locale, None, None),
            expected
        );
    }
}
