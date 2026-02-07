use csln_core::template::TemplateComponent;

/// Deduplicate title components in nested lists.
pub fn deduplicate_titles_in_lists(components: &mut Vec<TemplateComponent>) {
    for component in components {
        if let TemplateComponent::List(list) = component {
            deduplicate_titles_in_list(list);
        }
    }
}

/// Deduplicate number components in nested lists.
/// Removes duplicate edition, volume, issue, etc. within the same List.
pub fn deduplicate_numbers_in_lists(components: &mut Vec<TemplateComponent>) {
    for component in components {
        if let TemplateComponent::List(list) = component {
            deduplicate_numbers_in_list(list);
        }
    }
}

fn deduplicate_numbers_in_list(list: &mut csln_core::template::TemplateList) {
    // Track seen number types in this list
    let mut seen_types = Vec::new();
    let mut i = 0;
    while i < list.items.len() {
        if let TemplateComponent::Number(n) = &list.items[i] {
            if seen_types.contains(&n.number) {
                // Remove duplicate number
                list.items.remove(i);
                continue;
            } else {
                seen_types.push(n.number.clone());
            }
        }
        i += 1;
    }

    // Recursively process nested lists
    for item in &mut list.items {
        if let TemplateComponent::List(inner_list) = item {
            deduplicate_numbers_in_list(inner_list);
        }
    }
}

fn deduplicate_titles_in_list(list: &mut csln_core::template::TemplateList) {
    // If list contains multiple titles of the same type, keep only the first
    // TitleType doesn't implement Hash/Eq in some versions, using Vec::contains instead
    let mut seen_types = Vec::new();
    let mut i = 0;
    while i < list.items.len() {
        if let TemplateComponent::Title(t) = &list.items[i] {
            if seen_types.contains(&t.title) {
                list.items.remove(i);
                continue;
            } else {
                seen_types.push(t.title.clone());
            }
        }
        i += 1;
    }

    // Recursively process nested lists
    for item in &mut list.items {
        if let TemplateComponent::List(inner_list) = item {
            deduplicate_titles_in_list(inner_list);
        }
    }
}

/// Deduplicate identical nested lists.
pub fn deduplicate_nested_lists(components: &mut [TemplateComponent]) {
    for component in components {
        if let TemplateComponent::List(list) = component {
            deduplicate_lists_in_items(&mut list.items);
            // Recursively process
            deduplicate_nested_lists(&mut list.items);
        }
    }
}

pub fn deduplicate_lists_in_items(items: &mut Vec<TemplateComponent>) {
    let mut i = 0;
    while i < items.len() {
        let mut j = i + 1;
        while j < items.len() {
            if let (TemplateComponent::List(l1), TemplateComponent::List(l2)) =
                (&items[i], &items[j])
            {
                if list_signature(l1) == list_signature(l2) {
                    items.remove(j);
                    continue;
                }
            }
            j += 1;
        }
        i += 1;
    }
}

pub fn list_signature(list: &csln_core::template::TemplateList) -> String {
    let mut sig = String::new();
    for item in &list.items {
        match item {
            TemplateComponent::Variable(v) => sig.push_str(&format!("v:{:?},", v.variable)),
            TemplateComponent::Number(n) => sig.push_str(&format!("n:{:?},", n.number)),
            TemplateComponent::Title(t) => sig.push_str(&format!("t:{:?},", t.title)),
            TemplateComponent::Contributor(c) => sig.push_str(&format!("c:{:?},", c.contributor)),
            TemplateComponent::Date(d) => sig.push_str(&format!("d:{:?},", d.date)),
            TemplateComponent::List(l) => sig.push_str(&format!("l({}),", list_signature(l))),
            _ => sig.push_str("unknown,"),
        }
    }
    sig
}

/// Suppress duplicate issue in parent-monograph lists for article-journal types.
pub fn suppress_duplicate_issue_for_journals(components: &mut [TemplateComponent], style_id: &str) {
    // Only apply to Chicago styles
    if !style_id.contains("chicago") {
        return;
    }

    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            suppress_issue_in_parent_monograph_list(&mut list.items);
        }
    }
}

fn suppress_issue_in_parent_monograph_list(items: &mut [TemplateComponent]) {
    use csln_core::template::{NumberVariable, TitleType};

    // Check if this list has parent-monograph (indicating it's the monographic source list)
    let has_parent_monograph = items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph
        ) || matches!(item, TemplateComponent::List(inner_list)
            if inner_list.items.iter().any(|i| matches!(i, TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph)))
    });

    if has_parent_monograph {
        // Suppress issue for article-journal in this list
        for item in items.iter_mut() {
            if let TemplateComponent::Number(n) = item {
                if n.number == NumberVariable::Issue {
                    let overrides = n
                        .overrides
                        .get_or_insert_with(std::collections::HashMap::new);
                    if let Some(rendering) = overrides.get_mut("article-journal") {
                        rendering.suppress = Some(true);
                    } else {
                        overrides.insert(
                            "article-journal".to_string(),
                            csln_core::template::Rendering {
                                suppress: Some(true),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            // Recursively check nested lists
            if let TemplateComponent::List(inner_list) = item {
                suppress_issue_in_parent_monograph_list(&mut inner_list.items);
            }
        }
    }

    // Recursively process all nested lists
    for item in items.iter_mut() {
        if let TemplateComponent::List(inner_list) = item {
            suppress_issue_in_parent_monograph_list(&mut inner_list.items);
        }
    }
}
