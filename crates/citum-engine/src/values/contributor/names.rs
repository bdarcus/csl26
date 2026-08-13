/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Name-formatting helpers for contributor rendering.

use crate::values::{ProcHints, RenderOptions};
use citum_schema::options::contributors::NameForm;
use citum_schema::options::{
    AndOptions, AndOtherOptions, ContributorConfig, DemoteNonDroppingParticle, DisplayAsSort,
    ShortenListOptions, TwoNameDelimiterPolicy,
};
use citum_schema::template::{ContributorForm, NameOrder};
use std::borrow::Cow;
use unicode_script::{Script, UnicodeScript};

/// Configuration for formatting a single name.
pub(crate) struct NameFormatContext<'a> {
    pub(crate) display_as_sort: Option<DisplayAsSort>,
    pub(crate) name_order: Option<&'a NameOrder>,
    /// Total number of names in the full contributor list (before et-al
    /// truncation), used by [`NameOrder::FamilyFirstExceptLast`] to identify
    /// the last name.
    pub(crate) total_names: usize,
    pub(crate) initialize_with: Option<&'a String>,
    pub(crate) initialize_with_hyphen: Option<bool>,
    pub(crate) name_form: Option<NameForm>,
    pub(crate) demote_ndp: Option<&'a DemoteNonDroppingParticle>,
    pub(crate) sort_separator: Option<&'a String>,
    pub(crate) component_sort_separator: Option<&'a String>,
    pub(crate) script_configs:
        Option<&'a std::collections::HashMap<String, citum_schema::options::ScriptConfig>>,
    pub(crate) integral_name_state: Option<citum_schema::citation::IntegralNameState>,
    pub(crate) org_abbreviation_state: Option<citum_schema::citation::IntegralNameState>,
    pub(crate) use_integral_short_name: bool,
    pub(crate) short_name_display: Option<citum_schema::options::ShortNameDisplay>,
    pub(crate) subsequent_form: Option<citum_schema::options::SubsequentNameForm>,
    /// Whether a name suffix's own trailing period is stripped before
    /// rendering. See [`NamesOverrides::strip_periods`].
    pub(crate) strip_periods: bool,
}

/// Per-call template overrides passed to [`format_names`].
///
/// Bundles the optional override parameters that come from a
/// `TemplateContributor` so that call sites do not need to spell out each
/// one individually.
pub struct NamesOverrides<'a> {
    /// Override for name display order (given-first vs family-first).
    pub name_order: Option<&'a NameOrder>,
    /// Override for the sort separator (e.g. `","` or `" "`).
    pub sort_separator: Option<&'a String>,
    /// Override for the semantic or literal delimiter between names.
    pub delimiter: Option<&'a citum_schema::template::DelimiterPunctuation>,
    /// Override for et-al shortening options.
    pub shorten: Option<&'a ShortenListOptions>,
    /// Override for the "and" conjunction between names.
    pub and: Option<&'a AndOptions>,
    /// Override for the `initialize-with` string used to form initials.
    pub initialize_with: Option<&'a String>,
    /// Override for the name form (full, initials, family-only).
    pub name_form: Option<NameForm>,
    /// Override for whether a name suffix's own trailing period (e.g. the
    /// literal "Jr." stored in source data) is stripped before rendering.
    /// Styles that don't punctuate name suffixes (e.g. GB/T 7714) set this
    /// so a suffix immediately followed by a list delimiter doesn't read as
    /// double-punctuated ("Jr.，" instead of "Jr，").
    pub strip_periods: Option<bool>,
    /// Effective item language used to realize semantic contributor delimiters.
    pub item_language: Option<String>,
}

/// Prefix and suffix attached to a selected name before list joining.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct NameDecoration {
    /// Text emitted immediately before the formatted name.
    pub(super) prefix: String,
    /// Text emitted immediately after the formatted name.
    pub(super) suffix: String,
}

/// Return the original indexes retained by et-al selection.
pub(super) fn selected_name_indices(
    name_count: usize,
    shorten: Option<&ShortenListOptions>,
    hints: &ProcHints,
) -> Vec<usize> {
    let (first, _, last) = partition_et_al_indices(name_count, shorten, hints);
    first.into_iter().chain(last).collect()
}

/// Partition name indexes into (`first`, `use_et_al`, `last`) based on et-al options.
fn partition_et_al_indices(
    name_count: usize,
    shorten: Option<&ShortenListOptions>,
    hints: &ProcHints,
) -> (Vec<usize>, bool, Vec<usize>) {
    if let Some(opts) = shorten {
        // Determine effective min/use_first based on citation position.
        let is_subsequent = matches!(
            hints.position,
            Some(
                citum_schema::citation::Position::Subsequent
                    | citum_schema::citation::Position::Ibid
                    | citum_schema::citation::Position::IbidWithLocator
            )
        );
        let effective_min_threshold = if is_subsequent {
            opts.subsequent_min.unwrap_or(opts.min) as usize
        } else {
            opts.min as usize
        };
        let effective_use_first = if is_subsequent {
            opts.subsequent_use_first.unwrap_or(opts.use_first) as usize
        } else {
            opts.use_first as usize
        };

        // When min_names_to_show is set (name expansion disambiguation),
        // determine effective threshold for et-al application.
        let effective_min = if let Some(expanded) = hints.min_names_to_show {
            expanded.max(effective_use_first)
        } else {
            effective_use_first
        };

        // Apply et-al only if the list exceeds the minimum threshold
        if name_count >= effective_min_threshold {
            if effective_min >= name_count {
                ((0..name_count).collect(), false, Vec::new())
            } else {
                let first = (0..effective_min).collect();
                let last = if let Some(ul) = opts.use_last {
                    let take_last = ul as usize;
                    let skip = std::cmp::max(effective_min, name_count.saturating_sub(take_last));
                    (skip..name_count).collect()
                } else {
                    Vec::new()
                };
                (first, true, last)
            }
        } else {
            ((0..name_count).collect(), false, Vec::new())
        }
    } else {
        ((0..name_count).collect(), false, Vec::new())
    }
}

/// Partition names into (`first_names`, `use_et_al`, `last_names`) based on et-al options.
fn partition_et_al<'a>(
    names: &'a [crate::reference::FlatName],
    shorten: Option<&ShortenListOptions>,
    hints: &ProcHints,
) -> (
    Vec<&'a crate::reference::FlatName>,
    bool,
    Vec<&'a crate::reference::FlatName>,
) {
    let (first, use_et_al, last) = partition_et_al_indices(names.len(), shorten, hints);
    (
        first
            .into_iter()
            .filter_map(|index| names.get(index))
            .collect(),
        use_et_al,
        last.into_iter()
            .filter_map(|index| names.get(index))
            .collect(),
    )
}

/// Resolve whether the configured rule places a delimiter before a conjunction.
fn delimiter_precedes_conjunction(
    rule: Option<&citum_schema::options::DelimiterPrecedesLast>,
    displayed_name_count: usize,
    display_as_sort: Option<DisplayAsSort>,
) -> bool {
    use citum_schema::options::DelimiterPrecedesLast;

    match rule {
        Some(DelimiterPrecedesLast::Always) => true,
        Some(DelimiterPrecedesLast::Never) => false,
        Some(DelimiterPrecedesLast::Contextual) | None => displayed_name_count >= 3,
        Some(DelimiterPrecedesLast::AfterInvertedName) => display_as_sort.is_some_and(|display| {
            matches!(display, DisplayAsSort::All)
                || (matches!(display, DisplayAsSort::First) && displayed_name_count == 2)
        }),
    }
}

/// Return whether a style policy suppresses an otherwise configured delimiter.
fn suppresses_two_name_delimiter(
    policy: TwoNameDelimiterPolicy,
    name_count: usize,
    name_order: Option<&NameOrder>,
    context: crate::values::RenderContext,
) -> bool {
    name_count == 2
        && matches!(
            policy,
            TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst
        )
        && (context == crate::values::RenderContext::Citation
            || matches!(name_order, Some(NameOrder::GivenFirst)))
}

/// Inputs used to resolve the delimiter before a name-list conjunction.
struct ConjunctionDelimiterContext<'a> {
    rule: Option<&'a citum_schema::options::DelimiterPrecedesLast>,
    policy: TwoNameDelimiterPolicy,
    contributor_count: usize,
    display_as_sort: Option<DisplayAsSort>,
    name_order: Option<&'a NameOrder>,
    render_context: crate::values::RenderContext,
}

/// Join a list of formatted names with a conjunction and delimiter rules.
fn join_names_with_conjunction(
    formatted_first: &[String],
    and_str: Option<&str>,
    delimiter: &str,
    delimiter_context: &ConjunctionDelimiterContext<'_>,
) -> String {
    match and_str {
        None => {
            // No conjunction - just join all with delimiter
            formatted_first.join(delimiter)
        }
        Some(conjunction) => {
            if let Some((last, rest)) = formatted_first.split_last() {
                let displayed_name_count = formatted_first.len();
                let use_delimiter = delimiter_precedes_conjunction(
                    delimiter_context.rule,
                    displayed_name_count,
                    delimiter_context.display_as_sort,
                ) && !suppresses_two_name_delimiter(
                    delimiter_context.policy,
                    delimiter_context.contributor_count,
                    delimiter_context.name_order,
                    delimiter_context.render_context,
                );
                let leading = if let [only] = rest {
                    Cow::Borrowed(only.as_str())
                } else {
                    Cow::Owned(rest.join(delimiter))
                };
                if use_delimiter {
                    format!("{leading}{delimiter}{conjunction} {last}")
                } else {
                    format!("{leading} {conjunction} {last}")
                }
            } else {
                String::new()
            }
        }
    }
}

/// Parameters controlling et-al abbreviation formatting.
struct EtAlContext<'a> {
    and_others: AndOtherOptions,
    delimiter: &'a str,
    delimiter_precedes: Option<&'a citum_schema::options::DelimiterPrecedesLast>,
    first_count: usize,
}

/// Apply et-al suffix or return result unchanged.
fn apply_et_al(
    result: String,
    formatted_last: &[String],
    et_al: EtAlContext<'_>,
    ctx: &NameFormatContext,
    locale: &citum_schema::locale::Locale,
) -> String {
    use citum_schema::options::DelimiterPrecedesLast;

    if !formatted_last.is_empty() {
        // et-al-use-last: result + ellipsis + last names. citeproc-js places
        // the configured name delimiter before the ellipsis whenever more
        // than one name is shown before it (continuing the same list
        // punctuation used between those names); a single shown name is
        // followed by a plain space instead. This placement does not consult
        // `delimiter-precedes-et-al` — citeproc-js ignores that option here.
        let joined_last = formatted_last.join(et_al.delimiter);
        return if et_al.first_count > 1 {
            format!("{result}{}… {joined_last}", et_al.delimiter)
        } else {
            format!("{result} … {joined_last}")
        };
    }

    // Determine delimiter before "et al." based on delimiter_precedes_et_al option
    let use_delimiter = match et_al.delimiter_precedes {
        Some(DelimiterPrecedesLast::Always) => true,
        Some(DelimiterPrecedesLast::Never) => false,
        Some(DelimiterPrecedesLast::AfterInvertedName) => {
            // Use delimiter if last displayed name was inverted (family-first)
            ctx.display_as_sort.as_ref().is_some_and(|das| {
                matches!(das, DisplayAsSort::All)
                    || (matches!(das, DisplayAsSort::First) && et_al.first_count == 1)
            })
        }
        Some(DelimiterPrecedesLast::Contextual) | None => {
            // Default: use delimiter only if more than one name displayed
            et_al.first_count > 1
        }
    };

    let and_others_term = match et_al.and_others {
        AndOtherOptions::EtAl => locale.et_al(),
        AndOtherOptions::Text => locale.et_al().trim_end_matches('.'),
    };

    if use_delimiter {
        format!("{result}{}{and_others_term}", et_al.delimiter)
    } else {
        format!("{result} {and_others_term}")
    }
}

/// Format a list of names according to style options.
///
/// # Panics
///
/// This function assumes the non-empty input check at the top remains in place;
/// violating that invariant can trigger indexing or `unwrap()` panics in later
/// formatting branches.
#[must_use]
pub fn format_names(
    names: &[crate::reference::FlatName],
    form: &ContributorForm,
    options: &RenderOptions<'_>,
    overrides: &NamesOverrides<'_>,
    hints: &ProcHints,
) -> String {
    format_names_decorated(names, form, options, overrides, hints, &[])
}

fn resolve_contributor_delimiter(
    config: Option<&ContributorConfig>,
    override_delimiter: Option<&citum_schema::template::DelimiterPunctuation>,
    options: &RenderOptions<'_>,
    item_language: Option<&str>,
) -> String {
    let default = citum_schema::template::DelimiterPunctuation::Comma;
    let punctuation = override_delimiter
        .or_else(|| config.and_then(|config| config.delimiter.as_ref()))
        .unwrap_or(&default);
    let (script, realization) = crate::values::punctuation_realization_context(
        item_language,
        options.config.multilingual.as_ref(),
        options.locale.punctuation_realization.as_ref(),
    );
    crate::render::format::realize_punctuation(
        punctuation,
        script,
        realization.as_deref(),
        crate::render::format::PunctuationPosition::Separator,
    )
    .into_owned()
}

/// Format names with per-entry decorations applied before list joining.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "linear context-building pipeline; no clean split point"
)]
pub(super) fn format_names_decorated(
    names: &[crate::reference::FlatName],
    form: &ContributorForm,
    options: &RenderOptions<'_>,
    overrides: &NamesOverrides<'_>,
    hints: &ProcHints,
    decorations: &[NameDecoration],
) -> String {
    if names.is_empty() {
        return String::new();
    }

    let config = options.config.contributors.as_ref();
    let locale = options.locale;

    // Determine shortening options:
    // 1. Use explicit override from template (e.g. bibliography et-al)
    // 2. Else use global config
    let shorten = overrides
        .shorten
        .or_else(|| config.and_then(|c| c.shorten.as_ref()));

    let and_others = shorten.map_or(AndOtherOptions::EtAl, |opts| opts.and_others);

    let (first_names, use_et_al, last_names) = partition_et_al(names, shorten, hints);

    // Build format context once
    let ctx = NameFormatContext {
        display_as_sort: config.and_then(|c| c.display_as_sort),
        name_order: overrides.name_order,
        total_names: names.len(),
        initialize_with: overrides
            .initialize_with
            .or_else(|| config.and_then(|c| c.initialize_with.as_ref())),
        initialize_with_hyphen: config.and_then(|c| c.initialize_with_hyphen),
        name_form: overrides
            .name_form
            .or_else(|| config.and_then(|c| c.name_form)),
        demote_ndp: config.and_then(|c| c.demote_non_dropping_particle.as_ref()),
        sort_separator: overrides
            .sort_separator
            .or_else(|| config.and_then(|c| c.sort_separator.as_ref())),
        component_sort_separator: overrides.sort_separator,
        script_configs: options
            .config
            .multilingual
            .as_ref()
            .map(|multilingual| &multilingual.scripts),
        integral_name_state: hints.integral_name_state,
        org_abbreviation_state: hints.org_abbreviation_state,
        use_integral_short_name: matches!(
            options.mode,
            citum_schema::citation::CitationMode::Integral
        ),
        short_name_display: options
            .config
            .org_abbreviation_memory
            .as_ref()
            .map(|c| c.resolve().short_name_display),
        subsequent_form: options
            .config
            .integral_name_memory
            .as_ref()
            .map(|c| c.resolve().subsequent_form),
        strip_periods: overrides
            .strip_periods
            .or(options.config.strip_periods)
            .unwrap_or(false),
    };

    let delimiter = resolve_contributor_delimiter(
        config,
        overrides.delimiter,
        options,
        overrides.item_language.as_deref(),
    );

    let (formatted_first, formatted_last) = format_selected_names(
        names,
        &first_names,
        &last_names,
        form,
        &ctx,
        hints,
        decorations,
    );

    let and_str = resolve_name_conjunction(
        overrides
            .and
            .or_else(|| config.and_then(|config| config.and.as_ref())),
        locale,
        use_et_al,
        !formatted_last.is_empty(),
    );

    // Check if delimiter should precede last name (Oxford comma)
    let delimiter_precedes_last = config.and_then(|c| c.delimiter_precedes_last.as_ref());
    let two_name_delimiter_policy = config
        .and_then(|c| c.two_name_delimiter_policy)
        .unwrap_or_default();

    let result = if formatted_first.len() == 1 {
        #[allow(clippy::unwrap_used, reason = "length checked")]
        formatted_first.first().unwrap().clone()
    } else {
        join_names_with_conjunction(
            &formatted_first,
            and_str,
            &delimiter,
            &ConjunctionDelimiterContext {
                rule: delimiter_precedes_last,
                policy: two_name_delimiter_policy,
                contributor_count: names.len(),
                display_as_sort: ctx.display_as_sort,
                name_order: ctx.name_order,
                render_context: options.context,
            },
        )
    };

    if !use_et_al {
        return result;
    }

    apply_et_al(
        result,
        &formatted_last,
        EtAlContext {
            and_others,
            delimiter: &delimiter,
            delimiter_precedes: config.and_then(|c| c.delimiter_precedes_et_al.as_ref()),
            first_count: first_names.len(),
        },
        &ctx,
        locale,
    )
}

fn resolve_name_conjunction<'a>(
    and_option: Option<&AndOptions>,
    locale: &'a citum_schema::locale::Locale,
    use_et_al: bool,
    has_retained_last_names: bool,
) -> Option<&'a str> {
    if use_et_al && !has_retained_last_names {
        return None;
    }
    match and_option {
        Some(AndOptions::Text) => Some(locale.and_term(false)),
        Some(AndOptions::Symbol) => Some(locale.and_term(true)),
        Some(AndOptions::None) | None => None,
        _ => None,
    }
}

fn format_selected_names(
    names: &[crate::reference::FlatName],
    first_names: &[&crate::reference::FlatName],
    last_names: &[&crate::reference::FlatName],
    form: &ContributorForm,
    context: &NameFormatContext<'_>,
    hints: &ProcHints,
    decorations: &[NameDecoration],
) -> (Vec<String>, Vec<String>) {
    let formatted_first = first_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let expand =
                hints.expand_given_names && !(hints.expand_given_names_primary_only && index > 0);
            decorate_name(
                format_single_name(name, form, index, context, expand),
                index,
                decorations,
            )
        })
        .collect();
    let formatted_last = last_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let original_index = names.len() - last_names.len() + index;
            let expand = hints.expand_given_names
                && !(hints.expand_given_names_primary_only && original_index > 0);
            decorate_name(
                format_single_name(name, form, original_index, context, expand),
                original_index,
                decorations,
            )
        })
        .collect();
    (formatted_first, formatted_last)
}

fn decorate_name(value: String, index: usize, decorations: &[NameDecoration]) -> String {
    match decorations.get(index) {
        Some(decoration) if !decoration.prefix.is_empty() || !decoration.suffix.is_empty() => {
            format!("{}{value}{}", decoration.prefix, decoration.suffix)
        }
        _ => value,
    }
}

/// Initialize a given name by extracting initials.
///
/// Splits the given name on word separators (space, hyphen, non-breaking space),
/// and converts each part to its first character followed by the initialize suffix.
fn initialize_given_name(
    given: &str,
    initialize_with: Option<&String>,
    initialize_with_hyphen: Option<bool>,
) -> String {
    let init = initialize_with.map_or(". ", std::string::String::as_str);
    let separators = if initialize_with_hyphen == Some(false) {
        vec![' ', '\u{00A0}'] // Non-breaking space too
    } else {
        vec![' ', '-', '\u{00A0}']
    };

    let mut result = String::new();
    let mut current_part = String::new();

    for c in given.chars() {
        if separators.contains(&c) {
            if !current_part.is_empty() {
                if let Some(first) = current_part.chars().next() {
                    result.push(first);
                    result.push_str(init);
                }
                current_part.clear();
            }
            // Preserve only non-whitespace separators (e.g., hyphen for J.-P.).
            // Strip any trailing separator space before the hyphen so we get
            // "J.-P." rather than "J. -P." when init contains a trailing space.
            if !c.is_whitespace() {
                let trimmed_len = result.trim_end().len();
                result.truncate(trimmed_len);
                result.push(c);
            }
        } else {
            current_part.push(c);
        }
    }

    if !current_part.is_empty()
        && let Some(first) = current_part.chars().next()
    {
        result.push(first);
        result.push_str(init);
    }
    result.trim().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NameAssemblyOrder {
    GivenFirst,
    NativeFamilyFirst,
    Inverted,
}

#[derive(Debug, Default)]
struct NameScriptFlags {
    has_han: bool,
    has_hiragana: bool,
    has_katakana: bool,
    has_hangul: bool,
}

impl NameScriptFlags {
    fn record(&mut self, value: &str) {
        for ch in value.chars() {
            match ch.script() {
                Script::Han => self.has_han = true,
                Script::Hiragana => self.has_hiragana = true,
                Script::Katakana => self.has_katakana = true,
                Script::Hangul => self.has_hangul = true,
                _ => {}
            }
        }
    }

    fn cjk_script_count(&self) -> usize {
        usize::from(self.has_han)
            + usize::from(self.has_hiragana)
            + usize::from(self.has_katakana)
            + usize::from(self.has_hangul)
    }

    fn candidate_keys(&self) -> Vec<&'static str> {
        let count = self.cjk_script_count();
        if count == 0 {
            return Vec::new();
        }
        // Mixed kana (Hiragana + Katakana, no Han/Hangul) matches "kana" before "cjk".
        if count > 1
            && !self.has_han
            && !self.has_hangul
            && (self.has_hiragana || self.has_katakana)
        {
            return vec!["kana", "Hrkt", "cjk"];
        }
        if count > 1 {
            return vec!["cjk"];
        }
        if self.has_katakana {
            return vec!["katakana", "Kana", "Hrkt", "kana", "cjk"];
        }
        if self.has_hiragana {
            return vec!["hiragana", "Hira", "Hrkt", "kana", "cjk"];
        }
        if self.has_han {
            return vec!["han", "Hani", "cjk"];
        }
        if self.has_hangul {
            return vec!["hangul", "Hang", "cjk"];
        }
        Vec::new()
    }
}

fn script_config_for_name<'a>(
    name: &crate::reference::FlatName,
    ctx: &'a NameFormatContext<'a>,
) -> Option<&'a citum_schema::options::ScriptConfig> {
    let configs = ctx.script_configs?;
    if configs.is_empty() {
        return None;
    }

    let mut flags = NameScriptFlags::default();
    for part in [
        name.family.as_deref(),
        name.given.as_deref(),
        name.dropping_particle.as_deref(),
        name.non_dropping_particle.as_deref(),
        name.suffix.as_deref(),
        // A transliterated name carries its source script here, so script
        // options (e.g. use-native-ordering) apply to the romanized form too.
        name.original_script.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        flags.record(part);
    }

    flags.candidate_keys().into_iter().find_map(|key| {
        configs.get(key).or_else(|| {
            // Style authors write ISO 15924 keys in their canonical casing
            // ("Han", "Hangul"); candidate keys are lowercase aliases.
            configs.iter().find_map(|(config_key, config)| {
                config_key.eq_ignore_ascii_case(key).then_some(config)
            })
        })
    })
}

/// Assemble a long-form name from its computed parts.
///
/// Inverted order uses "Family, Given"; native family-first uses family-first
/// display order without sort punctuation.
fn assemble_long_name(
    family_part: String,
    given_part: String,
    particle_part: String,
    suffix: &str,
    order: NameAssemblyOrder,
    name_part_delimiter: &str,
    sort_separator: &str,
) -> String {
    match order {
        NameAssemblyOrder::Inverted => assemble_inverted_long_name(
            family_part,
            given_part,
            particle_part,
            suffix,
            sort_separator,
        ),
        NameAssemblyOrder::NativeFamilyFirst => assemble_native_family_first_long_name(
            family_part,
            given_part,
            particle_part,
            suffix,
            name_part_delimiter,
        ),
        NameAssemblyOrder::GivenFirst => assemble_given_first_long_name(
            family_part,
            given_part,
            particle_part,
            suffix,
            name_part_delimiter,
        ),
    }
}

fn assemble_inverted_long_name(
    family_part: String,
    given_part: String,
    particle_part: String,
    suffix: &str,
    sort_separator: &str,
) -> String {
    // citeproc-js places the sort-separator before a generational suffix
    // ("Smith, J., Jr."), not a plain space ("Smith, J. Jr.") — the suffix
    // gets its own separator-joined segment, same as the family/given split.
    let mut given_particle_part = String::new();
    if !given_part.is_empty() {
        given_particle_part.push_str(&given_part);
    }
    if !particle_part.is_empty() {
        if !given_particle_part.is_empty() {
            given_particle_part.push(' ');
        }
        given_particle_part.push_str(&particle_part);
    }

    match (given_particle_part.is_empty(), suffix.is_empty()) {
        (true, true) => family_part,
        (false, true) => format!("{family_part}{sort_separator}{given_particle_part}"),
        (true, false) => format!("{family_part}{sort_separator}{suffix}"),
        (false, false) => {
            format!("{family_part}{sort_separator}{given_particle_part}{sort_separator}{suffix}")
        }
    }
}

fn assemble_native_family_first_long_name(
    family_part: String,
    given_part: String,
    particle_part: String,
    suffix: &str,
    name_part_delimiter: &str,
) -> String {
    let mut parts = Vec::new();
    if !family_part.is_empty() {
        parts.push(family_part);
    }
    if !particle_part.is_empty() {
        parts.push(particle_part);
    }
    if !given_part.is_empty() {
        parts.push(given_part);
    }
    if !suffix.is_empty() {
        parts.push(suffix.to_string());
    }
    parts.join(name_part_delimiter)
}

fn assemble_given_first_long_name(
    family_part: String,
    given_part: String,
    particle_part: String,
    suffix: &str,
    name_part_delimiter: &str,
) -> String {
    let mut parts = Vec::new();
    if !given_part.is_empty() {
        parts.push(given_part);
    }
    if !particle_part.is_empty() {
        parts.push(particle_part);
    }
    if !family_part.is_empty() {
        if let Some(last) = parts.last_mut()
            && last.ends_with('-')
        {
            last.push_str(&family_part);
        } else {
            parts.push(family_part);
        }
    }
    if !suffix.is_empty() {
        parts.push(suffix.to_string());
    }
    parts.join(name_part_delimiter)
}

fn format_literal_name(literal: &str, short: Option<&str>, ctx: &NameFormatContext) -> String {
    if ctx.use_integral_short_name
        && let Some(short) = short
    {
        match ctx.org_abbreviation_state {
            Some(citum_schema::citation::IntegralNameState::First) => {
                return match ctx.short_name_display {
                    Some(citum_schema::options::ShortNameDisplay::ShortThenBracketed) => {
                        format!("{short} [{literal}]")
                    }
                    Some(citum_schema::options::ShortNameDisplay::ShortThenParenthetical) => {
                        format!("{short} ({literal})")
                    }
                    Some(citum_schema::options::ShortNameDisplay::FullThenBracketed) => {
                        format!("{literal} [{short}]")
                    }
                    _ => format!("{literal} ({short})"),
                };
            }
            Some(citum_schema::citation::IntegralNameState::Subsequent) => {
                return short.to_string();
            }
            _ => {}
        }
    }
    literal.to_string()
}

fn is_inverted_name_order(index: usize, ctx: &NameFormatContext) -> bool {
    match ctx.name_order {
        Some(NameOrder::GivenFirst) => false,
        Some(NameOrder::FamilyFirst) => match ctx.display_as_sort {
            Some(DisplayAsSort::First) => index == 0,
            _ => true,
        },
        Some(NameOrder::FamilyFirstOnly) => index == 0,
        Some(NameOrder::FamilyFirstExceptLast) => index != ctx.total_names.saturating_sub(1),
        None => match ctx.display_as_sort {
            Some(DisplayAsSort::All) => true,
            Some(DisplayAsSort::First) => index == 0,
            _ => false,
        },
    }
}

fn name_assembly_order(
    inverted: bool,
    script_config: Option<&citum_schema::options::ScriptConfig>,
    ctx: &NameFormatContext,
) -> NameAssemblyOrder {
    if inverted {
        return NameAssemblyOrder::Inverted;
    }
    let native_family_first =
        ctx.name_order.is_none() && script_config.is_some_and(|config| config.use_native_ordering);
    if native_family_first {
        NameAssemblyOrder::NativeFamilyFirst
    } else {
        NameAssemblyOrder::GivenFirst
    }
}

fn sort_separator_for_name<'a>(
    script_config: Option<&'a citum_schema::options::ScriptConfig>,
    ctx: &'a NameFormatContext<'a>,
) -> &'a str {
    ctx.component_sort_separator
        .map_or_else(
            || {
                script_config
                    .and_then(|config| config.sort_separator.as_deref())
                    .or_else(|| ctx.sort_separator.map(std::string::String::as_str))
            },
            |separator| Some(separator.as_str()),
        )
        .unwrap_or(", ")
}

/// Append the source-script form after a romanized long-form name when a
/// name pattern requests an `original-script` segment (e.g. "Hua Linfu 华林甫").
fn append_original_script(assembled: String, name: &crate::reference::FlatName) -> String {
    match &name.original_script {
        Some(original) if !original.is_empty() => format!("{assembled} {original}"),
        _ => assembled,
    }
}

/// Resolve a name suffix for rendering, optionally stripping its own
/// trailing period. See [`NamesOverrides::strip_periods`].
fn effective_suffix(raw_suffix: &str, strip_periods: bool) -> String {
    if strip_periods && raw_suffix.ends_with('.') {
        crate::values::strip_trailing_periods(raw_suffix)
    } else {
        raw_suffix.to_string()
    }
}

/// Format a single name.
pub(crate) fn format_single_name(
    name: &crate::reference::FlatName,
    form: &ContributorForm,
    index: usize,
    ctx: &NameFormatContext,
    expand_given_names: bool,
) -> String {
    fn join_particle_family(particle: &str, family: &str) -> String {
        if particle.ends_with('-') {
            format!("{particle}{family}")
        } else {
            format!("{particle} {family}")
        }
    }

    // Handle literal names (e.g., corporate authors)
    if let Some(literal) = &name.literal {
        return format_literal_name(literal, name.short_name.as_deref(), ctx);
    }

    let family = name.family.as_deref().unwrap_or("");
    let given = name.given.as_deref().unwrap_or("");
    let dp = name.dropping_particle.as_deref().unwrap_or("");
    let ndp = name.non_dropping_particle.as_deref().unwrap_or("");
    let suffix = &effective_suffix(name.suffix.as_deref().unwrap_or(""), ctx.strip_periods);
    let script_config = script_config_for_name(name, ctx);

    // Determine if we should invert (Family, Given).
    // `display-as-sort: first` in the config limits inversion to the first name
    // even when the template requests `name-order: family-first` for all names.
    let inverted = is_inverted_name_order(index, ctx);
    let assembly_order = name_assembly_order(inverted, script_config, ctx);

    // Determine effective form; integral name-memory overrides template form
    // so first mentions render full name and subsequent mentions render short.
    // Only applies when a memory config is active (subsequent_form is Some).
    let effective_form = if ctx.use_integral_short_name && ctx.subsequent_form.is_some() {
        match ctx.integral_name_state {
            Some(citum_schema::citation::IntegralNameState::First) => &ContributorForm::Long,
            Some(citum_schema::citation::IntegralNameState::Subsequent) => {
                match ctx.subsequent_form {
                    Some(citum_schema::options::SubsequentNameForm::FamilyOnly) => {
                        &ContributorForm::FamilyOnly
                    }
                    _ => &ContributorForm::Short,
                }
            }
            _ => {
                if expand_given_names && matches!(form, ContributorForm::Short) {
                    &ContributorForm::Long
                } else {
                    form
                }
            }
        }
    } else if expand_given_names && matches!(form, ContributorForm::Short) {
        &ContributorForm::Long
    } else {
        form
    };

    match effective_form {
        ContributorForm::FamilyOnly => {
            // FamilyOnly form strictly outputs literally just the family name without non-dropping particles.
            family.to_string()
        }
        ContributorForm::Short => {
            // Short form usually just family name, but includes non-dropping particle
            // e.g. "van Beethoven" (unless demoted? CSL spec says demote only affects sorting/display of full names mostly?)
            // Spec: "demote-non-dropping-particle ... This attribute does not affect ... the short form"
            // So for short form, we keep ndp with family.

            if ndp.is_empty() {
                family.to_string()
            } else {
                format!("{ndp} {family}")
            }
        }
        ContributorForm::Long | ContributorForm::Verb | ContributorForm::VerbShort => {
            // Determine parts based on demotion
            let demote = matches!(
                ctx.demote_ndp,
                Some(DemoteNonDroppingParticle::DisplayAndSort)
            );

            let family_part = if !ndp.is_empty() && !demote {
                join_particle_family(ndp, family)
            } else {
                family.to_string()
            };

            // Determine how to render the given name based on NameForm.
            // initialize-with only controls the separator between initials, not whether
            // to use initials at all. name-form controls the form.
            let effective_name_form = match ctx.name_form {
                Some(f) => f,
                None => NameForm::Full,
            };

            let given_part = match effective_name_form {
                NameForm::FamilyOnly => String::new(),
                NameForm::Initials => {
                    initialize_given_name(given, ctx.initialize_with, ctx.initialize_with_hyphen)
                }
                NameForm::Full => given.to_string(),
            };

            // Construct particle part (dropping + demoted non-dropping)
            let mut particle_part = String::new();
            if !dp.is_empty() {
                particle_part.push_str(dp);
            }
            if demote && !ndp.is_empty() {
                if !particle_part.is_empty() {
                    particle_part.push(' ');
                }
                particle_part.push_str(ndp);
            }

            let name_part_delimiter = script_config
                .and_then(|config| config.delimiter.as_deref())
                .unwrap_or(" ");
            let sep = sort_separator_for_name(script_config, ctx);
            let assembled = assemble_long_name(
                family_part,
                given_part,
                particle_part,
                suffix,
                assembly_order,
                name_part_delimiter,
                sep,
            );
            append_original_script(assembled, name)
        }
    }
}

/// Format contributors in short form for citation grouping.
#[must_use]
pub fn format_contributors_short(
    names: &[crate::reference::FlatName],
    options: &RenderOptions<'_>,
) -> String {
    format_names(
        names,
        &ContributorForm::Short,
        options,
        &NamesOverrides {
            name_order: None,
            sort_separator: None,
            delimiter: None,
            shorten: None,
            and: None,
            initialize_with: None,
            name_form: None,
            strip_periods: None,
            item_language: None,
        },
        &ProcHints::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::RenderContext;
    use citum_schema::options::DelimiterPrecedesLast;

    #[test]
    fn configured_delimiter_rule_is_count_and_inversion_aware() {
        let cases = [
            (DelimiterPrecedesLast::Always, 2, None, true),
            (DelimiterPrecedesLast::Always, 3, None, true),
            (DelimiterPrecedesLast::Contextual, 2, None, false),
            (DelimiterPrecedesLast::Contextual, 3, None, true),
            (DelimiterPrecedesLast::Never, 2, None, false),
            (DelimiterPrecedesLast::Never, 3, None, false),
            (
                DelimiterPrecedesLast::AfterInvertedName,
                2,
                Some(DisplayAsSort::All),
                true,
            ),
            (
                DelimiterPrecedesLast::AfterInvertedName,
                2,
                Some(DisplayAsSort::First),
                true,
            ),
            (
                DelimiterPrecedesLast::AfterInvertedName,
                3,
                Some(DisplayAsSort::First),
                false,
            ),
        ];

        for (rule, displayed_name_count, display_as_sort, expected) in cases {
            assert_eq!(
                delimiter_precedes_conjunction(Some(&rule), displayed_name_count, display_as_sort),
                expected,
                "failed for {rule:?}, displayed_name_count={displayed_name_count}, display_as_sort={display_as_sort:?}"
            );
        }
    }

    #[test]
    fn two_name_policy_uses_contributor_count_not_shortened_display_count() {
        let formatted = ["First".to_string(), "Second".to_string()];
        let delimiter_context = ConjunctionDelimiterContext {
            rule: Some(&DelimiterPrecedesLast::Always),
            policy: TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst,
            contributor_count: 4,
            display_as_sort: None,
            name_order: None,
            render_context: RenderContext::Citation,
        };

        assert_eq!(
            join_names_with_conjunction(&formatted, Some("and"), ", ", &delimiter_context,),
            "First, and Second"
        );
    }

    #[test]
    fn two_name_policy_only_suppresses_declared_contexts() {
        let cases = [
            (
                TwoNameDelimiterPolicy::FollowRule,
                2,
                Some(NameOrder::GivenFirst),
                RenderContext::Citation,
                false,
            ),
            (
                TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst,
                2,
                None,
                RenderContext::Citation,
                true,
            ),
            (
                TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst,
                2,
                Some(NameOrder::GivenFirst),
                RenderContext::Bibliography,
                true,
            ),
            (
                TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst,
                2,
                Some(NameOrder::FamilyFirst),
                RenderContext::Bibliography,
                false,
            ),
            (
                TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst,
                3,
                Some(NameOrder::GivenFirst),
                RenderContext::Citation,
                false,
            ),
        ];

        for (policy, name_count, name_order, context, expected) in cases {
            assert_eq!(
                suppresses_two_name_delimiter(policy, name_count, name_order.as_ref(), context),
                expected,
                "failed for {policy:?}, name_count={name_count}, name_order={name_order:?}, context={context:?}"
            );
        }
    }
}
