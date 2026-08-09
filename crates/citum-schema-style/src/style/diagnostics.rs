/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Raw YAML diagnostics for template-bearing style surfaces.

use serde_yaml::Value;

const COMPONENT_KINDS: &[&str] = &[
    "contributor",
    "date",
    "title",
    "number",
    "identifier",
    "variable",
    "message",
    "group",
    "term",
    "type-label",
];

const RENDERING_FIELDS: &[&str] = &[
    "text-case",
    "emph",
    "quote",
    "strong",
    "small-caps",
    "vertical-align",
    "prefix",
    "suffix",
    "wrap",
    "suppress",
    "initialize-with",
    "name-form",
    "strip-periods",
];

const ADD_OPERATION_FIELDS: &[&str] = &["before", "after", "component"];
const DIFF_FIELDS: &[&str] = &["extends", "modify", "remove", "add"];
const MODIFY_OPERATION_FIELDS: &[&str] = &[
    "match",
    "label-form",
    "text-case",
    "emph",
    "quote",
    "strong",
    "small-caps",
    "vertical-align",
    "prefix",
    "suffix",
    "wrap",
    "suppress",
    "initialize-with",
    "name-form",
    "strip-periods",
];
const REMOVE_OPERATION_FIELDS: &[&str] = &["match"];

pub(super) fn validate_raw_style(raw: &Value) -> Result<(), String> {
    let Some(root) = raw.as_mapping() else {
        return Ok(());
    };

    if let Some(templates) = child(root, "templates") {
        validate_templates_map(templates, "templates")?;
    }
    if let Some(citation) = child(root, "citation") {
        validate_citation_spec(citation, "citation")?;
    }
    if let Some(bibliography) = child(root, "bibliography") {
        validate_bibliography_spec(bibliography, "bibliography")?;
    }

    Ok(())
}

fn validate_templates_map(value: &Value, path: &str) -> Result<(), String> {
    let Some(map) = value.as_mapping() else {
        return Ok(());
    };

    for (key, template) in map {
        let path = append_path(path, key_path_segment(key));
        validate_template(template, &path)?;
    }

    Ok(())
}

fn validate_citation_spec(value: &Value, path: &str) -> Result<(), String> {
    let Some(map) = value.as_mapping() else {
        return Ok(());
    };

    validate_common_section_templates(map, path)?;

    for mode in ["integral", "non-integral", "subsequent", "ibid"] {
        if let Some(child_spec) = child(map, mode) {
            validate_citation_spec(child_spec, &format!("{path}.{mode}"))?;
        }
    }

    Ok(())
}

fn validate_bibliography_spec(value: &Value, path: &str) -> Result<(), String> {
    let Some(map) = value.as_mapping() else {
        return Ok(());
    };

    validate_common_section_templates(map, path)?;

    if let Some(groups) = child(map, "groups")
        && let Some(group_values) = groups.as_sequence()
    {
        for (index, group) in group_values.iter().enumerate() {
            let Some(group_map) = group.as_mapping() else {
                continue;
            };
            if let Some(template) = child(group_map, "template") {
                validate_template(template, &format!("{path}.groups[{index}].template"))?;
            }
        }
    }

    Ok(())
}

fn validate_common_section_templates(map: &serde_yaml::Mapping, path: &str) -> Result<(), String> {
    if let Some(template) = child(map, "template") {
        let template_path = format!("{path}.template");
        if template.as_sequence().is_some() {
            validate_template(template, &template_path)?;
        } else if let Some(diff) = template.as_mapping() {
            validate_diff(diff, &template_path)?;
        }
    }
    if let Some(locales) = child(map, "locales")
        && let Some(locale_values) = locales.as_sequence()
    {
        for (index, locale) in locale_values.iter().enumerate() {
            let Some(locale_map) = locale.as_mapping() else {
                continue;
            };
            if let Some(template) = child(locale_map, "template") {
                validate_template(template, &format!("{path}.locales[{index}].template"))?;
            }
        }
    }
    if let Some(type_variants) = child(map, "type-variants") {
        validate_type_variants(type_variants, &format!("{path}.type-variants"))?;
    }

    Ok(())
}

fn validate_type_variants(value: &Value, path: &str) -> Result<(), String> {
    let Some(variants) = value.as_mapping() else {
        return Ok(());
    };

    for (selector, variant) in variants {
        let variant_path = append_path(path, key_path_segment(selector));
        if variant.as_sequence().is_some() {
            validate_template(variant, &variant_path)?;
        } else if let Some(diff) = variant.as_mapping() {
            validate_diff(diff, &variant_path)?;
        }
    }

    Ok(())
}

fn validate_diff(diff: &serde_yaml::Mapping, path: &str) -> Result<(), String> {
    validate_fields(diff, path, "TemplateVariantDiff", DIFF_FIELDS)?;

    if let Some(modify) = child(diff, "modify") {
        validate_operation_list(
            modify,
            &format!("{path}.modify"),
            "TemplateModifyOperation",
            MODIFY_OPERATION_FIELDS,
            None,
        )?;
    }
    if let Some(remove) = child(diff, "remove") {
        validate_operation_list(
            remove,
            &format!("{path}.remove"),
            "TemplateRemoveOperation",
            REMOVE_OPERATION_FIELDS,
            None,
        )?;
    }
    if let Some(add) = child(diff, "add") {
        validate_operation_list(
            add,
            &format!("{path}.add"),
            "TemplateAddOperation",
            ADD_OPERATION_FIELDS,
            Some(validate_add_operation),
        )?;
    }

    Ok(())
}

fn validate_operation_list(
    value: &Value,
    path: &str,
    type_name: &str,
    allowed_fields: &[&str],
    extra: Option<fn(&serde_yaml::Mapping, &str) -> Result<(), String>>,
) -> Result<(), String> {
    let Some(operations) = value.as_sequence() else {
        return Ok(());
    };

    for (index, operation) in operations.iter().enumerate() {
        let Some(map) = operation.as_mapping() else {
            continue;
        };
        let operation_path = format!("{path}[{index}]");
        validate_fields(map, &operation_path, type_name, allowed_fields)?;
        if let Some(extra) = extra {
            extra(map, &operation_path)?;
        }
    }

    Ok(())
}

fn validate_add_operation(map: &serde_yaml::Mapping, path: &str) -> Result<(), String> {
    if let Some(component) = child(map, "component") {
        validate_component(component, &format!("{path}.component"))?;
    }
    Ok(())
}

fn validate_template(value: &Value, path: &str) -> Result<(), String> {
    let Some(components) = value.as_sequence() else {
        return Ok(());
    };

    for (index, component) in components.iter().enumerate() {
        validate_component(component, &format!("{path}[{index}]"))?;
    }

    Ok(())
}

fn validate_component(value: &Value, path: &str) -> Result<(), String> {
    let Some(map) = value.as_mapping() else {
        return Ok(());
    };

    let mut kinds = Vec::new();
    for key in string_keys(map) {
        if COMPONENT_KINDS.contains(&key) {
            kinds.push(key);
        }
    }

    match kinds.as_slice() {
        [] => {
            let unknown = string_keys(map)
                .into_iter()
                .find(|key| !RENDERING_FIELDS.contains(key));
            if let Some(unknown) = unknown {
                return Err(format!(
                    "{path}: unknown template component property \"{unknown}\"; component must contain exactly one of {}{}",
                    COMPONENT_KINDS.join("/"),
                    suggestion_suffix(unknown, COMPONENT_KINDS)
                ));
            }
            Err(format!(
                "{path}: component must contain exactly one of {}",
                COMPONENT_KINDS.join("/")
            ))
        }
        [kind] => {
            validate_component_fields(map, path, kind)?;
            validate_nested_component_templates(map, path, kind)
        }
        _ => Err(format!(
            "{path}: component must contain exactly one of {}; found {}",
            COMPONENT_KINDS.join("/"),
            kinds.join(", ")
        )),
    }
}

fn validate_component_fields(
    map: &serde_yaml::Mapping,
    path: &str,
    kind: &str,
) -> Result<(), String> {
    let allowed = component_allowed_fields(kind);
    let allowed_refs: Vec<&str> = allowed
        .iter()
        .copied()
        .chain(RENDERING_FIELDS.iter().copied())
        .collect();
    let type_name = component_type_name(kind);
    validate_fields(map, path, type_name, &allowed_refs)
}

fn validate_nested_component_templates(
    map: &serde_yaml::Mapping,
    path: &str,
    kind: &str,
) -> Result<(), String> {
    match kind {
        "date" | "contributor" => {
            if let Some(fallback) = child(map, "fallback") {
                validate_template(fallback, &format!("{path}.fallback"))?;
            }
        }
        "group" => {
            if let Some(group) = child(map, "group") {
                validate_template(group, &format!("{path}.group"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_fields(
    map: &serde_yaml::Mapping,
    path: &str,
    type_name: &str,
    allowed_fields: &[&str],
) -> Result<(), String> {
    for key in string_keys(map) {
        if !allowed_fields.contains(&key) {
            return Err(format!(
                "{path}: unknown property \"{key}\" in {type_name}{}",
                suggestion_suffix(key, allowed_fields)
            ));
        }
    }
    Ok(())
}

fn component_allowed_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "contributor" => &[
            "contributor",
            "form",
            "fallback",
            "label",
            "merge",
            "name-order",
            "name-form",
            "delimiter",
            "sort-separator",
            "shorten",
            "and",
            "links",
            "gender",
            "custom",
        ],
        "date" => &[
            "date",
            "form",
            "fallback",
            "suppress-note",
            "suppress-disamb-suffix",
            "links",
            "custom",
        ],
        "title" => &[
            "title",
            "form",
            "disambiguate-only",
            "strip-periods-all",
            "links",
            "custom",
        ],
        "number" => &[
            "number",
            "form",
            "label-form",
            "when-numeric",
            "show-with-locator",
            "links",
            "gender",
            "custom",
        ],
        "identifier" => &["identifier"],
        "variable" => &["variable", "links", "custom"],
        "message" => &["message", "form", "gender", "args", "custom"],
        "term" => &["term", "form", "gender", "custom"],
        "group" => &["group", "render-when", "delimiter", "custom"],
        "type-label" => &["type-label", "custom"],
        _ => &[],
    }
}

fn component_type_name(kind: &str) -> &'static str {
    match kind {
        "contributor" => "TemplateContributor",
        "date" => "TemplateDate",
        "title" => "TemplateTitle",
        "number" => "TemplateNumber",
        "identifier" => "TemplateIdentifier",
        "variable" => "TemplateVariable",
        "message" => "TemplateMessage",
        "term" => "TemplateTerm",
        "group" => "TemplateGroup",
        "type-label" => "TemplateTypeLabel",
        _ => "TemplateComponent",
    }
}

fn child<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

fn string_keys(map: &serde_yaml::Mapping) -> Vec<&str> {
    map.keys().filter_map(Value::as_str).collect::<Vec<_>>()
}

fn key_path_segment(key: &Value) -> String {
    key.as_str()
        .map_or_else(|| format!("[{key:?}]"), ToString::to_string)
}

fn append_path(path: &str, segment: String) -> String {
    if segment.starts_with('[') {
        format!("{path}{segment}")
    } else {
        format!("{path}.{segment}")
    }
}

fn suggestion_suffix(value: &str, candidates: &[&str]) -> String {
    closest_candidate(value, candidates).map_or_else(String::new, |candidate| {
        format!("; did you mean \"{candidate}\"?")
    })
}

fn closest_candidate<'a>(value: &str, candidates: &'a [&str]) -> Option<&'a str> {
    candidates
        .iter()
        .copied()
        .filter_map(|candidate| {
            let distance = levenshtein(value, candidate);
            let max_len = value.len().max(candidate.len());
            (distance <= 3 || distance * 3 <= max_len).then_some((candidate, distance))
        })
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| candidate)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous_row = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current_row = Vec::with_capacity(right_chars.len().saturating_add(1));

    for (left_index, left_char) in left.chars().enumerate() {
        current_row.clear();
        current_row.push(left_index + 1);
        let mut northwest = left_index;

        for (right_index, right_char) in right_chars.iter().enumerate() {
            let north = previous_row
                .get(right_index + 1)
                .copied()
                .unwrap_or(usize::MAX / 4);
            let west = current_row.last().copied().unwrap_or_default();
            let substitution = northwest + usize::from(left_char != *right_char);
            let next = north
                .saturating_add(1)
                .min(west.saturating_add(1))
                .min(substitution);
            northwest = north;
            current_row.push(next);
        }

        std::mem::swap(&mut previous_row, &mut current_row);
    }

    previous_row.last().copied().unwrap_or_default()
}
