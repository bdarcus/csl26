/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Author-group formation for grouped citation rendering.

use crate::reference::Reference;

/// Group citation items by author, respecting individual citation hints and
/// the style's `citation.collapse` setting.
///
/// Returns a list of (`author_key`, items) tuples. Same-author collapse only
/// merges adjacent same-author items when `collapse` is
/// `Some(CitationCollapse::SameAuthor(_))` — see
/// `docs/specs/SAME_AUTHOR_COLLAPSE.md` §3. When it isn't (or any item has
/// disambiguation hints, as before), every item becomes its own singleton
/// group, keyed on `(index, id)` so two citation items sharing the same
/// reference id in one cluster can never spuriously merge (§11).
pub(crate) fn group_citation_items_by_author<'a>(
    renderer: &super::super::Renderer<'_>,
    items: &'a [crate::reference::CitationItem],
    collapse: Option<&citum_schema::CitationCollapse>,
) -> Vec<(String, Vec<&'a crate::reference::CitationItem>)> {
    let same_author_collapse = matches!(
        collapse,
        Some(citum_schema::CitationCollapse::SameAuthor(_))
    );
    let preserve_individual_citations = !same_author_collapse
        || items.iter().any(|item| {
            renderer
                .hints
                .get(&item.id)
                .is_some_and(|hints| hints.min_names_to_show.is_some() || hints.expand_given_names)
        });

    let mut groups: Vec<(String, Vec<&'a crate::reference::CitationItem>)> = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let reference = renderer.bibliography.get(&item.id);
        let author_key = if preserve_individual_citations {
            format!("{index}\u{1f}{}", item.id)
        } else {
            reference.map(author_grouping_key).unwrap_or_default()
        };

        match groups.last_mut() {
            Some(group) if !author_key.is_empty() && group.0 == author_key => {
                group.1.push(item);
            }
            _ => groups.push((author_key, vec![item])),
        }
    }

    groups
}

/// Generate a grouping key for a reference.
///
/// Prefers author, then editor, then title. The key is lowercased for
/// case-insensitive grouping.
fn author_grouping_key(reference: &Reference) -> String {
    reference
        .author()
        .map_or_else(
            || {
                reference.editor().map_or_else(
                    || {
                        reference
                            .title()
                            .map_or_else(String::new, |title| title.to_string())
                    },
                    |editor| editor.to_string(),
                )
            },
            |author| author.to_string(),
        )
        .to_lowercase()
}
