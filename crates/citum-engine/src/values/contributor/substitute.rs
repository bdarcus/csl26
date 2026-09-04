/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Author-substitution logic for contributor rendering.
//!
//! When a reference has no author, this module handles the fallback chain:
//! editor → title → translator, as configured by the style's `substitute` block.

use crate::processor::rendering::get_variable_key;
use crate::reference::Reference;
use crate::render::format::OutputFormat;
use crate::values::title::{render_substitute_title_text, resolve_substitute_text_case};
use crate::values::{ProcHints, ProcValues, RenderContext, RenderOptions};
use citum_schema::options::{
    RoleLabelPreset, SubstituteField, SubstituteKey, SubstituteTitleQuoteMode,
    SubstituteTitleRendering,
};
use citum_schema::reference::Title;
use citum_schema::reference::{
    AudioVisualType, ClassExtension, Contributor, ContributorRole as DataRole,
};
use citum_schema::template::{
    ContributorRole, ContributorRoles, Rendering, TemplateComponent, TemplateContributor,
    TemplateTitle, TitleType, WrapPunctuation,
};

/// Resolved value occupying the style's effective primary-contributor slot.
#[derive(Debug, Clone)]
pub(crate) enum EffectivePrimary {
    /// A scalar contributor payload, including existing semantic-author fallback.
    Contributor {
        contributor: Box<Contributor>,
        role: ContributorRole,
    },
    /// An ordered cross-role candidate assembled from the native contributor vector.
    Merged(ContributorRoles),
    /// A title field used after contributor candidates are exhausted.
    Title { title: Title, kind: TitleType },
}

enum ResolvedRole {
    BuiltIn(ContributorRole),
    Custom(String),
}

impl ResolvedRole {
    fn key(&self) -> &str {
        match self {
            Self::BuiltIn(role) => role.as_str(),
            Self::Custom(role) => role.as_str(),
        }
    }

    fn built_in(&self) -> Option<&ContributorRole> {
        match self {
            Self::BuiltIn(role) => Some(role),
            Self::Custom(_) => None,
        }
    }
}

fn normalize_role_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let canonical = trimmed
        .chars()
        .map(|ch| match ch {
            '_' => '-',
            other => other.to_ascii_lowercase(),
        })
        .collect::<String>();

    canonical
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        .then_some(canonical)
}

fn parse_known_role(value: &str) -> Option<ContributorRole> {
    Some(match value {
        "author" => ContributorRole::Author,
        "chair" => ContributorRole::Chair,
        "editor" => ContributorRole::Editor,
        "translator" => ContributorRole::Translator,
        "annotator" => ContributorRole::Annotator,
        "commentator" => ContributorRole::Commentator,
        "foreword-author" => ContributorRole::ForewordAuthor,
        "introduction-author" => ContributorRole::IntroductionAuthor,
        "afterword-author" => ContributorRole::AfterwordAuthor,
        "director" => ContributorRole::Director,
        "composer" => ContributorRole::Composer,
        "illustrator" => ContributorRole::Illustrator,
        "narrator" => ContributorRole::Narrator,
        "collection-editor" => ContributorRole::CollectionEditor,
        "container-author" => ContributorRole::ContainerAuthor,
        "editorial-director" => ContributorRole::EditorialDirector,
        "textual-editor" => ContributorRole::TextualEditor,
        "original-author" => ContributorRole::OriginalAuthor,
        "reviewed-author" => ContributorRole::ReviewedAuthor,
        "recipient" => ContributorRole::Recipient,
        "interviewer" => ContributorRole::Interviewer,
        "guest" => ContributorRole::Guest,
        "performer" => ContributorRole::Performer,
        "inventor" => ContributorRole::Inventor,
        "counsel" => ContributorRole::Counsel,
        "writer" => ContributorRole::Writer,
        _ => return None,
    })
}

fn resolve_role_key(value: &str) -> Option<ResolvedRole> {
    let canonical = normalize_role_key(value)?;
    Some(
        parse_known_role(&canonical)
            .map(ResolvedRole::BuiltIn)
            .unwrap_or(ResolvedRole::Custom(canonical)),
    )
}

fn lookup_role_contributor(
    reference: &Reference,
    role: &ResolvedRole,
) -> Option<citum_schema::reference::contributor::Contributor> {
    match role {
        ResolvedRole::BuiltIn(ContributorRole::Editor) => reference.editor(),
        ResolvedRole::BuiltIn(ContributorRole::Translator) => reference.translator(),
        ResolvedRole::BuiltIn(role) => {
            data_role_for_builtin(role).and_then(|data_role| reference.contributor(data_role))
        }
        ResolvedRole::Custom(role) => reference.contributor(data_role_for_custom(role)),
    }
}

fn data_role_for_builtin(role: &ContributorRole) -> Option<DataRole> {
    Some(match role {
        ContributorRole::Director => DataRole::Director,
        ContributorRole::Composer => DataRole::Composer,
        ContributorRole::Illustrator => DataRole::Illustrator,
        ContributorRole::Annotator => DataRole::Annotator,
        ContributorRole::Commentator => DataRole::Commentator,
        ContributorRole::ForewordAuthor => DataRole::ForewordAuthor,
        ContributorRole::IntroductionAuthor => DataRole::IntroductionAuthor,
        ContributorRole::AfterwordAuthor => DataRole::AfterwordAuthor,
        ContributorRole::Recipient => DataRole::Recipient,
        ContributorRole::Interviewer => DataRole::Interviewer,
        ContributorRole::Guest => DataRole::Guest,
        ContributorRole::Performer => DataRole::Performer,
        ContributorRole::Narrator => DataRole::Narrator,
        ContributorRole::Writer => DataRole::Writer,
        ContributorRole::Compiler => DataRole::Compiler,
        ContributorRole::ContainerAuthor
        | ContributorRole::CollectionEditor
        | ContributorRole::EditorialDirector
        | ContributorRole::TextualEditor
        | ContributorRole::OriginalAuthor
        | ContributorRole::ReviewedAuthor
        | ContributorRole::Chair
        | ContributorRole::Inventor
        | ContributorRole::Counsel => DataRole::Unknown(role.as_str().to_string()),
        ContributorRole::Unknown(role) => DataRole::Unknown(role.clone()),
        ContributorRole::Author
        | ContributorRole::Editor
        | ContributorRole::Translator
        | ContributorRole::Publisher
        | ContributorRole::Interviewee => return None,
        _ => return None,
    })
}

fn data_role_for_custom(role: &str) -> DataRole {
    match role {
        "compiler" => DataRole::Compiler,
        "performer" => DataRole::Performer,
        "narrator" => DataRole::Narrator,
        "host" => DataRole::Host,
        "producer" | "executive-producer" => DataRole::Producer,
        "writer" => DataRole::Writer,
        // Compatibility aliases for styles authored before these roles became
        // built-in style vocabulary.
        "annotator" => DataRole::Annotator,
        "commentator" => DataRole::Commentator,
        "foreword-author" => DataRole::ForewordAuthor,
        "introduction-author" => DataRole::IntroductionAuthor,
        "afterword-author" => DataRole::AfterwordAuthor,
        _ => DataRole::Unknown(role.to_string()),
    }
}

/// Resolve all multilingual names for a contributor using the current options.
///
/// Eliminates the copy-paste resolution pattern across Editor, Translator, and
/// primary-contributor paths.
pub(super) fn resolve_multilingual_for_contrib(
    contrib: &citum_schema::reference::contributor::Contributor,
    options: &RenderOptions<'_>,
) -> Vec<crate::reference::FlatName> {
    let mode = options
        .config
        .multilingual
        .as_ref()
        .and_then(|m| m.name_mode.as_ref());
    let preferred_transliteration = options
        .config
        .multilingual
        .as_ref()
        .and_then(|m| m.preferred_transliteration.as_deref());
    let preferred_script = options
        .config
        .multilingual
        .as_ref()
        .and_then(|m| m.preferred_script.as_ref());
    crate::values::resolve_multilingual_name(
        contrib,
        mode,
        preferred_transliteration,
        preferred_script,
        &options.locale.locale,
    )
}

struct SubstituteRoleLabelContext<'a, 'b, F> {
    component: &'a TemplateContributor,
    role: &'a ResolvedRole,
    names_count: usize,
    reference: &'a Reference,
    options: &'a RenderOptions<'b>,
    effective_rendering: &'a Rendering,
    fmt: &'a F,
    substitute: &'a citum_schema::options::Substitute,
}

fn substitute_role_label_preset(
    role: &ResolvedRole,
    options: &RenderOptions<'_>,
    substitute: &citum_schema::options::Substitute,
) -> Option<RoleLabelPreset> {
    substitute
        .contributor_role_form
        .as_deref()
        .and_then(|form| match form {
            "short" => Some(RoleLabelPreset::ShortSuffix),
            "short-comma" => Some(RoleLabelPreset::ShortSuffixComma),
            "long" => Some(RoleLabelPreset::LongSuffix),
            _ => None,
        })
        .or_else(|| {
            let contributors = options.config.contributors.as_ref()?;
            role.built_in()
                .and_then(|known| contributors.effective_role_label_preset(known))
        })
        .or_else(|| {
            let contributors = options.config.contributors.as_ref()?;
            role.built_in()
                .and_then(|known| contributors.default_role_label_preset(known))
        })
}

/// Resolve substitute-path role labels for a rendered fallback contributor.
fn resolve_substitute_role_labels<F: OutputFormat<Output = String>>(
    context: &SubstituteRoleLabelContext<'_, '_, F>,
) -> (Option<String>, Option<String>) {
    let SubstituteRoleLabelContext {
        component,
        role,
        names_count,
        reference,
        options,
        effective_rendering,
        fmt,
        substitute,
    } = context;
    if matches!(role.built_in(), Some(ContributorRole::Author)) {
        return (None, None);
    }
    if options.context != RenderContext::Bibliography
        || role
            .built_in()
            .is_some_and(|known| super::is_role_label_omitted(options, known))
    {
        return (None, None);
    }

    if substitute.contributor_role_form.is_none()
        && substitute.contributor_role_case.is_none()
        && let Some(known) = role.built_in()
        && options
            .config
            .contributors
            .as_ref()
            .is_some_and(|contributors| {
                contributors.role_label_presentation(known).is_some()
                    || contributors
                        .role
                        .as_ref()
                        .and_then(|role_options| role_options.defaults)
                        .and_then(|defaults| defaults.presentation_for(known))
                        .is_some()
            })
    {
        let mut role_component = (*component).clone();
        role_component.contributor = known.clone().into();
        return super::labels::resolve_role_labels::<F>(super::labels::RoleLabelContext {
            component: &role_component,
            role: known,
            reference,
            names_count: *names_count,
            effective_rendering,
            options,
            fmt: *fmt,
            role_omitted: super::is_role_label_omitted(options, known),
        });
    }

    let preset = substitute_role_label_preset(role, options, substitute);

    preset
        .and_then(|selected| {
            if component.contributor == ContributorRole::Author
                && matches!(
                    selected,
                    RoleLabelPreset::VerbPrefix | RoleLabelPreset::VerbShortPrefix
                )
            {
                return None;
            }

            role.built_in().map(|known| {
                super::labels::resolve_role_label_preset::<F>(
                    known,
                    selected,
                    *names_count,
                    super::labels::RoleLabelTermOptions {
                        gender: None,
                        text_case: substitute.contributor_role_case,
                    },
                    effective_rendering,
                    options,
                    fmt,
                )
            })
        })
        .unwrap_or((None, None))
}

/// Format a substitute contributor using the current role-aware config path.
#[allow(
    clippy::too_many_arguments,
    reason = "Role-aware substitute formatting needs shared engine state until this module is refactored."
)]
fn resolve_named_substitute<F: OutputFormat<Output = String>>(
    role: &ResolvedRole,
    contributor: &citum_schema::reference::contributor::Contributor,
    component: &TemplateContributor,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    reference: &Reference,
    effective_rendering: &Rendering,
    fmt: &F,
    substitute: &citum_schema::options::Substitute,
) -> Option<ProcValues<F::Output>> {
    let names_vec = resolve_multilingual_for_contrib(contributor, options);
    if names_vec.is_empty() {
        return None;
    }

    // Preserve the scalar author fast path. Semantic authors are not
    // substitutes, do not carry role labels, and should avoid constructing a
    // substituted variable key on every bibliography render.
    if matches!(role.built_in(), Some(ContributorRole::Author)) {
        let formatted = super::format_contributor_names(
            component,
            &ContributorRole::Author,
            &names_vec,
            reference,
            effective_rendering,
            options,
            hints,
        );
        return Some(ProcValues {
            value: crate::values::apply_abbreviation(formatted, options.abbreviation_map),
            prefix: None,
            suffix: None,
            url: crate::values::resolve_effective_url(
                component.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                citum_schema::options::LinkAnchor::Component,
            ),
            substituted_key: None,
            pre_formatted: false,
        });
    }

    let effective_name_order = component.name_order.as_ref().or_else(|| {
        role.built_in().and_then(|known| {
            options
                .config
                .contributors
                .as_ref()
                .and_then(|contributors| contributors.effective_role_name_order(known))
        })
    });

    // Priority chain for name_form:
    // 1. component.name_form (TemplateContributor-level override - highest priority)
    // 2. effective_rendering.name_form (from overrides, second priority)
    // 3. config (options-level fallback)
    let effective_name_form = component.name_form.or(effective_rendering.name_form);

    let name_overrides = super::names::NamesOverrides {
        name_order: effective_name_order,
        sort_separator: component.sort_separator.as_ref(),
        delimiter: component.delimiter.as_ref(),
        shorten: component.shorten.as_ref(),
        and: component.and.as_ref(),
        initialize_with: effective_rendering.initialize_with.as_ref(),
        name_form: effective_name_form,
        strip_periods: effective_rendering.strip_periods,
        item_language: crate::values::effective_item_language(reference),
    };
    let formatted =
        super::names::format_names(&names_vec, &component.form, options, &name_overrides, hints);
    let (prefix, suffix) = resolve_substitute_role_labels::<F>(&SubstituteRoleLabelContext {
        component,
        role,
        names_count: names_vec.len(),
        reference,
        options,
        effective_rendering,
        fmt,
        substitute,
    });

    let url = crate::values::resolve_effective_url(
        component.links.as_ref(),
        options.config.links.as_ref(),
        reference,
        citum_schema::options::LinkAnchor::Component,
    );

    let substituted_key = role.built_in().map_or_else(
        || Some(format!("contributor:{}", role.key())),
        |known| {
            get_variable_key(&TemplateComponent::Contributor(TemplateContributor {
                contributor: known.clone().into(),
                rendering: component.rendering.clone(),
                ..Default::default()
            }))
        },
    );

    Some(ProcValues {
        value: fmt.text(&formatted),
        prefix,
        suffix,
        url,
        substituted_key,
        pre_formatted: true,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "Substitute lookup and formatting share the same rendering state."
)]
fn resolve_contributor_substitute_for_role<F: OutputFormat<Output = String>>(
    role: &ResolvedRole,
    component: &TemplateContributor,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    reference: &Reference,
    effective_rendering: &Rendering,
    fmt: &F,
    substitute: &citum_schema::options::Substitute,
) -> Option<ProcValues<F::Output>> {
    let contributor = lookup_role_contributor(reference, role)?;
    resolve_named_substitute(
        role,
        &contributor,
        component,
        hints,
        options,
        reference,
        effective_rendering,
        fmt,
        substitute,
    )
}

/// Check if a role should be suppressed by role-substitute configuration.
///
/// Returns true if this role appears as a fallback in some other role's chain
/// AND that primary role has data on the reference.
pub(super) fn is_role_suppressed_by_substitute(
    role: &ContributorRole,
    substitute: &citum_schema::options::Substitute,
    reference: &Reference,
) -> bool {
    let role_str = role.as_str();

    for (primary_role_str, fallback_chain) in &substitute.role_substitute {
        // Check if this role is in the fallback chain
        if !fallback_chain
            .iter()
            .filter_map(|entry| resolve_role_key(entry))
            .any(|entry| entry.key() == role_str)
        {
            continue;
        }

        if let Some(primary_role) = resolve_role_key(primary_role_str)
            && lookup_role_contributor(reference, &primary_role).is_some()
        {
            return true;
        }
    }

    false
}

fn find_role_substitute_chain<'a>(
    substitute: &'a citum_schema::options::Substitute,
    primary_role: &ContributorRole,
) -> Option<&'a Vec<String>> {
    let primary_role_str = primary_role.as_str();

    substitute
        .role_substitute
        .get(primary_role_str)
        .or_else(|| {
            substitute
                .role_substitute
                .iter()
                .find_map(|(configured_role, fallback_chain)| {
                    resolve_role_key(configured_role)
                        .filter(|resolved| resolved.key() == primary_role_str)
                        .map(|_| fallback_chain)
                })
        })
}

/// Attempt to substitute a non-author contributor field via role-substitute fallback chain.
///
/// Returns `Some(ProcValues)` if a substitute from the chain was found, `None` if the chain
/// is exhausted with no result.
#[allow(
    clippy::too_many_arguments,
    reason = "Role-aware role-substitute needs shared engine state."
)]
pub(super) fn resolve_role_substitute<F: OutputFormat<Output = String>>(
    primary_role: &ContributorRole,
    component: &TemplateContributor,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    reference: &Reference,
    effective_rendering: &Rendering,
    fmt: &F,
    substitute: &citum_schema::options::Substitute,
) -> Option<ProcValues<F::Output>> {
    let fallback_chain = find_role_substitute_chain(substitute, primary_role)?;

    for fallback_role_str in fallback_chain {
        let Some(fallback_role) = resolve_role_key(fallback_role_str) else {
            continue;
        };

        if let Some(result) = resolve_contributor_substitute_for_role(
            &fallback_role,
            component,
            hints,
            options,
            reference,
            effective_rendering,
            fmt,
            substitute,
        ) {
            return Some(result);
        }
    }

    None
}

/// The `substituted_key` recorded on `ProcValues` for a title substitution,
/// keyed by which title slot filled the missing-author position.
fn substituted_key_for_title_type(title_type: &TitleType) -> &'static str {
    match title_type {
        TitleType::ParentSerial => "title:ParentSerial",
        _ => "title:Primary",
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Title-substitute rendering needs shared engine state until this module is refactored."
)]
fn resolve_title_substitute<F: OutputFormat<Output = String>>(
    title: Title,
    title_type: &TitleType,
    component: &TemplateContributor,
    options: &RenderOptions<'_>,
    reference: &Reference,
    fmt: &F,
    quote_in_citation: bool,
    substitute: &citum_schema::options::Substitute,
) -> ProcValues<F::Output> {
    let substituted_key = substituted_key_for_title_type(title_type);
    let title_str = title_substitute_text(title, options.context);
    // The substitute chain has no `TemplateTitle` to carry a per-component
    // `text-case:` override, so only the style's category-level `titles:`
    // config (keyed by reference type) applies here — see
    // `resolve_substitute_text_case`.
    let case = resolve_substitute_text_case(title_type, reference, options);
    let language = crate::values::title::effective_title_language(title_type, reference);
    // Bibliography-context styles opted into `title-rendering: from-template`
    // derive formatting from the reference type's own resolved `title:
    // primary` node instead of category-only config — see
    // `SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md`.
    let template_rendering = resolve_template_derived_title_rendering(
        title_type,
        reference,
        language.as_deref(),
        options,
        substitute,
    );
    let quoted = if let Some(rendering) = &template_rendering {
        rendering.quote == Some(true)
    } else {
        options.context == RenderContext::Citation && quote_in_citation
    };
    let quote_depth = usize::from(quoted);
    // Route through the same Djot-aware pipeline as `TemplateTitle::values`
    // so markup (e.g. `[...]{.nocase}`) and case-protection are honoured
    // here too, rather than leaking the raw title string verbatim.
    let (rendered, has_explicit_link, is_djot) =
        render_substitute_title_text(&title_str, fmt, case, quote_depth, language.as_deref());
    let rendered = if is_djot {
        rendered
    } else {
        fmt.text(&rendered)
    };
    let rendered = if let Some(rendering) = &template_rendering {
        // From-template mode faithfully applies the merged node rendering:
        // quote and emph both apply if the node declares both, superseding
        // div-011's either/or for opted-in styles.
        let mut output = rendered;
        if rendering.emph == Some(true) {
            output = fmt.emph(output);
        }
        if rendering.strong == Some(true) {
            output = fmt.strong(output);
        }
        if rendering.small_caps == Some(true) {
            output = fmt.small_caps(output);
        }
        output
    } else if quoted {
        rendered
    } else {
        // Category-level `titles:` emphasis (emph/strong/small-caps) that a
        // normal `title:` component would pick up from
        // `get_effective_rendering` — the substitute chain renders through a
        // contributor component, so it never reaches that path and must
        // apply it here. Order mirrors
        // `render_component_detailed_with_format_and_renderer`. Skipped when
        // quoting: per div-011, quote and category emphasis are either/or
        // for a substituted title, so the default unconditional-quote path
        // stays byte-identical.
        apply_substitute_title_emphasis(
            rendered,
            title_type,
            reference,
            language.as_deref(),
            options,
            fmt,
        )
    };
    let value = if quoted {
        let marks = crate::render::format::QuoteMarks::from(&options.locale.grammar_options);
        fmt.quote(rendered, &marks)
    } else {
        rendered
    };

    let url = crate::values::resolve_effective_url(
        component.links.as_ref(),
        options.config.links.as_ref(),
        reference,
        citum_schema::options::LinkAnchor::Title,
    );

    ProcValues {
        value,
        prefix: None,
        suffix: None,
        url: if has_explicit_link { None } else { url },
        substituted_key: Some(substituted_key.to_string()),
        pre_formatted: true,
    }
}

/// Apply the reference's title-category `emph` / `strong` / `small-caps` to a
/// substituted title, mirroring the order
/// `render_component_detailed_with_format_and_renderer` applies them for a
/// normal (non-substitute) title component. Called only when the title is not
/// being quoted — see the div-011 either/or note at the call site.
fn apply_substitute_title_emphasis<F: OutputFormat<Output = String>>(
    rendered: F::Output,
    title_type: &TitleType,
    reference: &Reference,
    language: Option<&str>,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> F::Output {
    let ref_type = reference.ref_type();
    let Some(rendering) = crate::render::component::get_title_category_title_rendering(
        title_type,
        Some(&ref_type),
        language,
        &options.config,
    ) else {
        return rendered;
    };

    let mut output = rendered;
    if rendering.emph == Some(true) {
        output = fmt.emph(output);
    }
    if rendering.strong == Some(true) {
        output = fmt.strong(output);
    }
    if rendering.small_caps == Some(true) {
        output = fmt.small_caps(output);
    }
    output
}

/// Resolve whether a substituted title should be quoted per the reference's
/// title-category rendering (`titles:` config), mirroring how a normal
/// (non-substitute) title component resolves quoting — see
/// `effective_title_quote_depth`. Defaults to no quoting when the style has
/// no `titles:` config for the category, matching that function's default.
fn resolve_category_quote(reference: &Reference, options: &RenderOptions<'_>) -> bool {
    let ref_type = reference.ref_type();
    let lang = reference.language();
    crate::render::component::get_title_category_rendering(
        &TitleType::Primary,
        Some(&ref_type),
        lang.as_deref(),
        &options.config,
    )
    .and_then(|rendering| rendering.quote)
    .unwrap_or(false)
}

/// Find the resolved bibliography template's own node for `title_type`
/// (e.g. `title: primary`), honoring `render_when` branches the same way
/// the normal render path resolves them — see
/// [`crate::values::group_condition_matches`]. Returns the first matching
/// node in document order, matching how the renderer walks a template.
fn find_template_title_node<'a>(
    template: &'a [TemplateComponent],
    title_type: &TitleType,
    reference: &Reference,
) -> Option<&'a TemplateTitle> {
    for component in template {
        if component.rendering().suppress == Some(true) {
            continue;
        }
        match component {
            TemplateComponent::Title(title) if title.title == *title_type => {
                return Some(title);
            }
            TemplateComponent::Group(group) => {
                if group.render_when.as_ref().is_some_and(|condition| {
                    !crate::values::group_condition_matches(reference, condition)
                }) {
                    continue;
                }
                if let Some(found) = find_template_title_node(&group.group, title_type, reference) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Transfer only the formatting fields that describe how a title's *text*
/// is presented — never structural fields like `prefix`/`suffix`/`form`,
/// which are positional and meaningless detached from the node's slot. A
/// `wrap` transfers only when it is quoting punctuation: `wrap: parentheses`
/// / `wrap: brackets` on a template node is positional (correct next to a
/// specific sibling), not title formatting, so it is deliberately ignored
/// here — see `SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md` §4.2/§4.4.
fn merge_substitute_title_rendering(base: &mut Rendering, node: &Rendering) {
    if node.emph.is_some() {
        base.emph = node.emph;
    }
    if node.quote.is_some() {
        base.quote = node.quote;
    }
    if node.strong.is_some() {
        base.strong = node.strong;
    }
    if node.small_caps.is_some() {
        base.small_caps = node.small_caps;
    }
    if let Some(wrap) = &node.wrap
        && wrap.punctuation == WrapPunctuation::Quotes
    {
        base.quote = Some(true);
    }
}

/// Resolve a substituted title's bibliography-context rendering from the
/// reference type's own resolved template node, merged over the title
/// category's rendering (category-then-node precedence, matching
/// `effective_title_quote_depth`). Returns `None` when the style has not
/// opted in (`substitute.title-rendering: from-template`), when no
/// resolved template was threaded through for this render, or when the
/// template has no matching title node.
fn resolve_template_derived_title_rendering(
    title_type: &TitleType,
    reference: &Reference,
    language: Option<&str>,
    options: &RenderOptions<'_>,
    substitute: &citum_schema::options::Substitute,
) -> Option<Rendering> {
    if options.context != RenderContext::Bibliography {
        return None;
    }
    if substitute.title_rendering != Some(SubstituteTitleRendering::FromTemplate) {
        return None;
    }
    let template = options.substitute_title_template?;
    let node = find_template_title_node(template, title_type, reference)?;
    let ref_type = reference.ref_type();
    let mut rendering = crate::render::component::get_title_category_rendering(
        title_type,
        Some(&ref_type),
        language,
        &options.config,
    )
    .unwrap_or_default();
    merge_substitute_title_rendering(&mut rendering, &node.rendering);
    Some(rendering)
}

fn title_substitute_text(title: Title, context: RenderContext) -> String {
    if context != RenderContext::Citation {
        return title.to_string();
    }

    match title {
        Title::Structured(s) => s.main,
        Title::MultiStructured(v) => v
            .into_iter()
            .next()
            .map(|(_, s)| s.main)
            .unwrap_or_default(),
        Title::Shorthand(abbr, _) => abbr,
        _ => title.to_string(),
    }
}

fn resolve_parent_serial_title(reference: &Reference) -> Option<Title> {
    match reference.extension() {
        ClassExtension::SerialComponent(_)
        | ClassExtension::LegalCase(_)
        | ClassExtension::Treaty(_) => reference.container_title(),
        _ => None,
    }
}

fn native_type_key(reference: &Reference) -> Option<&'static str> {
    reference
        .as_audio_visual()
        .and_then(|work| match work.r#type {
            AudioVisualType::Film => Some("film"),
            AudioVisualType::Episode => Some("episode"),
            AudioVisualType::Recording => Some("recording"),
            AudioVisualType::Broadcast => Some("broadcast"),
            _ => None,
        })
}

fn exact_type_candidates<'a>(
    reference: &Reference,
    substitute: &'a citum_schema::options::Substitute,
) -> Option<&'a [SubstituteKey]> {
    if let Some(candidates) =
        native_type_key(reference).and_then(|key| substitute.overrides.get(key))
    {
        return Some(candidates.as_slice());
    }

    substitute
        .overrides
        .get(&reference.ref_type())
        .map(citum_schema::options::SubstituteCandidates::as_slice)
}

fn contributor_for_candidate(reference: &Reference, role: &ContributorRole) -> Option<Contributor> {
    lookup_role_contributor(reference, &ResolvedRole::BuiltIn(role.clone()))
}

/// An [`EffectivePrimary`] candidate paired with the multilingual names that
/// justify selecting it, resolved together in a single pass.
///
/// `resolve_candidate` needs a candidate's names only to test whether it is
/// non-empty (to decide whether the candidate qualifies); keeping the
/// resolved vector alongside the selected variant lets
/// [`effective_primary_names`] reuse it instead of re-running the same
/// (potentially expensive, suppression-aware) name assembly a second time.
struct EffectivePrimaryResolution {
    primary: EffectivePrimary,
    names: Vec<crate::reference::FlatName>,
}

fn resolve_candidate(
    candidate: &SubstituteKey,
    reference: &Reference,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Option<EffectivePrimaryResolution> {
    match candidate {
        SubstituteKey::Contributor(candidate) => {
            if let Some(role) = candidate.contributor.as_single() {
                contributor_for_candidate(reference, role).and_then(|contributor| {
                    let names =
                        super::merged::semantic_contributor_names(&contributor, config, locale);
                    (!names.is_empty()).then(|| EffectivePrimaryResolution {
                        primary: EffectivePrimary::Contributor {
                            contributor: Box::new(contributor),
                            role: role.clone(),
                        },
                        names,
                    })
                })
            } else {
                let component = TemplateContributor {
                    contributor: candidate.contributor.clone(),
                    ..Default::default()
                };
                let names = super::merged::semantic_names(&component, reference, config, locale);
                (!names.is_empty()).then(|| EffectivePrimaryResolution {
                    primary: EffectivePrimary::Merged(candidate.contributor.clone()),
                    names,
                })
            }
        }
        SubstituteKey::Field(field) => match field {
            SubstituteField::CollectionEditor => contributor_for_candidate(
                reference,
                &ContributorRole::CollectionEditor,
            )
            .and_then(|contributor| {
                let names = super::merged::semantic_contributor_names(&contributor, config, locale);
                (!names.is_empty()).then(|| EffectivePrimaryResolution {
                    primary: EffectivePrimary::Contributor {
                        contributor: Box::new(contributor),
                        role: ContributorRole::CollectionEditor,
                    },
                    names,
                })
            }),
            SubstituteField::Editor => reference.local_editor().and_then(|contributor| {
                let names = super::merged::semantic_contributor_names(&contributor, config, locale);
                (!names.is_empty()).then(|| EffectivePrimaryResolution {
                    primary: EffectivePrimary::Contributor {
                        contributor: Box::new(contributor),
                        role: ContributorRole::Editor,
                    },
                    names,
                })
            }),
            SubstituteField::ParentSerial => {
                resolve_parent_serial_title(reference).map(|title| EffectivePrimaryResolution {
                    primary: EffectivePrimary::Title {
                        title,
                        kind: TitleType::ParentSerial,
                    },
                    names: Vec::new(),
                })
            }
            SubstituteField::Title => reference.title().map(|title| EffectivePrimaryResolution {
                primary: EffectivePrimary::Title {
                    title,
                    kind: TitleType::Primary,
                },
                names: Vec::new(),
            }),
            SubstituteField::Translator => reference.translator().and_then(|contributor| {
                let names = super::merged::semantic_contributor_names(&contributor, config, locale);
                (!names.is_empty()).then(|| EffectivePrimaryResolution {
                    primary: EffectivePrimary::Contributor {
                        contributor: Box::new(contributor),
                        role: ContributorRole::Translator,
                    },
                    names,
                })
            }),
        },
    }
}

/// Resolve the effective-primary candidate and its names together, computing
/// each candidate's name assembly at most once. Shared by [`effective_primary`]
/// and [`effective_primary_names`] so neither re-derives the other's work.
fn resolve_effective_primary(
    reference: &Reference,
    substitute: &citum_schema::options::Substitute,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Option<EffectivePrimaryResolution> {
    if let Some(candidates) = exact_type_candidates(reference, substitute) {
        for candidate in candidates {
            if let Some(resolved) = resolve_candidate(candidate, reference, config, locale) {
                return Some(resolved);
            }
        }
    }

    if let Some(contributor) = reference.author() {
        let names = super::merged::semantic_contributor_names(&contributor, config, locale);
        if !names.is_empty() {
            return Some(EffectivePrimaryResolution {
                primary: EffectivePrimary::Contributor {
                    contributor: Box::new(contributor),
                    role: ContributorRole::Author,
                },
                names,
            });
        }
    }

    substitute
        .candidates()
        .iter()
        .find_map(|candidate| resolve_candidate(candidate, reference, config, locale))
}

/// Resolve the single effective-primary value shared by rendering and semantic consumers.
pub(crate) fn effective_primary(
    reference: &Reference,
    substitute: &citum_schema::options::Substitute,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Option<EffectivePrimary> {
    resolve_effective_primary(reference, substitute, config, locale)
        .map(|resolved| resolved.primary)
}

/// Resolve effective-primary names for sorting, matching, and disambiguation.
pub(crate) fn effective_primary_names(
    reference: &Reference,
    substitute: &citum_schema::options::Substitute,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Vec<crate::reference::FlatName> {
    resolve_effective_primary(reference, substitute, config, locale)
        .map(|resolved| resolved.names)
        .unwrap_or_default()
}

/// Attempt to substitute an empty author field with editor, title, or translator.
///
/// Returns `Some(ProcValues)` if a substitute was found, `None` if the chain
/// is exhausted with no result (caller should then return `None` from `values()`).
pub(super) fn resolve_author_substitute<F: OutputFormat<Output = String>>(
    component: &TemplateContributor,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    reference: &Reference,
    effective_rendering: &Rendering,
    fmt: &F,
    substitute: &citum_schema::options::Substitute,
) -> Option<ProcValues<F::Output>> {
    match effective_primary(reference, substitute, &options.config, options.locale) {
        Some(EffectivePrimary::Contributor { contributor, role }) => resolve_named_substitute(
            &ResolvedRole::BuiltIn(role),
            &contributor,
            component,
            hints,
            options,
            reference,
            effective_rendering,
            fmt,
            substitute,
        ),
        Some(EffectivePrimary::Merged(roles)) => {
            let mut merged = component.clone();
            merged.contributor = roles;
            merged.merge = None;
            let mut values = super::merged::values(
                &merged,
                reference,
                hints,
                options,
                effective_rendering,
                fmt,
            )?;
            values.substituted_key = Some("contributor:effective-primary".to_string());
            Some(values)
        }
        Some(EffectivePrimary::Title { title, kind }) => {
            let quote_in_citation = matches!(kind, TitleType::Primary)
                && match substitute.title_quote {
                    Some(SubstituteTitleQuoteMode::ByCategory) => {
                        resolve_category_quote(reference, options)
                    }
                    Some(SubstituteTitleQuoteMode::Always) | None => true,
                };
            Some(resolve_title_substitute(
                title,
                &kind,
                component,
                options,
                reference,
                fmt,
                quote_in_citation,
                substitute,
            ))
        }
        None => None,
    }
}
