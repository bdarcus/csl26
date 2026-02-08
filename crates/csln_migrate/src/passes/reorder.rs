use csln_core::template::{TemplateComponent, TemplateList};

/// Recursively reorder serial components (container-title, volume) in a template.
pub fn reorder_serial_components(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::{NumberVariable, TitleType};

    for component in components {
        if let TemplateComponent::List(list) = component {
            // Check if this list contains both volume and parent-serial
            let has_volume = list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Number(n) if n.number == NumberVariable::Volume
                )
            });
            let has_parent_serial = list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
                )
            });

            if has_volume && has_parent_serial {
                // Find positions
                let volume_pos = list.items.iter().position(|item| {
                    matches!(
                        item,
                        TemplateComponent::Number(n) if n.number == NumberVariable::Volume
                    )
                });
                let parent_serial_pos = list.items.iter().position(|item| {
                    matches!(
                        item,
                        TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
                    )
                });

                // If volume is before parent-serial, swap them
                if let (Some(vol_pos), Some(ps_pos)) = (volume_pos, parent_serial_pos) {
                    if vol_pos < ps_pos {
                        list.items.swap(vol_pos, ps_pos);
                    }
                }
            }

            // Recursively process nested lists
            for item in &mut list.items {
                if let TemplateComponent::List(inner_list) = item {
                    reorder_serial_components_in_list(inner_list);
                }
            }
        }
    }
}

/// Helper to reorder components in a single list.
pub fn reorder_serial_components_in_list(list: &mut TemplateList) {
    use csln_core::template::{NumberVariable, TitleType};

    // Check if this list contains both volume and parent-serial
    let has_volume = list.items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Number(n) if n.number == NumberVariable::Volume
        )
    });
    let has_parent_serial = list.items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
        )
    });

    if has_volume && has_parent_serial {
        // Find positions
        let volume_pos = list.items.iter().position(|item| {
            matches!(
                item,
                TemplateComponent::Number(n) if n.number == NumberVariable::Volume
            )
        });
        let parent_serial_pos = list.items.iter().position(|item| {
            matches!(
                item,
                TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
            )
        });

        // If volume is before parent-serial, swap them
        if let (Some(vol_pos), Some(ps_pos)) = (volume_pos, parent_serial_pos) {
            if vol_pos < ps_pos {
                list.items.swap(vol_pos, ps_pos);
            }
        }
    }
}

/// Move pages component to appear after the container-title/volume List.
pub fn reorder_pages_for_serials(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::{NumberVariable, TitleType};

    // Find the pages component position
    let pages_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Number(n) if n.number == NumberVariable::Pages
        )
    });

    // Find the List containing parent-serial (container-title for journals)
    // Need to search recursively since parent-serial may be in a nested List
    let serial_list_pos = components.iter().position(contains_parent_serial_recursive);

    // If pages is BEFORE the serial list, move it to right after
    if let (Some(p_pos), Some(s_pos)) = (pages_pos, serial_list_pos) {
        if p_pos < s_pos {
            let pages_component = components.remove(p_pos);
            // After removal, indices shift - insert at s_pos (which is now s_pos - 1 + 1 = s_pos)
            components.insert(s_pos, pages_component);
        }
    }

    fn contains_parent_serial_recursive(component: &TemplateComponent) -> bool {
        match component {
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial => true,
            TemplateComponent::List(list) => {
                list.items.iter().any(contains_parent_serial_recursive)
            }
            _ => false,
        }
    }
}

/// Reorder publisher-place for Chicago journal articles.
pub fn reorder_publisher_place_for_chicago(
    components: &mut Vec<TemplateComponent>,
    style_preset: Option<crate::preset_detector::StylePreset>,
) {
    use crate::preset_detector::StylePreset;
    use csln_core::template::{SimpleVariable, TitleType};

    // Only apply to Chicago styles
    if !matches!(style_preset, Some(StylePreset::Chicago)) {
        return;
    }

    // Find the publisher-place component (it's in a List with wrap: parentheses)
    let publisher_place_pos = components.iter().position(|c| {
        if let TemplateComponent::List(list) = c {
            list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Variable(v)
                    if v.variable == SimpleVariable::PublisherPlace
                )
            })
        } else {
            false
        }
    });

    // Find the parent-serial title position
    let parent_serial_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
        )
    });

    // If we found both, move publisher-place to right after parent-serial
    if let (Some(pp_pos), Some(ps_pos)) = (publisher_place_pos, parent_serial_pos) {
        if pp_pos > ps_pos {
            // Remove the publisher-place List
            let mut publisher_place_component = components.remove(pp_pos);

            // Add space suffix to prevent default period separator
            if let TemplateComponent::List(ref mut list) = publisher_place_component {
                list.rendering.suffix = Some(" ".to_string());
            }

            // Insert it right after parent-serial
            components.insert(ps_pos + 1, publisher_place_component);
        }
    }
}

/// Reorder chapter components for APA style.
pub fn reorder_chapters_for_apa(
    components: &mut Vec<TemplateComponent>,
    style_preset: Option<crate::preset_detector::StylePreset>,
) {
    use crate::preset_detector::StylePreset;
    use csln_core::template::{ContributorRole, TitleType};

    // Only apply to APA styles
    if !matches!(style_preset, Some(StylePreset::Apa)) {
        return;
    }

    // Find the editor contributor
    let editor_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Contributor(contrib)
            if contrib.contributor == ContributorRole::Editor
        )
    });

    // Find the parent-monograph title
    let parent_monograph_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph
        )
    });

    if let (Some(ed_pos), Some(pm_pos)) = (editor_pos, parent_monograph_pos) {
        if ed_pos > pm_pos {
            // Swap them: move editor before parent-monograph
            let editor_comp = components.remove(ed_pos);
            components.insert(pm_pos, editor_comp);

            // Re-calculate positions after move
            let ed_pos = pm_pos;

            // Apply APA chapter formatting
            if let Some(TemplateComponent::Contributor(ref mut ed)) = components.get_mut(ed_pos) {
                ed.name_order = Some(csln_core::template::NameOrder::GivenFirst);
                let overrides = ed
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                overrides.insert(
                    "chapter".to_string(),
                    csln_core::template::Rendering {
                        prefix: Some("In ".to_string()),
                        suffix: Some(", ".to_string()),
                        ..Default::default()
                    },
                );
                overrides.insert(
                    "paper-conference".to_string(),
                    csln_core::template::Rendering {
                        prefix: Some("In ".to_string()),
                        suffix: Some(", ".to_string()),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

/// Reorder chapter components for Chicago style.
pub fn reorder_chapters_for_chicago(
    components: &mut Vec<TemplateComponent>,
    style_preset: Option<crate::preset_detector::StylePreset>,
) {
    use crate::preset_detector::StylePreset;
    use csln_core::template::{ContributorRole, TitleType};

    // Only apply to Chicago styles
    if !matches!(style_preset, Some(StylePreset::Chicago)) {
        return;
    }

    // Find the editor contributor (form: verb)
    let editor_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Contributor(contrib)
            if contrib.contributor == ContributorRole::Editor
        )
    });

    // Find the parent-monograph title
    let parent_monograph_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph
        )
    });

    // If we found both and editor comes before parent-monograph, swap them
    if let (Some(editor_pos), Some(pm_pos)) = (editor_pos, parent_monograph_pos) {
        if editor_pos < pm_pos {
            // Get mutable references to both components
            let editor_component = components.remove(editor_pos);
            let pm_component = components.remove(pm_pos - 1); // Adjust index after removal

            // Add "In " prefix and ", " suffix to parent-monograph for chapters
            let mut pm_with_prefix = pm_component.clone();
            if let TemplateComponent::Title(ref mut title) = pm_with_prefix {
                // Use type-specific override to add "In " prefix and ", " suffix for chapters
                let mut overrides = title.overrides.clone().unwrap_or_default();
                overrides.insert(
                    "chapter".to_string(),
                    csln_core::template::Rendering {
                        prefix: Some("In ".to_string()),
                        suffix: Some(", ".to_string()),
                        ..Default::default()
                    },
                );
                title.overrides = Some(overrides);
            }

            // Adjust editor for chapters: use ". " suffix and given-first name order
            let mut editor_with_suffix = editor_component.clone();
            if let TemplateComponent::Contributor(ref mut contrib) = editor_with_suffix {
                // For chapters, editors should use given-first name order
                use csln_core::template::NameOrder;
                contrib.name_order = Some(NameOrder::GivenFirst);

                // Add override to change suffix for chapters
                let mut overrides = contrib.overrides.clone().unwrap_or_default();
                overrides.insert(
                    "chapter".to_string(),
                    csln_core::template::Rendering {
                        suffix: Some(". ".to_string()),
                        ..Default::default()
                    },
                );
                contrib.overrides = Some(overrides);
            }

            // Re-insert in new order: parent-monograph, then editor
            components.insert(editor_pos, pm_with_prefix);
            components.insert(editor_pos + 1, editor_with_suffix);
        }
    }
}

/// Propagate type-specific overrides within Lists.
pub fn propagate_list_overrides(components: &mut [TemplateComponent]) {
    use csln_core::template::Rendering;
    use std::collections::HashSet;

    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            propagate_overrides_in_list(&mut list.items);

            // Recursively process nested lists
            for item in &mut list.items {
                if let TemplateComponent::List(inner_list) = item {
                    propagate_overrides_in_list(&mut inner_list.items);
                }
            }
        }
    }

    fn propagate_overrides_in_list(items: &mut [TemplateComponent]) {
        // Collect all type keys that have overrides in any item
        let mut all_override_types: HashSet<String> = HashSet::new();

        for item in items.iter() {
            if let Some(overrides) = get_component_overrides(item) {
                for key in overrides.keys() {
                    all_override_types.insert(key.clone());
                }
            }
        }

        // For each type that exists in any item, ensure all items have it
        for type_key in &all_override_types {
            for item in items.iter_mut() {
                if let Some(overrides) = get_component_overrides_mut(item) {
                    if !overrides.contains_key(type_key) {
                        // Add the override with suppress: false
                        overrides.insert(
                            type_key.clone(),
                            Rendering {
                                suppress: Some(false),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }

    fn get_component_overrides(
        comp: &TemplateComponent,
    ) -> Option<&std::collections::HashMap<String, Rendering>> {
        match comp {
            TemplateComponent::Contributor(c) => c.overrides.as_ref(),
            TemplateComponent::Date(d) => d.overrides.as_ref(),
            TemplateComponent::Title(t) => t.overrides.as_ref(),
            TemplateComponent::Number(n) => n.overrides.as_ref(),
            TemplateComponent::Variable(v) => v.overrides.as_ref(),
            _ => None,
        }
    }

    fn get_component_overrides_mut(
        comp: &mut TemplateComponent,
    ) -> Option<&mut std::collections::HashMap<String, Rendering>> {
        match comp {
            TemplateComponent::Contributor(c) => {
                if c.overrides.is_none() {
                    c.overrides = Some(std::collections::HashMap::new());
                }
                c.overrides.as_mut()
            }
            TemplateComponent::Date(d) => {
                if d.overrides.is_none() {
                    d.overrides = Some(std::collections::HashMap::new());
                }
                d.overrides.as_mut()
            }
            TemplateComponent::Title(t) => {
                if t.overrides.is_none() {
                    t.overrides = Some(std::collections::HashMap::new());
                }
                t.overrides.as_mut()
            }
            TemplateComponent::Number(n) => {
                if n.overrides.is_none() {
                    n.overrides = Some(std::collections::HashMap::new());
                }
                n.overrides.as_mut()
            }
            TemplateComponent::Variable(v) => {
                if v.overrides.is_none() {
                    v.overrides = Some(std::collections::HashMap::new());
                }
                v.overrides.as_mut()
            }
            _ => None,
        }
    }
}

/// Recursively ensure specific variables are un-suppressed for a given type.
pub fn unsuppress_for_type(components: &mut [TemplateComponent], item_type: &str) {
    use csln_core::template::SimpleVariable;

    for component in components {
        match component {
            TemplateComponent::Variable(v)
                if matches!(
                    v.variable,
                    SimpleVariable::Publisher | SimpleVariable::PublisherPlace
                ) =>
            {
                let overrides = v
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                overrides.insert(
                    item_type.to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(false),
                        ..Default::default()
                    },
                );
            }
            TemplateComponent::List(list) => {
                unsuppress_for_type(&mut list.items, item_type);
            }
            _ => {}
        }
    }
}

/// Add space prefix to volume when it directly follows parent-serial title.
/// This handles numeric styles where journal and volume are siblings, not in a List.
pub fn add_volume_prefix_after_serial(components: &mut [TemplateComponent]) {
    use csln_core::template::{NumberVariable, TitleType};

    for i in 1..components.len() {
        let prev_is_serial = matches!(
            components.get(i - 1),
            Some(TemplateComponent::Title(t)) if t.title == TitleType::ParentSerial
        );

        if prev_is_serial {
            if let Some(TemplateComponent::Number(ref mut num)) = components.get_mut(i) {
                if num.number == NumberVariable::Volume {
                    eprintln!("DEBUG: Adding space prefix to volume after parent-serial");
                    // Add space prefix if not already present
                    if num.rendering.prefix.is_none() {
                        num.rendering.prefix = Some(" ".to_string());
                    }
                }
            }
        }
    }
}

/// Move DOI and URL components to the end of the bibliography template.
pub fn move_access_components_to_end(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::SimpleVariable;

    // Find indices of access components (DOI, URL)
    let mut access_indices: Vec<usize> = Vec::new();
    for (i, c) in components.iter().enumerate() {
        if let TemplateComponent::Variable(v) = c {
            if matches!(v.variable, SimpleVariable::Doi | SimpleVariable::Url) {
                access_indices.push(i);
            }
        }
        // Also check for List items containing accessed date (URL + accessed date pattern)
        if let TemplateComponent::List(list) = c {
            let has_access = list.items.iter().any(|item| {
                matches!(item, TemplateComponent::Variable(v) if v.variable == SimpleVariable::Url)
                    || matches!(item, TemplateComponent::Date(d) if d.date == csln_core::template::DateVariable::Accessed)
            });
            if has_access {
                access_indices.push(i);
            }
        }
    }

    // Extract access components in reverse order (to preserve indices)
    let mut access_components: Vec<TemplateComponent> = Vec::new();
    for idx in access_indices.into_iter().rev() {
        access_components.push(components.remove(idx));
    }
    access_components.reverse();

    // Append access components at the end
    components.extend(access_components);
}
