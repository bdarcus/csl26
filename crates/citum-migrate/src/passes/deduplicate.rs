/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::{
    locale::GeneralTerm,
    template::{DateVariable, TemplateComponent},
};
use std::collections::HashSet;
use std::fmt::Write;

/// Deduplicate title components in nested lists.
pub fn deduplicate_titles_in_lists(components: &mut Vec<TemplateComponent>) {
    for component in components {
        if let TemplateComponent::Group(list) = component {
            deduplicate_titles_in_list(list);
        }
    }
}

/// Deduplicate number components in nested lists.
/// Removes duplicate edition, volume, issue, etc. within the same List.
pub fn deduplicate_numbers_in_lists(components: &mut Vec<TemplateComponent>) {
    for component in components {
        if let TemplateComponent::Group(list) = component {
            deduplicate_numbers_in_list(list);
        }
    }
}

/// Deduplicate date components in nested lists.
/// Removes duplicate issued, accessed, etc. within the same List.
pub fn deduplicate_dates_in_lists(components: &mut Vec<TemplateComponent>) {
    for component in components {
        if let TemplateComponent::Group(list) = component {
            deduplicate_dates_in_list(list);
        }
    }
}

/// Fold an explicit `no-date` term into an issued date's implicit fallback.
///
/// Migrated issued dates normally use an authoritative empty fallback to match
/// CSL's missing-variable behavior. When the source scope explicitly contains a
/// no-date term, restore the engine's locale-specific implicit fallback and
/// remove the now-redundant standalone term.
pub fn remove_redundant_no_date_terms(components: &mut Vec<TemplateComponent>) {
    remove_redundant_no_date_terms_in_scope(components);
}

fn deduplicate_typed_components<T, F>(list: &mut citum_schema::template::TemplateGroup, extract: F)
where
    T: PartialEq + Clone,
    F: Fn(&TemplateComponent) -> Option<T> + Copy,
{
    let mut seen = Vec::new();
    let mut i = 0;
    while i < list.group.len() {
        #[allow(clippy::indexing_slicing, reason = "i < list.group.len()")]
        let item = &list.group[i];
        if let Some(key) = extract(item) {
            if seen.contains(&key) {
                list.group.remove(i);
                continue;
            }
            seen.push(key);
        }
        i += 1;
    }
    for item in &mut list.group {
        if let TemplateComponent::Group(inner) = item {
            deduplicate_typed_components(inner, extract);
        }
    }
}

fn deduplicate_numbers_in_list(list: &mut citum_schema::template::TemplateGroup) {
    deduplicate_typed_components(list, |c| {
        if let TemplateComponent::Number(n) = c {
            Some(n.number.clone())
        } else {
            None
        }
    });
}

fn deduplicate_dates_in_list(list: &mut citum_schema::template::TemplateGroup) {
    deduplicate_typed_components(list, |c| {
        if let TemplateComponent::Date(d) = c {
            Some(d.date.clone())
        } else {
            None
        }
    });
}

fn deduplicate_titles_in_list(list: &mut citum_schema::template::TemplateGroup) {
    deduplicate_typed_components(list, |c| {
        if let TemplateComponent::Title(t) = c {
            Some(t.title.clone())
        } else {
            None
        }
    });
}

fn remove_redundant_no_date_terms_in_scope(items: &mut Vec<TemplateComponent>) {
    let has_issued = items.iter().any(component_contains_issued_date);
    let has_no_date = items.iter().any(component_contains_no_date_term);

    for item in items.iter_mut() {
        if let TemplateComponent::Group(list) = item {
            remove_redundant_no_date_terms_in_scope(&mut list.group);
        }
    }

    if has_issued && has_no_date {
        remove_no_date_terms_recursively(items);
    }
}

fn component_contains_issued_date(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Date(date) => date.date == DateVariable::Issued,
        TemplateComponent::Group(list) => list.group.iter().any(component_contains_issued_date),
        _ => false,
    }
}

fn is_no_date_term(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Term(term) if term.term == GeneralTerm::NoDate
    )
}

fn component_contains_no_date_term(component: &TemplateComponent) -> bool {
    is_no_date_term(component)
        || matches!(
            component,
            TemplateComponent::Group(list)
                if list.group.iter().any(component_contains_no_date_term)
        )
}

fn remove_no_date_terms_recursively(items: &mut Vec<TemplateComponent>) {
    items.retain(|item| !is_no_date_term(item));

    for item in items.iter_mut() {
        if let TemplateComponent::Group(list) = item {
            remove_no_date_terms_recursively(&mut list.group);
        }
    }
}

/// Deduplicate identical nested lists.
pub fn deduplicate_nested_lists(components: &mut [TemplateComponent]) {
    for component in components {
        if let TemplateComponent::Group(list) = component {
            deduplicate_lists_in_items(&mut list.group);
            // Recursively process
            deduplicate_nested_lists(&mut list.group);
        }
    }
}

pub fn deduplicate_lists_in_items(items: &mut Vec<TemplateComponent>) {
    let mut i = 0;
    while i < items.len() {
        let mut j = i + 1;
        while j < items.len() {
            #[allow(clippy::indexing_slicing, reason = "i and j are within bounds")]
            let item_pair = (&items[i], &items[j]);

            if let (TemplateComponent::Group(l1), TemplateComponent::Group(l2)) = item_pair
                && list_signature(l1) == list_signature(l2)
            {
                items.remove(j);
                continue;
            }
            j += 1;
        }
        i += 1;
    }
}

#[must_use]
pub fn list_signature(list: &citum_schema::template::TemplateGroup) -> String {
    let mut sig = String::new();
    for item in &list.group {
        match item {
            TemplateComponent::Variable(v) => {
                let _ = write!(sig, "v:{:?},", v.variable);
            }
            TemplateComponent::Number(n) => {
                let _ = write!(sig, "n:{:?},", n.number);
            }
            TemplateComponent::Title(t) => {
                let _ = write!(sig, "t:{:?},", t.title);
            }
            TemplateComponent::Contributor(c) => {
                let _ = write!(sig, "c:{:?},", c.contributor);
            }
            TemplateComponent::Date(d) => {
                let _ = write!(sig, "d:{:?},", d.date);
            }
            TemplateComponent::Group(l) => {
                let _ = write!(sig, "l({}),", list_signature(l));
            }
            _ => sig.push_str("unknown,"),
        }
    }
    sig
}

/// Suppress duplicate issue in parent-monograph lists for article-journal types.
pub fn suppress_duplicate_issue_for_journals(
    _components: &mut [TemplateComponent],
    _fixup_family: Option<crate::base_detector::FixupFamily>,
) {
    // Overrides-based suppression removed as component-level overrides are deprecated.
}

/// Deduplicate variables across sibling lists using global tracking.
/// When a variable is rendered in multiple sibling List nodes at the same nesting level,
/// suppress it in all but the first occurrence to enforce the "once" rule.
/// Deduplicate variables across sibling lists.
/// Removes components whose variable was already seen at the same nesting level.
pub fn deduplicate_variables_cross_lists(components: &mut Vec<TemplateComponent>) {
    let mut seen_vars = HashSet::new();
    remove_duplicate_variables(components, &mut seen_vars);
}

fn remove_duplicate_variables(items: &mut Vec<TemplateComponent>, seen_vars: &mut HashSet<String>) {
    let mut i = 0;
    while i < items.len() {
        #[allow(clippy::indexing_slicing, reason = "i < items.len()")]
        match &mut items[i] {
            TemplateComponent::Group(list) => {
                remove_duplicate_variables(&mut list.group, seen_vars);
                i += 1;
            }
            comp => {
                let key = component_var_key(comp);
                if let Some(k) = key {
                    if seen_vars.contains(&k) {
                        items.remove(i);
                        continue;
                    }
                    seen_vars.insert(k);
                }
                i += 1;
            }
        }
    }
}

fn component_var_key(component: &TemplateComponent) -> Option<String> {
    match component {
        TemplateComponent::Variable(v) => Some(format!("{:?}", v.variable)),
        TemplateComponent::Contributor(c) => Some(format!("{:?}", c.contributor)),
        TemplateComponent::Title(t) => Some(format!("{:?}", t.title)),
        TemplateComponent::Date(d) => Some(format!("{:?}", d.date)),
        TemplateComponent::Number(n) => Some(format!("{:?}", n.number)),
        _ => None,
    }
}
