use csln_core::template::{
    DelimiterPunctuation, NumberVariable, Rendering, TemplateComponent, TemplateList,
    TemplateNumber, WrapPunctuation,
};

pub fn group_volume_and_issue(
    components: &mut Vec<TemplateComponent>,
    options: &csln_core::options::Config,
    style_preset: Option<crate::preset_detector::StylePreset>,
) {
    // Volume-issue spacing varies by style:
    // - APA (comma delimiter): no space, e.g., "2(2)"
    // - Chicago (colon delimiter): space, e.g., "2 (2)"
    let vol_issue_delimiter = if options
        .volume_pages_delimiter
        .as_ref()
        .is_some_and(|d| matches!(d, DelimiterPunctuation::Comma))
    {
        DelimiterPunctuation::None
    } else {
        DelimiterPunctuation::Space
    };

    // Check for issue at top level
    let issue_pos = components.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Issue),
    );

    // Check for volume at top level
    let vol_pos = components.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Volume),
    );

    // Case 1: Both at top level - combine into a List
    if let (Some(vol_idx), Some(issue_idx)) = (vol_pos, issue_pos) {
        let min_idx = vol_idx.min(issue_idx);
        let max_idx = vol_idx.max(issue_idx);

        // Remove from end first to preserve indices
        components.remove(max_idx);
        components.remove(min_idx);

        let vol_issue_list = TemplateComponent::List(TemplateList {
            items: vec![
                TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::Volume,
                    form: None,
                    rendering: Rendering::default(),
                    overrides: None,
                    ..Default::default()
                }),
                TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::Issue,
                    form: None,
                    rendering: Rendering {
                        wrap: Some(WrapPunctuation::Parentheses),
                        ..Default::default()
                    },
                    overrides: None,
                    ..Default::default()
                }),
            ],
            delimiter: Some(vol_issue_delimiter),
            rendering: Rendering::default(),
            overrides: None,
            ..Default::default()
        });

        components.insert(min_idx, vol_issue_list);
        return;
    }

    // Case 2: Issue at top level, volume inside a nested List
    // Find the List containing volume and add issue to it
    if let Some(issue_idx) = issue_pos {
        // First, find which List index contains volume (immutable borrow)
        let list_idx = components.iter().enumerate().find_map(|(idx, c)| {
            if let TemplateComponent::List(list) = c
                && find_volume_in_list(list).is_some()
            {
                return Some(idx);
            }
            None
        });

        if let Some(list_idx) = list_idx {
            // Extract the issue's overrides before removing it
            let issue_overrides =
                if let Some(TemplateComponent::Number(n)) = components.get(issue_idx) {
                    n.overrides.clone()
                } else {
                    None
                };

            // Remove issue from top level (adjusting for index shift if needed)
            components.remove(issue_idx);

            // Adjust list_idx if issue was before it
            let adjusted_list_idx = if issue_idx < list_idx {
                list_idx - 1
            } else {
                list_idx
            };

            // Create issue component with parentheses wrap
            let issue_with_parens = TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Issue,
                form: None,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    ..Default::default()
                },
                overrides: issue_overrides,
                ..Default::default()
            });

            // Now mutably access the list and add issue after volume
            if let Some(TemplateComponent::List(list)) = components.get_mut(adjusted_list_idx) {
                // Try to insert issue after volume - recursively searching nested lists
                if insert_issue_after_volume(
                    &mut list.items,
                    issue_with_parens,
                    vol_issue_delimiter.clone(),
                ) {
                    // Only update outer list delimiter if it's a serial source list
                    // (avoid changing delimiters for lists containing titles)
                    if matches!(style_preset, Some(crate::preset_detector::StylePreset::Apa))
                        && !list_contains_title(list)
                    {
                        list.delimiter = Some(DelimiterPunctuation::Comma);
                    }
                }
            }
        }
    }

    // Case 3: Neither at top level - issue is in a nested list somewhere
    // Find issue anywhere in nested lists and try to move it to volume's list
    if issue_pos.is_none() && vol_pos.is_none() {
        // Find the issue in any nested list and create a new one after volume
        let issue_exists_nested = find_issue_in_components(components);
        let volume_exists_nested = components.iter().any(|c| {
            if let TemplateComponent::List(list) = c {
                find_volume_in_list(list).is_some()
            } else {
                false
            }
        });

        if issue_exists_nested && volume_exists_nested {
            // Create issue component with parentheses wrap
            let issue_with_parens = TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Issue,
                form: None,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    ..Default::default()
                },
                overrides: None,
                ..Default::default()
            });

            // Find the list containing volume and add issue to it
            for component in components.iter_mut() {
                if let TemplateComponent::List(list) = component
                    && find_volume_in_list(list).is_some()
                    && insert_issue_after_volume(
                        &mut list.items,
                        issue_with_parens.clone(),
                        vol_issue_delimiter.clone(),
                    )
                {
                    break;
                }
            }
        }
    }
}

/// Check if issue exists anywhere in nested components.
pub fn find_issue_in_components(components: &[TemplateComponent]) -> bool {
    for component in components {
        match component {
            TemplateComponent::Number(n) if n.number == NumberVariable::Issue => {
                return true;
            }
            TemplateComponent::List(list) => {
                if find_issue_in_components(&list.items) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Insert issue component after volume, handling nested lists.
/// Returns true if successfully inserted.
pub fn insert_issue_after_volume(
    items: &mut Vec<TemplateComponent>,
    issue: TemplateComponent,
    delimiter: DelimiterPunctuation,
) -> bool {
    // First, check if volume is directly in this list
    if let Some(vol_pos) = items.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Volume),
    ) {
        // Remove volume from the list
        let volume = items.remove(vol_pos);

        // Create a new List containing [volume, issue] with no delimiter
        // This preserves the outer list's delimiter for other items
        let vol_issue_group = TemplateComponent::List(TemplateList {
            items: vec![volume, issue],
            delimiter: Some(delimiter), // No space between volume and issue
            rendering: Rendering::default(),
            overrides: None,
            ..Default::default()
        });

        // Insert the new group where volume was
        items.insert(vol_pos, vol_issue_group);
        return true;
    }

    // Otherwise, recurse into nested lists
    for item in items.iter_mut() {
        if let TemplateComponent::List(inner_list) = item
            && insert_issue_after_volume(&mut inner_list.items, issue.clone(), delimiter.clone())
        {
            return true;
        }
    }

    false
}

/// Check if a List contains a volume variable (recursively).
pub fn find_volume_in_list(list: &TemplateList) -> Option<()> {
    for item in &list.items {
        match item {
            TemplateComponent::Number(n) if n.number == NumberVariable::Volume => {
                return Some(());
            }
            TemplateComponent::List(inner_list) => {
                if find_volume_in_list(inner_list).is_some() {
                    return Some(());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn list_contains_title(list: &TemplateList) -> bool {
    list.items.iter().any(|c| {
        matches!(c, TemplateComponent::Title(_))
            || matches!(c, TemplateComponent::List(l) if list_contains_title(l))
    })
}
