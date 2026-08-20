/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::options::{
    GivennameRule, Group, LabelConfig, LabelPreset, Processing, ProcessingBase, ProcessingCustom,
    Sort, SortEntry, SortKey, SortSpec,
};
use citum_schema::presets::SortPreset;
use csl_legacy::model::{CslNode, Style};
use std::collections::HashSet;

/// Detects the citation processing mode from a CSL style.
///
/// Analyzes style attributes and layout patterns to determine if the style
/// uses author-date, numeric, note-based, or label-based citation processing.
pub fn detect_processing_mode(style: &Style) -> Option<Processing> {
    // 0. Label (trigraph) styles render the generated `citation-label`
    // variable (e.g. `[Kuhn62]`). The engine only emits citation labels under
    // `Processing::Label`, so the mode must be detected here or the label
    // renders empty.
    fn has_citation_label(nodes: &[csl_legacy::model::CslNode]) -> bool {
        use csl_legacy::model::CslNode;
        nodes.iter().any(|node| match node {
            CslNode::Text(t) => t.variable.as_deref() == Some("citation-label"),
            CslNode::Group(g) => has_citation_label(&g.children),
            _ => false,
        })
    }

    if has_citation_label(&style.citation.layout.children) {
        return Some(Processing::Label(LabelConfig {
            preset: LabelPreset::Ams,
            ..Default::default()
        }));
    }

    // 0b. Note styles are explicit in CSL and should map directly when they
    // do not render generated citation labels.
    if style.class == "note" {
        return Some(Processing::Note);
    }

    // 1. Explicitly numeric style. `second-field-align` is now parsed
    // (`csl_legacy::model::Bibliography::second_field_align`,
    // `docs/specs/SECOND_FIELD_ALIGN.md`) but is not used as a numeric-mode
    // signal here: it is a layout attribute, not proof a style is numeric —
    // an author-date style could in principle declare it too. Citation-number
    // presence in the layout stays the numeric-mode heuristic.

    // Helper to recursively search for citation-number in layout nodes
    fn has_citation_number(nodes: &[csl_legacy::model::CslNode]) -> bool {
        use csl_legacy::model::CslNode;
        nodes.iter().any(|node| match node {
            CslNode::Number(n) => n.variable == "citation-number",
            CslNode::Group(g) => has_citation_number(&g.children),
            CslNode::Text(t) if t.variable.as_deref() == Some("citation-number") => true,
            _ => false,
        })
    }

    let is_numeric =
        style.class == "in-text" && has_citation_number(&style.citation.layout.children);

    if is_numeric {
        return Some(Processing::Numeric);
    }

    // 2. Author-date style
    // Some styles hide date/year logic in nested macro trees. Follow macro calls
    // recursively so we don't miss author-date processing config extraction.
    let mut visited_macros = HashSet::new();
    let is_author_date =
        nodes_have_author_date_signal(&style.citation.layout.children, style, &mut visited_macros);

    if is_author_date {
        let overrides = ProcessingOverrides::from_style(style);
        return Some(fold_to_named_processing(&overrides));
    }

    None
}

/// Sparse CSL processing overrides for the author-date family.
///
/// Every field is `None` when the CSL never stated the corresponding
/// attribute, as opposed to eagerly materializing a default value. Folding
/// (see [`fold_to_named_processing`]) applies these overrides on top of a
/// disambiguation-nearest named preset instead of comparing a fully
/// materialized struct against every preset, so an absent attribute never
/// clobbers the chosen preset's own default.
struct ProcessingOverrides {
    sort: Option<SortEntry>,
    group: Option<Group>,
    disamb_names: Option<bool>,
    disamb_add_givenname: Option<bool>,
    disamb_year_suffix: Option<bool>,
    givenname_rule: Option<GivennameRule>,
}

impl ProcessingOverrides {
    fn from_style(style: &Style) -> Self {
        let sort = style.citation.sort.as_ref().and_then(extract_sort);
        // Preset sorts already carry their canonical group via the base
        // config; only derive group from explicit (non-preset) sort keys.
        let group = match sort.as_ref() {
            Some(SortEntry::Explicit(explicit_sort)) => extract_group_from_sort(explicit_sort),
            _ => None,
        };
        let givenname_rule = style
            .citation
            .disambiguate_givenname_rule
            .as_deref()
            .map(|rule| match rule {
                "primary-name" => GivennameRule::PrimaryName,
                "primary-name-with-initials" => GivennameRule::PrimaryNameWithInitials,
                "all-names-with-initials" => GivennameRule::AllNamesWithInitials,
                "all-names" => GivennameRule::AllNames,
                _ => GivennameRule::ByCite,
            });

        Self {
            sort,
            group,
            disamb_names: style.citation.disambiguate_add_names,
            disamb_add_givenname: style.citation.disambiguate_add_givenname,
            disamb_year_suffix: style.citation.disambiguate_add_year_suffix,
            givenname_rule,
        }
    }

    /// Disambiguation-nearest named author-date preset for the explicit
    /// names/given-name signal, per the folding table in
    /// `docs/reference/PROCESSING_MIGRATION.md`.
    fn author_date_preset(&self) -> ProcessingBase {
        match (
            self.disamb_names.unwrap_or(false),
            self.disamb_add_givenname.unwrap_or(false),
        ) {
            (false, false) => ProcessingBase::AuthorDate,
            (false, true) => ProcessingBase::AuthorDateGivenname,
            (true, false) => ProcessingBase::AuthorDateNames,
            (true, true) => ProcessingBase::AuthorDateFull,
        }
    }

    /// Overlay the remaining explicit overrides (sort/group/year-suffix/
    /// givenname-rule) onto a preset-seeded config. Names/given-name are not
    /// re-applied here: [`author_date_preset`](Self::author_date_preset)
    /// already selected a preset whose canonical values match them.
    fn apply(&self, mut custom: ProcessingCustom) -> ProcessingCustom {
        if let Some(sort) = self.sort.clone() {
            custom.sort = Some(sort);
        }
        if self.group.is_some() {
            custom.group = self.group.clone();
        }
        if let Some(disamb) = custom.disambiguate.as_mut() {
            if let Some(year_suffix) = self.disamb_year_suffix {
                disamb.year_suffix = year_suffix;
            }
            if let Some(givenname_rule) = self.givenname_rule.clone() {
                disamb.givenname_rule = givenname_rule;
            }
        }
        custom
    }
}

/// Fold sparse processing overrides onto the disambiguation-nearest named
/// preset, falling back to `Processing::Custom` only when the remaining
/// overrides (sort/group/year-suffix/givenname-rule) diverge from that
/// preset's canonical config. Keeps the migrated YAML idiomatic
/// (`processing: author-date`) instead of dumping a custom block for
/// styles that never stated a divergent attribute.
///
/// The custom fallback is emitted as a delta: `base:` names the chosen
/// preset and only the fields that diverge from its config are written,
/// so the YAML never materializes defaults the CSL never stated.
fn fold_to_named_processing(overrides: &ProcessingOverrides) -> Processing {
    let base = overrides.author_date_preset();
    let preset = base.processing();
    let base_config = preset.config();
    let custom = overrides.apply(base_config.clone());
    if custom == base_config {
        return preset;
    }
    Processing::Custom(ProcessingCustom {
        base: Some(base),
        sort: (custom.sort != base_config.sort)
            .then_some(custom.sort)
            .flatten(),
        group: (custom.group != base_config.group)
            .then_some(custom.group)
            .flatten(),
        disambiguate: (custom.disambiguate != base_config.disambiguate)
            .then_some(custom.disambiguate)
            .flatten(),
    })
}

fn nodes_have_author_date_signal(
    nodes: &[CslNode],
    style: &Style,
    visited_macros: &mut HashSet<String>,
) -> bool {
    nodes
        .iter()
        .any(|node| node_has_author_date_signal(node, style, visited_macros))
}

fn node_has_author_date_signal(
    node: &CslNode,
    style: &Style,
    visited_macros: &mut HashSet<String>,
) -> bool {
    match node {
        CslNode::Date(_) => true,
        CslNode::Text(t) => {
            if t.variable.as_deref().is_some_and(|v| {
                matches!(
                    v,
                    "issued" | "original-date" | "event-date" | "accessed" | "year-suffix"
                )
            }) {
                return true;
            }

            if let Some(macro_name) = &t.macro_name {
                let lowered = macro_name.to_ascii_lowercase();
                if lowered.contains("year") || lowered.contains("date") {
                    return true;
                }

                if visited_macros.insert(macro_name.clone())
                    && let Some(macro_def) = style.macros.iter().find(|m| m.name == *macro_name)
                    && nodes_have_author_date_signal(&macro_def.children, style, visited_macros)
                {
                    return true;
                }
            }

            false
        }
        CslNode::Group(g) => nodes_have_author_date_signal(&g.children, style, visited_macros),
        CslNode::Choose(c) => {
            nodes_have_author_date_signal(&c.if_branch.children, style, visited_macros)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_have_author_date_signal(&b.children, style, visited_macros))
                || c.else_branch.as_ref().is_some_and(|nodes| {
                    nodes_have_author_date_signal(nodes, style, visited_macros)
                })
        }
        CslNode::Names(n) => nodes_have_author_date_signal(&n.children, style, visited_macros),
        _ => false,
    }
}

fn extract_sort(legacy_sort: &csl_legacy::model::Sort) -> Option<SortEntry> {
    let template = deduplicate_sort_specs(
        legacy_sort
            .keys
            .iter()
            .filter_map(|key| {
                let key_kind = key
                    .variable
                    .as_ref()
                    .and_then(|name| parse_sort_key(name))
                    .or_else(|| {
                        key.macro_name
                            .as_ref()
                            .and_then(|name| parse_sort_key(name))
                    })?;

                let ascending = key.sort.as_deref() != Some("descending");
                Some(SortSpec {
                    key: key_kind,
                    ascending,
                })
            })
            .collect(),
    );

    if template.is_empty() {
        None
    } else if let Some(preset) = sort_preset_for_specs(&template) {
        Some(SortEntry::Preset(preset))
    } else {
        Some(SortEntry::Explicit(Sort {
            shorten_names: false,
            render_substitutions: false,
            template,
        }))
    }
}

fn deduplicate_sort_specs(template: Vec<SortSpec>) -> Vec<SortSpec> {
    let mut deduplicated = Vec::new();

    for spec in template {
        if let Some(existing) = deduplicated
            .iter_mut()
            .find(|existing: &&mut SortSpec| existing.key == spec.key)
        {
            existing.ascending &= spec.ascending;
            continue;
        }
        deduplicated.push(spec);
    }

    deduplicated
}

fn sort_preset_for_specs(template: &[SortSpec]) -> Option<SortPreset> {
    if template.iter().any(|spec| !spec.ascending) {
        return None;
    }

    let keys: Vec<&SortKey> = template.iter().map(|spec| &spec.key).collect();
    match keys.as_slice() {
        [SortKey::Author]
        | [SortKey::Author, SortKey::Year]
        | [SortKey::Author, SortKey::Year, SortKey::Title] => Some(SortPreset::AuthorDateTitle),
        [SortKey::Author, SortKey::Title] | [SortKey::Author, SortKey::Title, SortKey::Year] => {
            Some(SortPreset::AuthorTitleDate)
        }
        [SortKey::CitationNumber] => Some(SortPreset::CitationNumber),
        _ => None,
    }
}

fn extract_group_from_sort(sort: &Sort) -> Option<Group> {
    let mut keys: Vec<SortKey> = Vec::new();

    for spec in &sort.template {
        match spec.key {
            SortKey::Author | SortKey::Year | SortKey::Title if !keys.contains(&spec.key) => {
                keys.push(spec.key.clone());
            }
            SortKey::CitationNumber => {}
            _ => {}
        }
    }

    if keys.is_empty() {
        None
    } else {
        Some(Group { template: keys })
    }
}

fn parse_sort_key(name: &str) -> Option<SortKey> {
    let lowered = name.to_ascii_lowercase();

    if lowered == "citation-number" || lowered.contains("citation-number") {
        Some(SortKey::CitationNumber)
    } else if lowered == "author" || lowered.contains("author") {
        Some(SortKey::Author)
    } else if lowered == "issued"
        || lowered == "year"
        || lowered.contains("year")
        || lowered.contains("date")
    {
        Some(SortKey::Year)
    } else if lowered == "title" || lowered.contains("title") {
        Some(SortKey::Title)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use roxmltree::Document;
    use rstest::rstest;

    fn parse(xml: &str) -> Style {
        let doc = Document::parse(xml).expect("test style XML should parse");
        csl_legacy::parser::parse_style(doc.root_element()).expect("legacy style should parse")
    }

    fn style_with_citation_layout(class: &str, layout_body: &str) -> Style {
        parse(&format!(
            r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="{class}">
  <info><title>t</title><id>https://example.org/t</id></info>
  <citation><layout prefix="[" suffix="]">{layout_body}</layout></citation>
</style>"#
        ))
    }

    /// An author-date-signaling citation (via `<date variable="issued"/>`)
    /// with the given citation-level attributes and an optional `<sort>`
    /// body, for exercising the disambiguation folding table.
    fn author_date_style(citation_attrs: &str, sort_body: &str) -> Style {
        parse(&format!(
            r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>t</title><id>https://example.org/t</id></info>
  <citation {citation_attrs}>
    {sort_body}
    <layout prefix="[" suffix="]"><date variable="issued"/></layout>
  </citation>
</style>"#
        ))
    }

    #[test]
    fn detects_label_mode_from_citation_label_variable() {
        // given an in-text style whose citation renders the generated label
        let style = style_with_citation_layout("in-text", r#"<text variable="citation-label"/>"#);

        // when the processing mode is detected
        let mode = detect_processing_mode(&style);

        // then it is Label with CSL-compatible label parameters
        assert!(
            matches!(mode, Some(Processing::Label(_))),
            "expected Processing::Label, got: {mode:?}"
        );
        if let Some(Processing::Label(config)) = mode {
            let params = config.effective_params();
            assert_eq!(params.single_author_chars, 4);
            assert_eq!(params.et_al_min, 5);
            assert_eq!(params.et_al_marker, "");
            assert_eq!(params.et_al_names, 4);
        }
    }

    #[test]
    fn detects_label_mode_before_note_mode() {
        // given a note style whose citation renders the generated label
        let style = style_with_citation_layout("note", r#"<text variable="citation-label"/>"#);

        // then label processing wins because note processing cannot emit citation-label
        let mode = detect_processing_mode(&style);
        assert!(
            matches!(mode, Some(Processing::Label(_))),
            "expected Processing::Label, got: {mode:?}"
        );
    }

    #[test]
    fn citation_number_style_is_not_misdetected_as_label() {
        // given a numeric style (no citation-label), Label must not steal it
        let style = style_with_citation_layout("in-text", r#"<text variable="citation-number"/>"#);

        let mode = detect_processing_mode(&style);

        assert!(
            matches!(mode, Some(Processing::Numeric)),
            "expected Processing::Numeric, got: {mode:?}"
        );
    }

    #[rstest]
    #[case::bare("", Processing::AuthorDate)]
    #[case::givenname(
        r#"disambiguate-add-givenname="true""#,
        Processing::AuthorDateGivenname
    )]
    #[case::names(r#"disambiguate-add-names="true""#, Processing::AuthorDateNames)]
    #[case::full(
        r#"disambiguate-add-names="true" disambiguate-add-givenname="true""#,
        Processing::AuthorDateFull
    )]
    fn folds_disambiguation_signal_to_named_preset(
        #[case] citation_attrs: &str,
        #[case] expected: Processing,
    ) {
        // given a style whose only explicit signal is the disambiguation
        // attribute table row, with no other divergent attribute
        let style = author_date_style(citation_attrs, "");

        // when processing mode is detected
        let mode = detect_processing_mode(&style);

        // then it folds to the disambiguation-nearest named preset, per
        // docs/reference/PROCESSING_MIGRATION.md's folding table
        assert_eq!(mode, Some(expected));
    }

    #[test]
    fn absent_givenname_rule_does_not_clobber_the_chosen_presets_default() {
        // given an author-date-full-shaped style (names + given-name signal)
        // that never states `givenname-disambiguation-rule` explicitly
        let style = author_date_style(
            r#"disambiguate-add-names="true" disambiguate-add-givenname="true""#,
            "",
        );

        // when processing mode is detected
        let mode = detect_processing_mode(&style);

        // then it still folds to the named preset: AuthorDateFull's own
        // canonical givenname_rule (PrimaryName) is not overwritten back to
        // GivennameRule::default() just because the CSL never stated it
        assert_eq!(mode, Some(Processing::AuthorDateFull));
    }

    #[test]
    #[allow(clippy::panic, reason = "Panicking is acceptable in tests.")]
    fn explicit_non_preset_sort_diverges_to_custom_without_materializing_defaults() {
        // given a bare author-date style with an explicit, non-preset sort
        // (a single `title` key, which does not match the author-date-title
        // preset's canonical multi-key spec)
        let style = author_date_style("", "<sort><key variable=\"title\"/></sort>");

        // when processing mode is detected
        let mode = detect_processing_mode(&style);

        // then it falls to a Custom delta on the author-date base carrying
        // only the divergent sort (and its derived group): disambiguation is
        // not materialized in the emitted config at all
        let Some(Processing::Custom(custom)) = mode else {
            panic!("expected Processing::Custom, got: {mode:?}");
        };
        assert_eq!(custom.base, Some(ProcessingBase::AuthorDate));
        assert_eq!(custom.disambiguate, None);
        assert_eq!(
            custom.group,
            Some(Group {
                template: vec![SortKey::Title]
            })
        );
        assert_ne!(custom.sort, None);
        assert_ne!(custom.sort, Processing::AuthorDate.config().sort);

        // and the resolved config still inherits the preset's disambiguation
        assert_eq!(
            custom.resolved().disambiguate,
            Processing::AuthorDate.config().disambiguate
        );
    }

    #[test]
    #[allow(clippy::panic, reason = "Panicking is acceptable in tests.")]
    fn explicit_year_suffix_false_emits_base_delta_with_disambiguation_only() {
        // given an author-date style that contradicts the class default with
        // an explicit disambiguate-add-year-suffix="false"
        let style = author_date_style(r#"disambiguate-add-year-suffix="false""#, "");

        // when processing mode is detected
        let mode = detect_processing_mode(&style);

        // then it emits a delta on the author-date base whose only override
        // is the diverging disambiguation block; sort/group stay inherited
        let Some(Processing::Custom(custom)) = mode else {
            panic!("expected Processing::Custom, got: {mode:?}");
        };
        assert_eq!(custom.base, Some(ProcessingBase::AuthorDate));
        assert_eq!(custom.sort, None);
        assert_eq!(custom.group, None);
        let disamb = custom
            .disambiguate
            .as_ref()
            .unwrap_or_else(|| panic!("expected diverging disambiguation override"));
        assert!(!disamb.year_suffix);
    }
}
