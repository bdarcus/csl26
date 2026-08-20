/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::grouping::{
    GroupSort, GroupSortEntry, GroupSortKey, SortKey as GroupSortKeyType,
};
use citum_schema::options::{
    ArticleJournalBibliographyConfig, ArticleJournalNoPageFallback, BibliographyConfig,
    SecondFieldAlign, Sort, SortKey, SortSpec, SubsequentAuthorSubstituteRule,
};
use citum_schema::presets::SortPreset;
use citum_schema::template::DelimiterPunctuation;
use csl_legacy::model::{Choose, ChooseBranch, CslNode, Layout, Macro, Sort as LegacySort, Style};

/// Extracts bibliography configuration options from a CSL style.
///
/// Collects settings for subsequent author substitution, hanging indent,
/// entry suffix, and separator punctuation from the bibliography element.
#[must_use]
pub fn extract_bibliography_config(style: &Style) -> Option<BibliographyConfig> {
    let bib = style.bibliography.as_ref()?;

    let mut config = BibliographyConfig::default();
    let mut has_config = false;

    if let Some(sub) = &bib.subsequent_author_substitute {
        config.subsequent_author_substitute = Some(sub.clone());
        has_config = true;
    }

    if let Some(rule) = &bib.subsequent_author_substitute_rule {
        config.subsequent_author_substitute_rule = match rule.as_str() {
            "complete-all" => Some(SubsequentAuthorSubstituteRule::CompleteAll),
            "complete-each" => Some(SubsequentAuthorSubstituteRule::CompleteEach),
            "partial-each" => Some(SubsequentAuthorSubstituteRule::PartialEach),
            "partial-first" => Some(SubsequentAuthorSubstituteRule::PartialFirst),
            _ => Some(SubsequentAuthorSubstituteRule::CompleteAll),
        };
        has_config = true;
    }

    if let Some(hanging) = bib.hanging_indent {
        config.hanging_indent = Some(hanging);
        has_config = true;
    }

    // Extract `second-field-align` (e.g. from `<bibliography
    // second-field-align="flush">`). Unrecognized values are dropped rather
    // than defaulted, matching `subsequent_author_substitute_rule`'s
    // fallback-to-default policy above being the exception, not the norm —
    // an unknown alignment value is more likely a CSL authoring mistake than
    // a value this converter should silently coerce. See
    // `docs/specs/SECOND_FIELD_ALIGN.md`.
    if let Some(align) = &bib.second_field_align {
        config.second_field_align = match align.as_str() {
            "flush" => Some(SecondFieldAlign::Flush),
            "margin" => Some(SecondFieldAlign::Margin),
            _ => None,
        };
        has_config = has_config || config.second_field_align.is_some();
    }

    // Extract layout suffix (e.g., "." from `<layout suffix=".">`).
    if let Some(suffix) = &bib.layout.suffix {
        config.entry_suffix = Some(suffix.clone().into());
        has_config = true;
    }

    // Extract bibliography component separator from group delimiter. Flattened
    // to a literal `Custom` string rather than kept as the semantic mark
    // `from_csl_string` may have resolved it to (e.g. a literal CSL "," to
    // `DelimiterPunctuation::Comma`): migrated output must stay script-invariant
    // like every other migrated style, not silently start realizing full-width
    // under a later `realization-default: cjk` — see
    // `docs/specs/PUNCTUATION_REALIZATION.md` §7's literal-punctuation
    // compatibility contract.
    if let Some(separator) = extract_bibliography_separator_from_layout(&bib.layout, &style.macros)
    {
        config.separator = Some(DelimiterPunctuation::Custom(
            separator.to_string_with_space(),
        ));
        has_config = true;
    }

    // Detect if style wants to suppress period after URLs.
    if should_suppress_period_after_url(style, &bib.layout) {
        config.suppress_period_after_url = true;
        has_config = true;
    }

    if has_article_journal_no_page_doi_fallback(style, &bib.layout) {
        config.article_journal = Some(ArticleJournalBibliographyConfig {
            no_page_fallback: Some(ArticleJournalNoPageFallback::Doi),
            ..Default::default()
        });
        has_config = true;
    }

    // Sort extraction
    if let Some(sort) = &bib.sort
        && let Some(sort_ir) = extract_sort_from_bibliography(sort)
    {
        // Note: BibliographyConfig in citum_schema might not have a sort field if it's handled globally
        // For now, I'll assume it's NOT in BibliographyConfig and should be ignored or moved
        // to global config if necessary. The error said 'sort' is unknown on 'BibliographyConfig'.
        // I'll skip setting it on the config struct but keep the helper.
        let _ = sort_ir;
    }

    if has_config { Some(config) } else { None }
}

/// Determines if periods should be suppressed after URL components.
///
/// Returns true if the CSL style indicates that URL components should not
/// be followed by a period in bibliography output.
#[must_use]
pub fn should_suppress_period_after_url(style: &Style, layout: &Layout) -> bool {
    if layout.suffix.as_ref().is_some_and(|s| !s.is_empty()) {
        return false;
    }

    style_has_doi_without_period(style)
}

fn has_article_journal_no_page_doi_fallback(style: &Style, layout: &Layout) -> bool {
    nodes_have_article_journal_no_page_doi_fallback(&layout.children, &style.macros)
}

fn nodes_have_article_journal_no_page_doi_fallback(nodes: &[CslNode], macros: &[Macro]) -> bool {
    nodes.iter().any(|node| match node {
        CslNode::Choose(choose) => {
            choose_has_article_journal_no_page_doi_fallback(choose, macros)
                || nodes_have_article_journal_no_page_doi_fallback(
                    &choose.if_branch.children,
                    macros,
                )
                || choose.else_if_branches.iter().any(|branch| {
                    nodes_have_article_journal_no_page_doi_fallback(&branch.children, macros)
                })
                || choose.else_branch.as_ref().is_some_and(|branch| {
                    nodes_have_article_journal_no_page_doi_fallback(branch, macros)
                })
        }
        CslNode::Group(group) => {
            nodes_have_article_journal_no_page_doi_fallback(&group.children, macros)
        }
        CslNode::Text(text) => text
            .macro_name
            .as_ref()
            .and_then(|name| macros.iter().find(|macro_def| macro_def.name == *name))
            .is_some_and(|macro_def| {
                nodes_have_article_journal_no_page_doi_fallback(&macro_def.children, macros)
            }),
        CslNode::Names(names) => {
            nodes_have_article_journal_no_page_doi_fallback(&names.children, macros)
        }
        _ => false,
    })
}

fn choose_has_article_journal_no_page_doi_fallback(choose: &Choose, macros: &[Macro]) -> bool {
    branch_targets_article_journal(&choose.if_branch)
        && branch_has_exact_page_to_doi_fallback(&choose.if_branch.children, macros)
        || choose.else_if_branches.iter().any(|branch| {
            branch_targets_article_journal(branch)
                && branch_has_exact_page_to_doi_fallback(&branch.children, macros)
        })
}

fn branch_targets_article_journal(branch: &ChooseBranch) -> bool {
    branch.type_.as_deref().is_some_and(|types| {
        types
            .split_whitespace()
            .any(|item_type| item_type == "article-journal")
    })
}

fn branch_has_exact_page_to_doi_fallback(nodes: &[CslNode], macros: &[Macro]) -> bool {
    nodes.iter().any(|node| match node {
        CslNode::Choose(choose) => {
            is_exact_page_to_doi_fallback(choose, macros)
                || branch_has_exact_page_to_doi_fallback(&choose.if_branch.children, macros)
                || choose
                    .else_if_branches
                    .iter()
                    .any(|branch| branch_has_exact_page_to_doi_fallback(&branch.children, macros))
                || choose
                    .else_branch
                    .as_ref()
                    .is_some_and(|branch| branch_has_exact_page_to_doi_fallback(branch, macros))
        }
        CslNode::Group(group) => branch_has_exact_page_to_doi_fallback(&group.children, macros),
        CslNode::Text(text) => text
            .macro_name
            .as_ref()
            .and_then(|name| macros.iter().find(|macro_def| macro_def.name == *name))
            .is_some_and(|macro_def| {
                branch_has_exact_page_to_doi_fallback(&macro_def.children, macros)
            }),
        _ => false,
    })
}

fn is_exact_page_to_doi_fallback(choose: &Choose, macros: &[Macro]) -> bool {
    choose.else_if_branches.is_empty()
        && branch_is_page_presence_test(&choose.if_branch)
        && branch_renders_page_detail(&choose.if_branch.children, macros)
        && choose
            .else_branch
            .as_ref()
            .is_some_and(|branch| nodes_render_only_doi(branch, macros))
}

fn branch_is_page_presence_test(branch: &ChooseBranch) -> bool {
    branch.variable.as_deref() == Some("page")
        && branch.type_.is_none()
        && branch.is_numeric.is_none()
        && branch.is_uncertain_date.is_none()
        && branch.locator.is_none()
        && branch.position.is_none()
}

fn branch_renders_page_detail(nodes: &[CslNode], macros: &[Macro]) -> bool {
    nodes_reference_variable(nodes, macros, "page")
        && !nodes_reference_variable(nodes, macros, "doi")
        && !nodes_reference_variable(nodes, macros, "url")
}

fn nodes_render_only_doi(nodes: &[CslNode], macros: &[Macro]) -> bool {
    let mut saw_doi = false;
    if !nodes_render_only_doi_inner(nodes, macros, &mut saw_doi) {
        return false;
    }
    saw_doi
}

fn nodes_render_only_doi_inner(nodes: &[CslNode], macros: &[Macro], saw_doi: &mut bool) -> bool {
    nodes.iter().all(|node| match node {
        CslNode::Text(text) => {
            if let Some(variable) = text.variable.as_deref() {
                if variable.eq_ignore_ascii_case("doi") {
                    *saw_doi = true;
                    true
                } else {
                    false
                }
            } else if let Some(name) = text.macro_name.as_deref() {
                macros
                    .iter()
                    .find(|macro_def| macro_def.name == name)
                    .is_some_and(|macro_def| {
                        nodes_render_only_doi_inner(&macro_def.children, macros, saw_doi)
                    })
            } else {
                false
            }
        }
        CslNode::Group(group) => nodes_render_only_doi_inner(&group.children, macros, saw_doi),
        _ => false,
    })
}

fn nodes_reference_variable(nodes: &[CslNode], macros: &[Macro], variable: &str) -> bool {
    nodes.iter().any(|node| match node {
        CslNode::Text(text) => {
            text.variable
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(variable))
                || text
                    .macro_name
                    .as_deref()
                    .and_then(|name| macros.iter().find(|macro_def| macro_def.name == name))
                    .is_some_and(|macro_def| {
                        nodes_reference_variable(&macro_def.children, macros, variable)
                    })
        }
        CslNode::Group(group) => nodes_reference_variable(&group.children, macros, variable),
        CslNode::Choose(choose) => {
            nodes_reference_variable(&choose.if_branch.children, macros, variable)
                || choose
                    .else_if_branches
                    .iter()
                    .any(|branch| nodes_reference_variable(&branch.children, macros, variable))
                || choose
                    .else_branch
                    .as_ref()
                    .is_some_and(|branch| nodes_reference_variable(branch, macros, variable))
        }
        CslNode::Names(names) => nodes_reference_variable(&names.children, macros, variable),
        _ => false,
    })
}

fn style_has_doi_without_period(style: &Style) -> bool {
    for macro_def in &style.macros {
        if macro_has_doi_without_period(macro_def) {
            return true;
        }
    }
    false
}

fn macro_has_doi_without_period(macro_def: &Macro) -> bool {
    nodes_have_doi_without_period(&macro_def.children)
}

fn nodes_have_doi_without_period(nodes: &[CslNode]) -> bool {
    for node in nodes {
        match node {
            CslNode::Text(t)
                if t.variable
                    .as_ref()
                    .is_some_and(|v| v == "doi" || v == "url") =>
            {
                return t.suffix.is_none() || t.suffix.as_ref().is_some_and(|s| !s.contains('.'));
            }
            CslNode::Group(g) if nodes_have_doi_without_period(&g.children) => {
                return true;
            }
            CslNode::Choose(c) => {
                if nodes_have_doi_without_period(&c.if_branch.children) {
                    return true;
                }
                for branch in &c.else_if_branches {
                    if nodes_have_doi_without_period(&branch.children) {
                        return true;
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && nodes_have_doi_without_period(else_branch)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Extract the bibliography component separator from the layout.
///
/// Finds the delimiter that separates bibliography components (e.g., author,
/// title, date, publisher). This should be the delimiter of the DEEPEST group
/// that contains multiple variables, not just the first top-level group.
///
/// For nested structures like:
/// ```xml
/// <layout>
///   <group>  <!-- No delimiter, just wrapping -->
///     <group delimiter=", ">  <!-- This is what we want -->
///       <text variable="title"/>
///       <text variable="publisher"/>
///     </group>
///   </group>
/// </layout>
/// ```
///
/// The extraction should return the inner group's delimiter, not stop at the
/// outer group without one. Also expands macro calls to find delimiters inside
/// referenced macros.
/// Extracts the bibliography separator punctuation from the layout element.
///
/// Parses the delimiter between bibliography entries to determine the
/// appropriate separator character (comma, semicolon, etc.).
#[must_use]
pub fn extract_bibliography_separator_from_layout(
    layout: &Layout,
    macros: &[Macro],
) -> Option<DelimiterPunctuation> {
    // 1. First priority: the top-level layout delimiter if it exists
    if let Some(delim) = &layout.delimiter {
        return Some(DelimiterPunctuation::from_csl_string(delim));
    }

    // 2. Second priority: the delimiter of the FIRST group in the layout
    // (Many styles wrap everything in a top-level group with a delimiter)
    for node in &layout.children {
        if let CslNode::Group(g) = node
            && let Some(delim) = &g.delimiter
        {
            return Some(DelimiterPunctuation::from_csl_string(delim));
        }
    }

    // Helper to count variable-bearing nodes in a group
    fn has_multiple_variables(nodes: &[CslNode]) -> bool {
        let var_count = nodes
            .iter()
            .filter(|node| match node {
                CslNode::Text(t) => t.variable.is_some() || t.macro_name.is_some(),
                CslNode::Names(_) | CslNode::Date(_) => true,
                _ => false,
            })
            .count();
        var_count >= 2
    }

    // 3. Fallback: recursive search for the deepest group with delimiter and multiple variables.
    // Returns (delimiter, depth) to prioritize deeper matches.
    fn find_deepest_group_delimiter(
        nodes: &[CslNode],
        macros: &[Macro],
    ) -> Option<(String, usize)> {
        let mut best: Option<(String, usize)> = None;

        for node in nodes {
            match node {
                CslNode::Group(g) => {
                    // If this group has a delimiter and multiple variables, it's a candidate
                    if let Some(delimiter) = &g.delimiter
                        && has_multiple_variables(&g.children)
                        && best.as_ref().is_none_or(|(_, d)| 1 > *d)
                    {
                        best = Some((delimiter.clone(), 1));
                    }

                    // Recurse into children to find even deeper delimiters
                    if let Some((child_delim, depth)) =
                        find_deepest_group_delimiter(&g.children, macros)
                    {
                        let new_depth = depth + 1;
                        if best.as_ref().is_none_or(|(_, d)| new_depth > *d) {
                            best = Some((child_delim, new_depth));
                        }
                    }
                }
                CslNode::Choose(c) => {
                    // Search all branches of choose blocks
                    if let Some(result) =
                        find_deepest_group_delimiter(&c.if_branch.children, macros)
                        && best.as_ref().is_none_or(|(_, d)| result.1 > *d)
                    {
                        best = Some(result);
                    }
                    for branch in &c.else_if_branches {
                        if let Some(result) = find_deepest_group_delimiter(&branch.children, macros)
                            && best.as_ref().is_none_or(|(_, d)| result.1 > *d)
                        {
                            best = Some(result);
                        }
                    }
                    if let Some(else_branch) = &c.else_branch
                        && let Some(result) = find_deepest_group_delimiter(else_branch, macros)
                        && best.as_ref().is_none_or(|(_, d)| result.1 > *d)
                    {
                        best = Some(result);
                    }
                }
                CslNode::Text(t) => {
                    // Expand macro calls to find delimiters inside
                    if let Some(macro_name) = &t.macro_name
                        && let Some(macro_def) = macros.iter().find(|m| &m.name == macro_name)
                        && let Some((delim, depth)) =
                            find_deepest_group_delimiter(&macro_def.children, macros)
                    {
                        let new_depth = depth + 1;
                        if best.as_ref().is_none_or(|(_, d)| new_depth > *d) {
                            best = Some((delim, new_depth));
                        }
                    }
                }
                _ => {}
            }
        }

        best
    }

    find_deepest_group_delimiter(&layout.children, macros)
        .map(|(d, _)| DelimiterPunctuation::from_csl_string(&d))
}

/// Extracts bibliography sort configuration from a CSL sort element.
///
/// Converts CSL sort keys and order (ascending/descending) to the citum format.
#[must_use]
pub fn extract_sort_from_bibliography(sort: &LegacySort) -> Option<Sort> {
    let mut sort_ir = Sort::default();
    for key in &sort.keys {
        let sort_key = match key.variable.as_deref() {
            Some("author" | "editor") => SortKey::Author,
            Some("issued" | "year") => SortKey::Year,
            Some("title") => SortKey::Title,
            Some("citation-number") => SortKey::CitationNumber,
            _ => continue,
        };

        sort_ir.template.push(SortSpec {
            key: sort_key,
            ascending: key.sort.as_deref() != Some("descending"),
        });
    }

    if sort_ir.template.is_empty() {
        None
    } else {
        Some(sort_ir)
    }
}

/// Extract bibliography sort into the top-level Citum bibliography.sort shape.
///
/// This mapping is used by processor numeric citation-number assignment, where
/// citation numbers follow bibliography order when a sort spec is present.
/// Extracts group sort configuration for multi-group bibliographies.
///
/// Converts CSL sort keys with grouping context to citum `GroupSort` format.
#[must_use]
pub fn extract_group_sort_from_bibliography(sort: &LegacySort) -> Option<GroupSortEntry> {
    let template = deduplicate_group_sort_keys(
        sort.keys
            .iter()
            .filter_map(|key| {
                let key_kind = key
                    .variable
                    .as_ref()
                    .and_then(|name| parse_group_sort_key(name))
                    .or_else(|| {
                        key.macro_name
                            .as_ref()
                            .and_then(|name| parse_group_sort_key(name))
                    })?;

                Some(GroupSortKey {
                    key: key_kind,
                    ascending: key.sort.as_deref() != Some("descending"),
                    order: None,
                    sort_order: None,
                })
            })
            .collect(),
    );

    if template.is_empty() {
        None
    } else if let Some(preset) = group_sort_preset_for_keys(&template) {
        Some(GroupSortEntry::Preset(preset))
    } else {
        Some(GroupSortEntry::Explicit(GroupSort { template }))
    }
}

fn deduplicate_group_sort_keys(template: Vec<GroupSortKey>) -> Vec<GroupSortKey> {
    let mut deduplicated = Vec::new();

    for key in template {
        if let Some(existing) = deduplicated
            .iter_mut()
            .find(|existing: &&mut GroupSortKey| existing.key == key.key)
        {
            existing.ascending &= key.ascending;
            continue;
        }
        deduplicated.push(key);
    }

    deduplicated
}

fn group_sort_preset_for_keys(template: &[GroupSortKey]) -> Option<SortPreset> {
    if template.iter().any(|key| !key.ascending) {
        return None;
    }

    let keys: Vec<&GroupSortKeyType> = template.iter().map(|key| &key.key).collect();
    match keys.as_slice() {
        [GroupSortKeyType::Author]
        | [GroupSortKeyType::Author, GroupSortKeyType::Issued]
        | [
            GroupSortKeyType::Author,
            GroupSortKeyType::Issued,
            GroupSortKeyType::Title,
        ] => Some(SortPreset::AuthorDateTitle),
        [GroupSortKeyType::Author, GroupSortKeyType::Title]
        | [
            GroupSortKeyType::Author,
            GroupSortKeyType::Title,
            GroupSortKeyType::Issued,
        ] => Some(SortPreset::AuthorTitleDate),
        _ => None,
    }
}

fn parse_group_sort_key(name: &str) -> Option<GroupSortKeyType> {
    let lowered = name.to_ascii_lowercase();

    if lowered == "author"
        || lowered.contains("author")
        || lowered == "editor"
        || lowered.contains("editor")
    {
        Some(GroupSortKeyType::Author)
    } else if lowered == "issued"
        || lowered == "year"
        || lowered.contains("year")
        || lowered.contains("date")
    {
        Some(GroupSortKeyType::Issued)
    } else if lowered == "title" || lowered.contains("title") {
        Some(GroupSortKeyType::Title)
    } else if lowered == "type" {
        Some(GroupSortKeyType::RefType)
    } else {
        None
    }
}
