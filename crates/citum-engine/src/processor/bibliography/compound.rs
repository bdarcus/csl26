/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Compound-entry merging for numeric bibliography styles.

use super::Processor;
use crate::processor::FinalizedRun;
use crate::render::ProcEntry;
use crate::render::bibliography::render_entry_body_components_with_format;
use crate::render::component::ProcTemplateComponent;
use crate::render::format::OutputFormat;
use citum_schema::template::TemplateComponent;
use indexmap::IndexMap;
use std::collections::HashMap;

impl Processor {
    pub(super) fn compound_numeric_config(
        &self,
    ) -> Option<citum_schema::options::bibliography::CompoundNumericConfig> {
        self.get_bibliography_options().compound_numeric.clone()
    }

    pub(super) fn build_compound_group_lookup(
        compound_groups: &IndexMap<usize, Vec<String>>,
    ) -> HashMap<String, usize> {
        let mut ref_to_group = HashMap::new();
        for (&group_number, ids) in compound_groups {
            if ids.len() > 1 {
                for id in ids {
                    ref_to_group.insert(id.clone(), group_number);
                }
            }
        }
        ref_to_group
    }

    fn render_compound_entry_bodies<F>(entries: &[ProcEntry]) -> HashMap<String, String>
    where
        F: OutputFormat<Output = String>,
    {
        entries
            .iter()
            .map(|entry| {
                // The marker is not a template component, so the body is
                // already marker-free.
                let content_components = entry.template.clone();
                (
                    entry.id.clone(),
                    render_entry_body_components_with_format::<F>(&content_components)
                        .trim()
                        .to_string(),
                )
            })
            .collect()
    }

    fn build_present_group_members(
        entries: &[ProcEntry],
        ref_to_group: &HashMap<String, usize>,
    ) -> HashMap<usize, Vec<String>> {
        let mut group_members_present = HashMap::new();
        for entry in entries {
            if let Some(&group_number) = ref_to_group.get(&entry.id) {
                group_members_present
                    .entry(group_number)
                    .or_insert_with(Vec::new)
                    .push(entry.id.clone());
            }
        }
        group_members_present
    }

    fn build_merged_compound_entry(
        &self,
        entry: ProcEntry,
        group_ids: &[String],
        rendered_strings: &HashMap<String, String>,
        compound_config: &citum_schema::options::bibliography::CompoundNumericConfig,
    ) -> ProcEntry {
        let mut merged_body = String::new();
        let mut has_content = false;

        for (index, id) in group_ids.iter().enumerate() {
            let Some(rendered) = rendered_strings.get(id) else {
                continue;
            };

            let sub_label = match compound_config.sub_label {
                citum_schema::options::bibliography::SubLabelStyle::Alphabetic => {
                    format!(
                        "{}{}",
                        crate::values::int_to_letter((index + 1) as u32)
                            .unwrap_or_else(|| "a".into()),
                        compound_config.sub_label_suffix
                    )
                }
                citum_schema::options::bibliography::SubLabelStyle::Numeric => {
                    format!("{}{}", index + 1, compound_config.sub_label_suffix)
                }
            };

            if has_content {
                merged_body.push_str(&compound_config.sub_delimiter);
            }
            has_content = true;
            merged_body.push_str(&sub_label);
            merged_body.push(' ');
            merged_body.push_str(rendered);
        }

        // The marker rides on the entry, so the merged body is a single
        // pre-formatted component.
        let merged_template: Vec<ProcTemplateComponent> = vec![ProcTemplateComponent {
            template_component: TemplateComponent::default(),
            value: merged_body,
            sentence_initial: false,
            pre_formatted: true,
            config: entry
                .template
                .first()
                .and_then(|component| component.config.clone()),
            ..Default::default()
        }];

        ProcEntry {
            marker: entry.marker.clone(),
            id: entry.id,
            template: merged_template,
            metadata: entry.metadata,
        }
    }

    pub(super) fn merge_compound_entries<F>(
        &self,
        entries: Vec<ProcEntry>,
        run: &FinalizedRun,
    ) -> Vec<ProcEntry>
    where
        F: OutputFormat<Output = String>,
    {
        let compound_groups = &run.state().compound_groups;
        if compound_groups.is_empty() {
            return entries;
        }

        let Some(compound_config) = self.compound_numeric_config() else {
            return entries;
        };

        let ref_to_group = Self::build_compound_group_lookup(compound_groups);
        if ref_to_group.is_empty() {
            return entries;
        }

        let rendered_strings = Self::render_compound_entry_bodies::<F>(&entries);
        let group_members_present = Self::build_present_group_members(&entries, &ref_to_group);
        let first_present_by_group: HashMap<usize, String> = group_members_present
            .iter()
            .filter_map(|(&group_number, ids)| {
                ids.first()
                    .cloned()
                    .map(|first_id| (group_number, first_id))
            })
            .collect();

        let mut result = Vec::new();
        for entry in entries {
            if let Some(&group_number) = ref_to_group.get(&entry.id) {
                let Some(present_ids) = group_members_present.get(&group_number) else {
                    result.push(entry);
                    continue;
                };

                if present_ids.len() == 1 {
                    result.push(entry);
                    continue;
                }

                if first_present_by_group.get(&group_number) == Some(&entry.id) {
                    #[allow(clippy::indexing_slicing, reason = "group_number is verified present")]
                    result.push(self.build_merged_compound_entry(
                        entry,
                        &compound_groups[&group_number],
                        &rendered_strings,
                        &compound_config,
                    ));
                }
            } else {
                result.push(entry);
            }
        }

        result
    }
}
