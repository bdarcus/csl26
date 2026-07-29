/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Null-aware style overlay merging.

use super::{BibliographySpec, CitationSpec, Style};

/// Clear base `Option` fields whose raw YAML key is explicitly null in the overlay.
///
/// Invocation: `clear_raw_nulls!(base, raw_val, field => "yaml-key", ...)`.
/// Only call from a `Some(raw_val)` branch; `raw_val` is `&serde_yaml::Value`.
macro_rules! clear_raw_nulls {
    ($base:expr, $raw:expr, $( $field:ident => $key:literal ),+ $(,)?) => {
        if let Some(m) = ($raw).as_mapping() {
            $(
                if m.get(serde_yaml::Value::String($key.into()))
                    .is_some_and(serde_yaml::Value::is_null)
                {
                    $base.$field = None;
                }
            )+
        }
    };
}

/// Recursively merge overlay YAML into base YAML, mapping-by-mapping.
///
/// Overlay keys win; an explicitly null overlay key removes the base key so
/// the field re-resolves to its default (or `None`) on deserialization; any
/// non-mapping overlay value (scalar, sequence, preset name) replaces the
/// base value whole. These are the `extends` merge rules of
/// `docs/specs/STYLE_INHERITANCE.md`.
fn deep_merge_yaml_value(base: &mut serde_yaml::Value, overlay: &serde_yaml::Value) {
    match (base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                if overlay_value.is_null() {
                    base_map.remove(key);
                } else if let Some(base_value) = base_map.get_mut(key) {
                    deep_merge_yaml_value(base_value, overlay_value);
                } else {
                    base_map.insert(key.clone(), overlay_value.clone());
                }
            }
        }
        (base_value, overlay_value) => *base_value = overlay_value.clone(),
    }
}

/// Substitute preset-name strings in authored options YAML with the resolved
/// mapping from the typed overlay.
///
/// A preset reference like `contributors: springer` resolves to a config
/// struct at deserialization; layering that resolved block field-by-field
/// over the inherited one (biblatex option layering) is the established
/// semantics, so the raw string must not replace the inherited mapping whole.
/// Recursion follows the authored keys; typed values are consulted only where
/// a resolved mapping exists for an authored string.
fn effective_overlay_options(
    raw: &serde_yaml::Value,
    typed: &serde_yaml::Value,
) -> serde_yaml::Value {
    match (raw, typed) {
        (serde_yaml::Value::String(_), serde_yaml::Value::Mapping(_)) => typed.clone(),
        (serde_yaml::Value::Mapping(raw_map), serde_yaml::Value::Mapping(typed_map)) => {
            let mut out = serde_yaml::Mapping::new();
            for (key, raw_value) in raw_map {
                let effective = match typed_map.get(key) {
                    Some(typed_value) => effective_overlay_options(raw_value, typed_value),
                    None => raw_value.clone(),
                };
                out.insert(key.clone(), effective);
            }
            serde_yaml::Value::Mapping(out)
        }
        _ => raw.clone(),
    }
}

/// Deep-merge the overlay's authored `options` YAML over resolved base options.
///
/// Serializes the base options struct, merges the overlay's raw `options`
/// mapping (with preset strings expanded via [`effective_overlay_options`])
/// onto it via [`deep_merge_yaml_value`], and deserializes the result.
/// Working on the raw mapping means only keys the style author actually wrote
/// override the parent — fields left unwritten inherit, even inside nested
/// blocks whose struct fields carry serde defaults (the `csl26-svfg` gap).
///
/// Returns `None` when the overlay's raw options are absent or not a mapping
/// (e.g. programmatic construction without `raw_yaml`), when the typed
/// overlay no longer matches its raw YAML (programmatic mutation after
/// parse — e.g. a caller setting options before `into_resolved`), or on any
/// serde round-trip failure; callers fall back to the typed whole-field
/// merge in all these cases.
fn deep_merge_options_from_raw<T>(
    base: &T,
    overlay_typed: &T,
    raw_options: Option<&serde_yaml::Value>,
) -> Option<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    let raw = raw_options?;
    if !raw.is_mapping() {
        return None;
    }
    let authored: T = serde_yaml::from_value(raw.clone()).ok()?;
    if authored != *overlay_typed {
        return None;
    }
    let typed_value = serde_yaml::to_value(overlay_typed).ok()?;
    let effective = effective_overlay_options(raw, &typed_value);
    let mut base_value = serde_yaml::to_value(base).ok()?;
    deep_merge_yaml_value(&mut base_value, &effective);
    serde_yaml::from_value(base_value).ok()
}

/// Look up the raw `options` mapping inside a raw YAML section.
fn raw_options_of(raw: Option<&serde_yaml::Value>) -> Option<&serde_yaml::Value> {
    raw?.as_mapping()
        .and_then(|m| m.get(serde_yaml::Value::String("options".to_string())))
}

/// Merge an overlay style into a base style, with the overlay taking precedence.
///
/// Uses typed field merges throughout, preserving null-aware semantics for
/// citation/bibliography sub-specs (e.g., `ibid: ~` clears inherited values).
///
/// Applies a **regime coherence guard** after the citation merge: when the
/// overlay (child) declares a citation regime whose family differs from the base
/// (parent) regime, and the child provides its own base `citation.template`, the
/// inherited `citation.integral` and `citation.non_integral` sub-specs are reset
/// to the child's own declared values (typically `None`). Bibliography spec is
/// never touched by this guard. See `docs/specs/CITATION_REGIME.md`.
pub(crate) fn merge_style_overlay(base: &mut Style, overlay: &Style) {
    // Capture the parent's regime family before any merge so the guard below
    // can compare against the pre-merge baseline.
    let parent_regime_family = base
        .options
        .as_ref()
        .and_then(|c| c.processing.as_ref())
        .map(|p| p.regime_family());

    // Merge StyleInfo: per-field Option merge, fields vec replaces if non-empty
    merge_info(&mut base.info, &overlay.info);

    // Merge templates: per-key merge
    if let Some(overlay_templates) = &overlay.templates {
        match &mut base.templates {
            Some(base_templates) => {
                for (key, overlay_template) in overlay_templates {
                    base_templates.insert(key.clone(), overlay_template.clone());
                }
            }
            None => base.templates = Some(overlay_templates.clone()),
        }
    }

    // Merge options: deep-merge authored raw keys over the resolved base
    // (STYLE_INHERITANCE.md rule 1); fall back to the typed whole-field
    // merge when no raw YAML is available.
    if let Some(overlay_options) = &overlay.options {
        match &mut base.options {
            Some(existing) => {
                let raw_options = raw_options_of(overlay.raw_yaml.as_ref());
                if let Some(merged) =
                    deep_merge_options_from_raw(existing, overlay_options, raw_options)
                {
                    *existing = merged;
                } else {
                    existing.merge(overlay_options);
                }
            }
            None => base.options = Some(overlay_options.clone()),
        }
    }

    // Merge citation spec with raw_yaml for null-aware semantics
    let raw_citation = overlay.raw_yaml.as_ref().and_then(|v| v.get("citation"));
    if raw_citation.is_some() || overlay.citation.is_some() {
        let merged = match (&base.citation, &overlay.citation, raw_citation) {
            (Some(existing), Some(overlay_spec), Some(raw)) => {
                merge_citation_spec(existing.clone(), overlay_spec, Some(raw))
            }
            (Some(existing), Some(overlay_spec), None) => {
                merge_citation_spec(existing.clone(), overlay_spec, None)
            }
            (Some(existing), None, Some(raw)) => {
                apply_raw_null_clears_citation(existing.clone(), raw)
            }
            (Some(existing), None, None) => existing.clone(),
            (None, Some(overlay_spec), _) => overlay_spec.clone(),
            (None, None, _) => return, // Impossible due to outer if guard
        };
        base.citation = Some(merged);
    }

    // Regime coherence guard: if the child (overlay) declares a citation regime
    // whose family differs from the parent's (base), and the child supplies its
    // own base `citation.template`, reset the inherited citation-mode sub-specs
    // so they don't leak across regime boundaries at render time.
    //
    // Only fires when:
    //   1. The child has an explicit `processing` with a different regime family.
    //   2. The child provides its own `citation.template` (not just sub-spec overrides).
    //
    // Bibliography spec is intentionally not touched — sort/grouping is orthogonal
    // to citation regime. See `docs/specs/CITATION_REGIME.md`.
    if let Some(child_regime) = overlay
        .options
        .as_ref()
        .and_then(|c| c.processing.as_ref())
        .map(|p| p.regime_family())
        && parent_regime_family.is_some_and(|pr| pr != child_regime)
        && overlay
            .citation
            .as_ref()
            .and_then(|c| c.template.as_ref())
            .is_some()
        && let Some(base_cit) = base.citation.as_mut()
    {
        base_cit.integral = overlay.citation.as_ref().and_then(|c| c.integral.clone());
        base_cit.non_integral = overlay
            .citation
            .as_ref()
            .and_then(|c| c.non_integral.clone());
    }

    // Merge bibliography spec with raw_yaml for null-aware semantics
    let raw_bibliography = overlay
        .raw_yaml
        .as_ref()
        .and_then(|v| v.get("bibliography"));
    if raw_bibliography.is_some() || overlay.bibliography.is_some() {
        let merged = match (&base.bibliography, &overlay.bibliography, raw_bibliography) {
            (Some(existing), Some(overlay_spec), Some(raw)) => {
                merge_bibliography_spec(existing.clone(), overlay_spec, Some(raw))
            }
            (Some(existing), Some(overlay_spec), None) => {
                merge_bibliography_spec(existing.clone(), overlay_spec, None)
            }
            (Some(existing), None, Some(raw)) => {
                apply_raw_null_clears_bibliography(existing.clone(), raw)
            }
            (Some(existing), None, None) => existing.clone(),
            (None, Some(overlay_spec), _) => overlay_spec.clone(),
            (None, None, _) => return, // Impossible due to outer if guard
        };
        base.bibliography = Some(merged);
    }

    // Merge custom fields: per-key, overlay wins
    if let Some(overlay_custom) = &overlay.custom {
        match &mut base.custom {
            Some(base_custom) => {
                for (key, overlay_value) in overlay_custom {
                    base_custom.insert(key.clone(), overlay_value.clone());
                }
            }
            None => base.custom = Some(overlay_custom.clone()),
        }
    }
}

/// Merge StyleInfo fields: per-field Option merge, fields vec replaces if non-empty.
fn merge_info(base: &mut crate::StyleInfo, overlay: &crate::StyleInfo) {
    crate::merge_options!(
        base,
        overlay,
        title,
        id,
        description,
        default_locale,
        source,
        short_name,
        edition,
        citum_version
    );

    // fields: Vec replaces if overlay is non-empty
    if !overlay.fields.is_empty() {
        base.fields = overlay.fields.clone();
    }
}

/// Merge two CitationSpec values with optional raw YAML for null-aware clearing.
fn merge_citation_spec(
    mut base: CitationSpec,
    overlay: &CitationSpec,
    raw: Option<&serde_yaml::Value>,
) -> CitationSpec {
    // Merge per-field Options (except type_variants, which needs null-aware logic)
    crate::merge_options!(
        base,
        overlay,
        template_ref,
        template,
        locales,
        wrap,
        prefix,
        suffix,
        delimiter,
        multi_cite_delimiter,
        collapse,
        sort,
        note_start_text_case,
        custom
    );

    // Restore explicit-null clearing: raw `field: ~` must clear the inherited Option.
    // merge_options! above only writes when overlay is Some; null deserializes to None,
    // so without this pass an explicit null would silently preserve the base value.
    if let Some(raw_val) = raw {
        clear_citation_raw_nulls(&mut base, raw_val);
    }

    // Handle type_variants: per-key merge (base keys not in overlay are preserved)
    if let Some(raw_val) = raw {
        if let Some(mapping) = raw_val.as_mapping() {
            if mapping
                .get(serde_yaml::Value::String("type-variants".to_string()))
                .is_some_and(|v| v.is_null())
            {
                base.type_variants = None;
            } else if let Some(overlay_variants) = &overlay.type_variants {
                let base_variants = base.type_variants.get_or_insert_with(Default::default);
                for (key, variant) in overlay_variants {
                    base_variants.insert(key.clone(), variant.clone());
                }
            }
        }
    } else if let Some(overlay_variants) = &overlay.type_variants {
        let base_variants = base.type_variants.get_or_insert_with(Default::default);
        for (key, variant) in overlay_variants {
            base_variants.insert(key.clone(), variant.clone());
        }
    }

    // Merge options: deep-merge authored raw keys (STYLE_INHERITANCE.md
    // rule 1), falling back to the typed CitationOptions::merge.
    match (&mut base.options, &overlay.options) {
        (Some(existing), Some(other)) => {
            if let Some(merged) = deep_merge_options_from_raw(existing, other, raw_options_of(raw))
            {
                *existing = merged;
            } else {
                existing.merge(other);
            }
        }
        (None, Some(other)) => base.options = Some(other.clone()),
        _ => {}
    }

    merge_citation_sub_specs(&mut base, overlay, raw);

    for (key, overlay_value) in &overlay.unknown_fields {
        base.unknown_fields
            .insert(key.clone(), overlay_value.clone());
    }

    base
}

/// Merge the four Box<CitationSpec> sub-specs with null-aware clearing.
fn merge_citation_sub_specs(
    base: &mut CitationSpec,
    overlay: &CitationSpec,
    raw: Option<&serde_yaml::Value>,
) {
    if let Some(raw_val) = raw {
        base.integral = merge_opt_box_spec(&base.integral, &overlay.integral, raw_val, "integral");
        base.non_integral = merge_opt_box_spec(
            &base.non_integral,
            &overlay.non_integral,
            raw_val,
            "non-integral",
        );
        base.subsequent =
            merge_opt_box_spec(&base.subsequent, &overlay.subsequent, raw_val, "subsequent");
        base.ibid = merge_opt_box_spec(&base.ibid, &overlay.ibid, raw_val, "ibid");
    } else {
        merge_opt_box_spec_typed(&mut base.integral, &overlay.integral);
        merge_opt_box_spec_typed(&mut base.non_integral, &overlay.non_integral);
        merge_opt_box_spec_typed(&mut base.subsequent, &overlay.subsequent);
        merge_opt_box_spec_typed(&mut base.ibid, &overlay.ibid);
    }
}

/// Typed-only (no raw) merge for an `Option<Box<CitationSpec>>` sub-spec.
fn merge_opt_box_spec_typed(
    base: &mut Option<Box<CitationSpec>>,
    overlay: &Option<Box<CitationSpec>>,
) {
    if let Some(overlay_spec) = overlay {
        let base_spec = base.take().map(|b| *b).unwrap_or_default();
        *base = Some(Box::new(merge_citation_spec(base_spec, overlay_spec, None)));
    }
}

/// Merge two BibliographySpec values with optional raw YAML for null-aware clearing.
fn merge_bibliography_spec(
    mut base: BibliographySpec,
    overlay: &BibliographySpec,
    raw: Option<&serde_yaml::Value>,
) -> BibliographySpec {
    // Merge per-field Options (except type_variants and groups, which need null-aware logic)
    crate::merge_options!(base, overlay, template_ref, template, locales, sort, custom);

    // Restore explicit-null clearing for bibliography flat Option fields (mirrors citation).
    if let Some(raw_val) = raw {
        clear_bibliography_raw_nulls(&mut base, raw_val);
    }

    // Handle type_variants: per-key merge (base keys not in overlay are preserved)
    if let Some(raw_val) = raw {
        if let Some(mapping) = raw_val.as_mapping() {
            if mapping
                .get(serde_yaml::Value::String("type-variants".to_string()))
                .is_some_and(|v| v.is_null())
            {
                base.type_variants = None;
            } else if let Some(overlay_variants) = &overlay.type_variants {
                let base_variants = base.type_variants.get_or_insert_with(Default::default);
                for (key, variant) in overlay_variants {
                    base_variants.insert(key.clone(), variant.clone());
                }
            }
        }
    } else if let Some(overlay_variants) = &overlay.type_variants {
        let base_variants = base.type_variants.get_or_insert_with(Default::default);
        for (key, variant) in overlay_variants {
            base_variants.insert(key.clone(), variant.clone());
        }
    }

    // Handle groups with null-aware semantics
    if let Some(raw_val) = raw {
        if let Some(mapping) = raw_val.as_mapping() {
            if mapping
                .get(serde_yaml::Value::String("groups".to_string()))
                .is_some_and(|v| v.is_null())
            {
                // Explicit null in raw: clear the field
                base.groups = None;
            } else if overlay.groups.is_some() {
                // Not explicitly null and overlay has value: use overlay
                base.groups = overlay.groups.clone();
            }
        }
    } else if overlay.groups.is_some() {
        // No raw YAML: standard Option merge
        base.groups = overlay.groups.clone();
    }

    // Merge options: deep-merge authored raw keys (STYLE_INHERITANCE.md
    // rule 1), falling back to the typed BibliographyOptions::merge.
    match (&mut base.options, &overlay.options) {
        (Some(existing), Some(other)) => {
            if let Some(merged) = deep_merge_options_from_raw(existing, other, raw_options_of(raw))
            {
                *existing = merged;
            } else {
                existing.merge(other);
            }
        }
        (None, Some(other)) => base.options = Some(other.clone()),
        _ => {}
    }

    // groups_enabled: only override if present in raw_yaml
    if let Some(raw_val) = raw
        && let Some(bib_mapping) = raw_val.as_mapping()
        && bib_mapping.contains_key(serde_yaml::Value::String("groups-enabled".to_string()))
    {
        // If explicitly set in raw, use overlay's value
        base.groups_enabled = overlay.groups_enabled;
    } else if overlay.groups_enabled != BibliographySpec::default().groups_enabled {
        // If no raw but overlay differs from default, use overlay
        base.groups_enabled = overlay.groups_enabled;
    }

    // Merge unknown fields: per-key, overlay wins
    for (key, overlay_value) in &overlay.unknown_fields {
        base.unknown_fields
            .insert(key.clone(), overlay_value.clone());
    }

    base
}

/// Merge a Box<CitationSpec> with null-aware clearing via raw YAML.
///
/// If raw_yaml has the key as explicit null, clear the field (set to None).
/// Otherwise, standard Option merge.
fn merge_opt_box_spec(
    base: &Option<Box<CitationSpec>>,
    overlay: &Option<Box<CitationSpec>>,
    raw: &serde_yaml::Value,
    key: &str,
) -> Option<Box<CitationSpec>> {
    let is_explicitly_null = raw
        .as_mapping()
        .and_then(|m| {
            m.get(serde_yaml::Value::String(key.to_string()))
                .map(|v| v.is_null())
        })
        .unwrap_or(false);

    if is_explicitly_null {
        // Explicit null in raw YAML: clear the field
        None
    } else if let Some(overlay_spec) = overlay {
        // overlay is Some: merge and keep
        let base_spec = base.as_ref().map(|b| (**b).clone()).unwrap_or_default();
        Some(Box::new(merge_citation_spec(base_spec, overlay_spec, None)))
    } else {
        // overlay is None: keep base
        base.clone()
    }
}

/// Apply null-clearing for citation fields based on raw YAML.
///
/// Called when `overlay.citation` is None but raw citation section exists.
/// Clears any fields that are explicitly null in the raw value.
fn apply_raw_null_clears_citation(mut base: CitationSpec, raw: &serde_yaml::Value) -> CitationSpec {
    if let Some(mapping) = raw.as_mapping() {
        // Check for explicit nulls in sub-specs
        if mapping
            .get(serde_yaml::Value::String("integral".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.integral = None;
        }
        if mapping
            .get(serde_yaml::Value::String("non-integral".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.non_integral = None;
        }
        if mapping
            .get(serde_yaml::Value::String("subsequent".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.subsequent = None;
        }
        if mapping
            .get(serde_yaml::Value::String("ibid".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.ibid = None;
        }
    }
    base
}

/// Apply null-clearing for bibliography fields based on raw YAML.
///
/// Called when `overlay.bibliography` is None but raw bibliography section exists.
/// Clears any fields that are explicitly null in the raw value.
fn apply_raw_null_clears_bibliography(
    mut base: BibliographySpec,
    raw: &serde_yaml::Value,
) -> BibliographySpec {
    if let Some(mapping) = raw.as_mapping() {
        // Check for explicit nulls in optional fields
        if mapping
            .get(serde_yaml::Value::String("options".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.options = None;
        }
        if mapping
            .get(serde_yaml::Value::String("groups".to_string()))
            .is_some_and(|v| v.is_null())
        {
            base.groups = None;
        }
    }
    base
}

/// Clear `CitationSpec` Option fields that are explicitly null in raw YAML.
///
/// Extracted so the complexity budget is counted against this function, not
/// the already-complex `merge_citation_spec`.
fn clear_citation_raw_nulls(base: &mut CitationSpec, raw: &serde_yaml::Value) {
    clear_raw_nulls!(
        base,
        raw,
        template_ref => "template-ref",
        template => "template",
        locales => "locales",
        wrap => "wrap",
        prefix => "prefix",
        suffix => "suffix",
        delimiter => "delimiter",
        multi_cite_delimiter => "multi-cite-delimiter",
        collapse => "collapse",
        sort => "sort",
        note_start_text_case => "note-start-text-case",
        custom => "custom",
        options => "options",
    );
}

/// Clear `BibliographySpec` Option fields that are explicitly null in raw YAML.
fn clear_bibliography_raw_nulls(base: &mut BibliographySpec, raw: &serde_yaml::Value) {
    clear_raw_nulls!(
        base,
        raw,
        template_ref => "template-ref",
        template => "template",
        locales => "locales",
        sort => "sort",
        custom => "custom",
        options => "options",
    );
}
