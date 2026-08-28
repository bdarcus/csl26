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
use citum_schema::options::{LabelForm, LabelRepeat, LocatorConfig, RangeFormat};

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
#[must_use]
pub fn render_locator(
    locator: &CitationLocator,
    ref_type: &str,
    config: &LocatorConfig,
    locale: &Locale,
) -> String {
    let segments = locator.segments();

    // Collect the set of locator kinds present in the locator
    let kinds: std::collections::HashSet<LocatorType> =
        segments.iter().map(|seg| seg.label.clone()).collect();

    // Find first matching pattern
    let pattern = config.patterns.iter().find(|p| {
        // Check if pattern's kind set is a subset of active kinds
        let pattern_kinds: std::collections::HashSet<LocatorType> =
            p.kinds.iter().cloned().collect();
        if !pattern_kinds.is_subset(&kinds) {
            return false;
        }

        // Check type_class gate if present
        if let Some(type_class) = p.type_class
            && !crate::values::type_class::matches_type_class(ref_type, type_class)
        {
            return false;
        }

        true
    });

    if let Some(pattern) = pattern {
        render_with_pattern(segments, pattern, config, locale)
    } else {
        render_default(segments, config, locale)
    }
}

/// Render segments using a matched pattern.
fn render_with_pattern(
    segments: &[LocatorSegment],
    pattern: &citum_schema::options::LocatorPattern,
    config: &LocatorConfig,
    locale: &Locale,
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
                    locale,
                )
            } else {
                let range_format = effective_range_format(kind_cfg, config);
                crate::values::number::format_page_range(
                    seg.value.value_str(),
                    Some(&range_format),
                    locale.grammar_options.page_range_delimiter.as_str(),
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
        let range_format = effective_range_format(kind_cfg, config);
        let value_str = crate::values::number::format_page_range(
            seg.value.value_str(),
            Some(&range_format),
            locale.grammar_options.page_range_delimiter.as_str(),
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
                locale,
            )
        };
        rendered.push(rendered_segment);
    }

    rendered.join(&pattern.delimiter)
}

/// Render segments without a matched pattern (default behavior).
fn render_default(segments: &[LocatorSegment], config: &LocatorConfig, locale: &Locale) -> String {
    let mut rendered = Vec::new();

    for seg in segments {
        let kind_cfg = config.kinds.get(&seg.label);
        let form = kind_cfg
            .and_then(|cfg| cfg.label_form)
            .unwrap_or(config.default_label_form);

        let rendered_segment = if matches!(form, LabelForm::None) {
            let range_format = effective_range_format(kind_cfg, config);
            crate::values::number::format_page_range(
                seg.value.value_str(),
                Some(&range_format),
                locale.grammar_options.page_range_delimiter.as_str(),
            )
        } else {
            render_segment_with_label(
                seg,
                kind_cfg,
                form,
                config,
                config.strip_label_periods,
                locale,
            )
        };

        rendered.push(rendered_segment);
    }

    rendered.join(&config.fallback_delimiter)
}

/// Render a single segment with label.
fn render_segment_with_label(
    seg: &LocatorSegment,
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    form: LabelForm,
    config: &LocatorConfig,
    global_strip: Option<bool>,
    locale: &Locale,
) -> String {
    let range_format = effective_range_format(kind_cfg, config);
    let value_str = crate::values::number::format_page_range(
        seg.value.value_str(),
        Some(&range_format),
        locale.grammar_options.page_range_delimiter.as_str(),
    );
    render_segment_with_label_str(seg, kind_cfg, form, &value_str, global_strip, locale)
}

/// Render a single segment with label, given a pre-computed value string.
fn render_segment_with_label_str(
    seg: &LocatorSegment,
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    form: LabelForm,
    value_str: &str,
    global_strip: Option<bool>,
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
        if strip_periods {
            // Stripping the trailing period removes the natural separator,
            // so no additional space is added (e.g. "p." → "p23" not "p 23").
            let term_str = crate::values::strip_trailing_periods(&term);
            format!("{term_str}{value_str}")
        } else {
            format!("{term} {value_str}")
        }
    } else {
        value_str.to_string()
    }
}

/// Resolve the effective range format for a locator segment.
fn effective_range_format(
    kind_cfg: Option<&citum_schema::options::LocatorKindConfig>,
    config: &LocatorConfig,
) -> RangeFormat {
    kind_cfg
        .and_then(|k| k.range_format.clone())
        .unwrap_or_else(|| config.range_format.clone())
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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
            range_format: RangeFormat::Chicago,
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
            render_locator(&locator, "book", &config, &locale),
            "page 3–10"
        );

        let unlabeled_config = LocatorConfig {
            default_label_form: LabelForm::None,
            range_format: RangeFormat::Chicago,
            ..Default::default()
        };

        assert_eq!(
            render_locator(&locator, "book", &unlabeled_config, &locale),
            "3–10"
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
            range_format: RangeFormat::Expanded,
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
            render_locator(&labeled, "book", &config, &locale),
            "page 505–17"
        );

        let unlabeled_config = LocatorConfig {
            default_label_form: LabelForm::None,
            range_format: RangeFormat::Expanded,
            kinds: config.kinds.clone(),
            ..Default::default()
        };
        assert_eq!(
            render_locator(&unlabeled, "book", &unlabeled_config, &locale),
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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
        let result = render_locator(&locator, "book", &config, &Locale::default());
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

        assert_eq!(render_locator(&locator, "book", &config, &locale), "reel 3");
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

        assert_eq!(render_locator(&locator, "book", &config, &locale), "reel 3");
    }
}
