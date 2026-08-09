/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Shared semantic representation for CSL-to-Citum comparison.
//!
//! Used by [`crate::profile_discovery`] (auditing known candidates) and
//! [`crate::coverage_gap`] (corpus-wide gap analysis). Both modules reduce
//! compiled templates to sets of [`SemanticItem`]s and compare them via
//! Jaccard similarity or feature-key set arithmetic.

use std::collections::HashSet;

use citum_schema::StyleBase;
use citum_schema::locale::GeneralTerm;
use citum_schema::template::{
    ContributorRole, DateVariable, NumberVariable, SimpleVariable, TemplateComponent,
    TemplateVariant, TitleType,
};

/// A simplified semantic representation of a compiled template component.
///
/// Structural wrappers (Conditional, Substitute, Separator, Layout, etc.) are
/// intentionally omitted; only content-bearing items are captured.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum SemanticItem {
    /// A text-variable reference (non-title, non-numeric).
    Variable(SimpleVariable),
    /// A numeric variable reference.
    Number(NumberVariable),
    /// A date variable reference.
    Date(DateVariable),
    /// A contributor-role reference (`<names>` element).
    Contributor(ContributorRole),
    /// A title reference (typed).
    Title(TitleType),
    /// A locale term reference.
    Term(GeneralTerm),
}

/// Convert a `SemanticItem` to a CSL-compatible feature key.
///
/// The key format mirrors what [`crate::coverage_gap::collect_legacy_features`]
/// produces from the raw CSL source:
/// `"var:title"`, `"num:volume"`, `"date:issued"`, `"names:author"`, `"term:and"`.
///
/// Several Citum schema names diverge from their CSL 1.0 counterparts:
/// - [`TitleType`] variants (`Primary`, `ParentMonograph`, …) are reverse-mapped to
///   their CSL variable names (`title`, `container-title`, `collection-title`).
/// - [`DateVariable::OriginalPublished`] is reverse-mapped to CSL's
///   `original-date` variable.
/// - [`SimpleVariable::Doi`] / [`SimpleVariable::Url`] use uppercase in CSL (`DOI`,
///   `URL`) but lowercase serde names in Citum.
/// - [`SimpleVariable::ArchiveLocation`] uses an underscore in CSL (`archive_location`)
///   but a hyphen in Citum serde (`archive-location`).
pub fn semantic_to_legacy_key(item: &SemanticItem) -> String {
    fn serialize<T: serde::Serialize>(v: &T) -> String {
        serde_json::to_string(v)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string()
    }
    match item {
        SemanticItem::Variable(v) => {
            let csl_name = match v {
                SimpleVariable::Doi => "DOI",
                SimpleVariable::Url => "URL",
                SimpleVariable::ArchiveLocation => "archive_location",
                _ => return format!("var:{}", serialize(v)),
            };
            format!("var:{csl_name}")
        }
        SemanticItem::Number(n) => format!("num:{}", serialize(n)),
        SemanticItem::Date(d) => {
            let csl_name = match d {
                DateVariable::OriginalPublished => "original-date",
                _ => return format!("date:{}", serialize(d)),
            };
            format!("date:{csl_name}")
        }
        SemanticItem::Contributor(c) => format!("names:{}", serialize(c)),
        SemanticItem::Title(t) => {
            // TitleType uses Citum-internal names; reverse-map to CSL 1.0 variable names.
            let csl_var = match t {
                TitleType::Primary => "title",
                TitleType::ContainerTitle
                | TitleType::ParentMonograph
                | TitleType::ParentSerial => "container-title",
                TitleType::CollectionTitle => "collection-title",
                _ => return format!("var:{}", serialize(t)),
            };
            format!("var:{csl_var}")
        }
        SemanticItem::Term(t) => format!("term:{}", serialize(t)),
    }
}

/// Recursively collect `SemanticItem`s from a single `TemplateComponent`.
///
/// Structural wrappers (Conditional, Substitute, Separator, Layout, etc.) are
/// traversed but not emitted as items.
pub fn to_semantic_items(component: &TemplateComponent, items: &mut Vec<SemanticItem>) {
    match component {
        TemplateComponent::Variable(v) => items.push(SemanticItem::Variable(v.variable.clone())),
        TemplateComponent::Number(n) => items.push(SemanticItem::Number(n.number.clone())),
        TemplateComponent::Date(d) => items.push(SemanticItem::Date(d.date.clone())),
        TemplateComponent::Contributor(c) => {
            items.extend(
                c.contributor
                    .as_slice()
                    .iter()
                    .cloned()
                    .map(SemanticItem::Contributor),
            );
        }
        TemplateComponent::Title(t) => items.push(SemanticItem::Title(t.title.clone())),
        TemplateComponent::Term(t) => items.push(SemanticItem::Term(t.term.clone())),
        TemplateComponent::Group(g) => {
            for child in &g.group {
                to_semantic_items(child, items);
            }
        }
        _ => {}
    }
}

/// Reduce a template slice to the set of distinct `SemanticItem`s it references.
pub fn template_to_set(template: &[TemplateComponent]) -> HashSet<SemanticItem> {
    let mut items = Vec::new();
    for component in template {
        to_semantic_items(component, &mut items);
    }
    items.into_iter().collect()
}

/// Jaccard similarity between two semantic sets: `|A ∩ B| / |A ∪ B|`.
///
/// Returns `1.0` when both sets are empty (two empty templates are considered identical).
pub fn jaccard_similarity(left: &HashSet<SemanticItem>, right: &HashSet<SemanticItem>) -> f32 {
    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Pre-compute bibliography and citation semantic sets for every `StyleBase`.
///
/// Returns `(base, bib_set, cit_sets)` where `cit_sets` may contain one or more
/// sets from the base's citation template, integral, and non-integral sub-templates.
pub fn collect_base_semantic_sets()
-> Vec<(StyleBase, HashSet<SemanticItem>, Vec<HashSet<SemanticItem>>)> {
    StyleBase::all()
        .iter()
        .map(|base| {
            let style = base.base().into_resolved();
            let bib_set = style
                .bibliography
                .as_ref()
                .and_then(|b| b.template.as_ref())
                .and_then(TemplateVariant::as_template)
                .map(template_to_set)
                .unwrap_or_default();

            let mut cit_sets = Vec::new();
            if let Some(cit) = &style.citation {
                if let Some(t) = &cit.template
                    && let Some(template) = t.as_template()
                {
                    cit_sets.push(template_to_set(template));
                }
                if let Some(i) = &cit.integral
                    && let Some(t) = &i.template
                    && let Some(template) = t.as_template()
                {
                    cit_sets.push(template_to_set(template));
                }
                if let Some(ni) = &cit.non_integral
                    && let Some(t) = &ni.template
                    && let Some(template) = t.as_template()
                {
                    cit_sets.push(template_to_set(template));
                }
            }
            (base.clone(), bib_set, cit_sets)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_original_published_date_to_csl_original_date() {
        assert_eq!(
            semantic_to_legacy_key(&SemanticItem::Date(DateVariable::OriginalPublished)),
            "date:original-date"
        );
    }

    #[test]
    fn maps_collection_title_to_csl_collection_title() {
        assert_eq!(
            semantic_to_legacy_key(&SemanticItem::Title(TitleType::CollectionTitle)),
            "var:collection-title"
        );
    }

    #[test]
    fn keeps_parent_titles_as_csl_container_title() {
        assert_eq!(
            semantic_to_legacy_key(&SemanticItem::Title(TitleType::ContainerTitle)),
            "var:container-title"
        );
        assert_eq!(
            semantic_to_legacy_key(&SemanticItem::Title(TitleType::ParentMonograph)),
            "var:container-title"
        );
        assert_eq!(
            semantic_to_legacy_key(&SemanticItem::Title(TitleType::ParentSerial)),
            "var:container-title"
        );
    }
}
