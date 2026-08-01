/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Sentence-initial context handling for bibliography entries and note-style
//! citation prefixes. Capitalizes the first word (or role label) of a
//! component when it appears at the start of an entry or note prefix.

use super::super::Renderer;
use crate::render::ProcTemplateComponent;
use crate::render::bibliography::{append_rendered_component, component_starts_new_sentence};
use crate::render::component::render_component_with_format;
use crate::render::punctuation::strong_terminal_comma_policy;
use crate::values::RenderContext;
use citum_schema::NoteStartTextCase;
use citum_schema::locale::GeneralTerm;
use citum_schema::options::titles::TextCase;
use citum_schema::template::TemplateComponent;

impl Renderer<'_> {
    pub(super) fn apply_sentence_initial_context<F>(
        &self,
        components: &mut [ProcTemplateComponent],
        context: RenderContext,
        note_start_text_case: Option<NoteStartTextCase>,
    ) where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        match context {
            RenderContext::Bibliography => {
                self.apply_bibliography_sentence_initial_context::<F>(components);
            }
            RenderContext::Citation => {
                self.apply_note_start_sentence_initial_context(components, note_start_text_case);
            }
        }
    }

    fn apply_bibliography_sentence_initial_context<F>(
        &self,
        components: &mut [ProcTemplateComponent],
    ) where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let punctuation_in_quote = components
            .first()
            .and_then(|component| component.config.as_ref())
            .is_some_and(|config| config.punctuation_in_quote);
        let strong_terminal_comma_policy = strong_terminal_comma_policy(
            components
                .first()
                .and_then(|component| component.config.as_deref()),
        );
        let default_separator = components
            .first()
            .and_then(|component| component.bibliography_config.as_ref())
            .and_then(|bib| bib.separator.as_deref())
            .unwrap_or(". ")
            .to_string();
        let close_quote = components
            .first()
            .map(|component| component.quote_marks.close.clone())
            .unwrap_or_else(|| "\u{201D}".to_string());

        let mut entry_output = String::new();
        for component in components.iter_mut() {
            let rendered = render_component_with_format::<F>(component);
            if rendered.is_empty() {
                continue;
            }

            if component_starts_new_sentence::<F>(
                &entry_output,
                &rendered,
                &default_separator,
                punctuation_in_quote,
                &close_quote,
            ) {
                component.sentence_initial = true;
                self.apply_sentence_initial_transform(component, None);
            }

            let rendered = render_component_with_format::<F>(component);
            append_rendered_component::<F>(
                &mut entry_output,
                &rendered,
                &default_separator,
                punctuation_in_quote,
                strong_terminal_comma_policy,
                &close_quote,
            );
        }
    }

    fn apply_note_start_sentence_initial_context(
        &self,
        components: &mut [ProcTemplateComponent],
        note_start_text_case: Option<NoteStartTextCase>,
    ) {
        let Some(text_case) = note_start_text_case else {
            return;
        };

        for component in components.iter_mut() {
            if !self.is_note_start_term_component(component) {
                continue;
            }

            component.sentence_initial = true;
            self.apply_sentence_initial_transform(component, Some(text_case));
            break;
        }
    }

    fn apply_sentence_initial_transform(
        &self,
        component: &mut ProcTemplateComponent,
        note_start_text_case: Option<NoteStartTextCase>,
    ) {
        if !component.sentence_initial {
            return;
        }

        let locale = Some(self.locale.locale.as_str());
        match &component.template_component {
            TemplateComponent::Contributor(contributor) => {
                let case =
                    crate::values::text_case::resolve_text_case(TextCase::CapitalizeFirst, locale);
                if let Some(prefix) = component.prefix.as_mut() {
                    // Explicit template prefix (e.g. ". Translated by ") — capitalize it.
                    *prefix = crate::values::text_case::apply_text_case_with_language(
                        prefix, case, locale,
                    );
                } else if contributor.rendering.prefix.is_some() {
                    // A static YAML `prefix:` (e.g. "Narrated by ") already supplies
                    // the sentence-initial capital as authored. The contributor's own
                    // value is a name (or literal descriptive text like "the author")
                    // and must not also be capitalized on top of it.
                } else {
                    // No explicit prefix: the role label (e.g. "edited by ") is baked
                    // into the rendered value.  Capitalize the first word so that
                    // sentence-initial contributors read "Edited by …" not "edited by …".
                    component.value = if component.pre_formatted {
                        crate::values::text_case::apply_text_case_markup_aware_with_language(
                            &component.value,
                            case,
                            locale,
                        )
                    } else {
                        crate::values::text_case::apply_text_case_with_language(
                            &component.value,
                            case,
                            locale,
                        )
                    };
                }
            }
            // Pre-formatted groups keep child identity only in the template
            // structure. Auto-case title groups and contributor-role prose,
            // but keep date/term/data groups' rendered casing intact.
            TemplateComponent::Group(group) if group_supports_sentence_initial_casing(group) => {
                let case =
                    crate::values::text_case::resolve_text_case(TextCase::CapitalizeFirst, locale);
                // Groups are always pre-formatted (rendered markup); use the markup-aware
                // variant so HTML tags and LaTeX/Typst command prefixes are not corrupted.
                component.value =
                    crate::values::text_case::apply_text_case_markup_aware_with_language(
                        &component.value,
                        case,
                        locale,
                    );
            }
            TemplateComponent::Message(message) if !message.message.starts_with("term.") => {
                let case =
                    crate::values::text_case::resolve_text_case(TextCase::CapitalizeFirst, locale);
                component.value = if component.pre_formatted {
                    crate::values::text_case::apply_text_case_markup_aware_with_language(
                        &component.value,
                        case,
                        locale,
                    )
                } else {
                    crate::values::text_case::apply_text_case_with_language(
                        &component.value,
                        case,
                        locale,
                    )
                };
            }
            TemplateComponent::Term(_) | TemplateComponent::Message(_)
                if self.is_note_start_term_component(component) =>
            {
                if let Some(case) = note_start_text_case {
                    component.value = crate::values::text_case::apply_note_start_text_case(
                        &component.value,
                        case,
                        locale,
                    );
                }
            }
            _ => {}
        }
    }

    fn is_note_start_term_component(&self, component: &ProcTemplateComponent) -> bool {
        matches!(
            &component.template_component,
            TemplateComponent::Term(term) if term.term == GeneralTerm::Ibid
        ) || matches!(
            &component.template_component,
            TemplateComponent::Message(message) if message.message == "term.ibid"
        )
    }
}

fn group_supports_sentence_initial_casing(group: &citum_schema::template::TemplateGroup) -> bool {
    group.group.iter().find_map(first_sentence_casing_candidate) == Some(true)
}

fn first_sentence_casing_candidate(component: &TemplateComponent) -> Option<bool> {
    if component.rendering().suppress == Some(true) {
        return None;
    }

    match component {
        TemplateComponent::Contributor(contributor) => Some(matches!(
            contributor.form,
            citum_schema::template::ContributorForm::Verb
                | citum_schema::template::ContributorForm::VerbShort
        )),
        TemplateComponent::Title(_) => Some(true),
        TemplateComponent::Group(group) => {
            group.group.iter().find_map(first_sentence_casing_candidate)
        }
        _ => Some(false),
    }
}
