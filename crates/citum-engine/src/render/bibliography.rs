/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use std::collections::HashMap;
use std::fmt::Write;

use crate::api::{AnnotationFormat, AnnotationStyle};
use crate::render::component::{
    ProcEntry, ProcTemplateComponent, RenderedComponent, render_component_detailed,
};
use crate::render::format::{OutputFormat, PunctuationPosition, RealizedPunctuation};
use crate::render::plain::PlainText;
use crate::render::punctuation::{
    is_strong_terminal, leading_movable_mark, move_punctuation_into_quote,
    strong_terminal_comma_policy, visible_projection,
};
use crate::render::rich_text::{render_djot_inline, render_org_inline};
use citum_schema::template::DelimiterPunctuation;

/// Realize a bibliography-scoped [`DelimiterPunctuation`] value (the
/// `separator` or `entry_suffix` config field) using the script/locale
/// realization context carried by `first` — the leading component of the
/// entry, the same source `punctuation_in_quote` and `close_quote` are
/// resolved from elsewhere in this module. Shared by
/// `render_entry_body_components_with_format` and
/// `processor/rendering/grouped/sentence_initial.rs`'s bibliography
/// sentence-initial pass, which build the identical separator to decide
/// capitalization boundaries just before calling
/// [`append_rendered_component`].
pub(crate) fn realize_bibliography_punctuation(
    first: Option<&ProcTemplateComponent>,
    punctuation: Option<&DelimiterPunctuation>,
    default: DelimiterPunctuation,
    position: PunctuationPosition,
) -> RealizedPunctuation<'static> {
    let multilingual = first
        .and_then(|c| c.config.as_ref())
        .and_then(|config| config.multilingual.as_ref());
    let (script, realization) = crate::values::punctuation_realization_context(
        first.and_then(|c| c.item_language.as_deref()),
        multilingual,
        first.and_then(|c| c.quote_marks.punctuation_realization.as_ref()),
    );
    let owned;
    let punctuation = if let Some(punctuation) = punctuation {
        punctuation
    } else {
        owned = default;
        &owned
    };
    crate::render::format::realize_punctuation_decomposed(
        punctuation,
        script,
        realization.as_deref(),
        position,
    )
    .into_owned()
}

/// The engine default bibliography separator when no style config is
/// present — a literal, matching [`crate::render::bibliography`]'s historical
/// `unwrap_or(". ")` fallback (not the semantic `Period` mark; see
/// `citum_schema::options::bibliography`'s `default_separator` for why the
/// engine default stays script-invariant).
fn default_separator_punctuation() -> DelimiterPunctuation {
    DelimiterPunctuation::Custom(". ".to_string())
}

/// Returns true if the character is a sentence-ending or clause-ending punctuation mark.
fn is_final_punctuation(c: char) -> bool {
    matches!(c, '.' | ',' | ':' | ';' | '!' | '?' | '…')
}

/// Returns true if the character ends a sentence (period, question mark, exclamation).
fn is_sentence_ending_punctuation(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…')
}

/// Returns the first character of the visible (markup-stripped) text for
/// format `F`, which may be whitespace.
fn first_visible_char<F: OutputFormat<Output = String>>(input: &str) -> Option<char> {
    F::default().visible_text(input).chars().next()
}

/// Returns the last character of the visible (markup-stripped) text for
/// format `F`, which may be whitespace. Unlike a raw `chars().last()`, this
/// is not fooled by trailing markup (an HTML `</span>`, a LaTeX `}`, ...):
/// under those formats the raw last byte is always the markup's own closing
/// character, never whitespace, which would otherwise make every boundary
/// look "non-space" regardless of the actual rendered content.
fn last_visible_char<F: OutputFormat<Output = String>>(input: &str) -> Option<char> {
    F::default().visible_text(input).chars().last()
}

/// Returns the last non-whitespace visible character, used for punctuation deduplication.
fn last_visible_non_space_char<F: OutputFormat<Output = String>>(input: &str) -> Option<char> {
    F::default()
        .visible_text(input)
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
}

/// Returns true if the rendered output ends with sentence-ending punctuation, used to suppress trailing period addition.
fn ends_with_sentence_ending_visible_punctuation<F: OutputFormat<Output = String>>(
    input: &str,
) -> bool {
    let visible = F::default().visible_text(input);
    let mut chars = visible.chars().rev().filter(|ch| !ch.is_whitespace());
    match chars.next() {
        Some(ch) if is_sentence_ending_punctuation(ch) => true,
        Some('"' | '\u{201D}') => chars.next().is_some_and(is_sentence_ending_punctuation),
        _ => false,
    }
}

/// Returns true when the next rendered component should be treated as sentence-initial
/// under the same join semantics used by bibliography rendering.
#[must_use]
pub(crate) fn component_starts_new_sentence<F: OutputFormat<Output = String>>(
    entry_output: &str,
    rendered: &str,
    default_separator: &RealizedPunctuation<'_>,
    punctuation_in_quote: bool,
    close_quote: &str,
) -> bool {
    if entry_output.is_empty() {
        return true;
    }

    let first_char = first_visible_char::<F>(rendered).unwrap_or(' ');
    let starts_with_separator = matches!(first_char, ',' | ';' | ':' | ' ' | '.' | '(');

    if starts_with_separator {
        return false;
    }

    if ends_with_sentence_ending_visible_punctuation::<F>(entry_output) {
        return true;
    }

    let last_char = last_visible_char::<F>(entry_output).unwrap_or(' ');
    let trimmed_last = last_visible_non_space_char::<F>(entry_output).unwrap_or(' ');
    if !last_char.is_whitespace()
        && !first_char.is_whitespace()
        && !is_final_punctuation(trimmed_last)
        && default_separator
            .core()
            .is_some_and(is_sentence_ending_punctuation)
    {
        return true;
    }

    punctuation_in_quote
        && default_separator.core() == Some('.')
        && ends_with_close_quote::<F>(entry_output, close_quote)
}

/// Returns true if `text` ends with the locale-resolved closing quote glyph
/// (or the legacy straight-quote fallback), matching the same acceptance
/// [`move_punctuation_into_quote`] uses when relocating trailing punctuation.
/// Checked against the *visible* text under `F`, so trailing markup (an HTML
/// `</span>`, a LaTeX `}`, ...) does not hide the quote glyph.
fn ends_with_close_quote<F: OutputFormat<Output = String>>(text: &str, close_quote: &str) -> bool {
    let visible = F::default().visible_text(text);
    (!close_quote.is_empty() && visible.ends_with(close_quote))
        || (close_quote != "\"" && visible.ends_with('"'))
}

/// Render processed templates into a final bibliography string using `PlainText` format.
#[must_use]
pub fn refs_to_string(proc_entries: Vec<ProcEntry>) -> String {
    refs_to_string_with_format::<PlainText>(proc_entries, None, None)
}

/// Render one processed bibliography entry body without outer entry/bibliography wrappers.
#[must_use]
pub fn render_entry_body_with_format<F: OutputFormat<Output = String>>(
    entry: &ProcEntry,
) -> String {
    render_entry_body_components_with_format::<F>(&entry.template)
}

/// Append `rendered` to `entry_output`, either via the normal separator logic
/// or, when `suppress` is set, as a raw append with no separator at all.
///
/// `suppress` is set when the *previous* flush was a bare bibliography
/// numeric label (`ProcTemplateComponent::label_only`) — see its docs for
/// why such a label must never engage normal inter-component separator
/// logic with whatever comes next.
#[allow(
    clippy::too_many_arguments,
    reason = "threads shared per-entry punctuation state"
)]
fn append_or_suppress<F: OutputFormat<Output = String>>(
    entry_output: &mut String,
    rendered: &RenderedComponent,
    suppress: bool,
    default_separator: &RealizedPunctuation<'_>,
    punctuation_in_quote: bool,
    strong_terminal_comma_policy: citum_schema::options::StrongTerminalCommaPolicy,
    close_quote: &str,
) {
    if suppress {
        entry_output.push_str(&rendered.text);
    } else {
        append_rendered_component::<F>(
            entry_output,
            rendered,
            default_separator,
            punctuation_in_quote,
            strong_terminal_comma_policy,
            close_quote,
        );
    }
}

/// Re-render `last_component` with its own suffix (and any wrap inner-suffix)
/// stripped, when trailing template components after it were all empty — a
/// component's suffix implies "there is more to come," which stops being
/// true once nothing after it actually rendered.
fn trim_trailing_only_suffix<F: OutputFormat<Output = String>>(
    last_component: &ProcTemplateComponent,
    rendered: RenderedComponent,
    is_truly_last: bool,
) -> RenderedComponent {
    if is_truly_last {
        return rendered;
    }
    let mut trimmed_component = last_component.clone();
    let rendering = trimmed_component.template_component.rendering_mut();
    rendering.suffix = None;
    if let Some(ref mut wrap_config) = rendering.wrap {
        wrap_config.inner_suffix = None;
    }
    trimmed_component.suffix = None;
    render_component_detailed::<F>(&trimmed_component)
}

/// Render processed bibliography components without outer entry/bibliography wrappers.
#[must_use]
pub(crate) fn render_entry_body_components_with_format<F: OutputFormat<Output = String>>(
    proc_template: &[ProcTemplateComponent],
) -> String {
    let mut entry_output = String::new();
    let mut pending_component: Option<(
        usize,
        &crate::render::component::ProcTemplateComponent,
        RenderedComponent,
    )> = None;

    // Check locale option for punctuation placement in quotes.
    let punctuation_in_quote = proc_template
        .first()
        .and_then(|c| c.config.as_ref())
        .is_some_and(|cfg| cfg.punctuation_in_quote);
    let strong_terminal_comma_policy = strong_terminal_comma_policy(
        proc_template
            .first()
            .and_then(|component| component.config.as_deref()),
    );
    let close_quote = proc_template
        .first()
        .map(|c| c.quote_marks.close.as_str())
        .unwrap_or("\u{201D}");

    // Get the bibliography separator from the config, defaulting to ". "
    let first_component = proc_template.first();
    let default_separator = realize_bibliography_punctuation(
        first_component,
        first_component
            .and_then(|c| c.bibliography_config.as_ref())
            .and_then(|bib| bib.separator.as_ref()),
        default_separator_punctuation(),
        PunctuationPosition::Separator,
    );

    // Bibliography numeric labels (the renderer's synthetic
    // `[label, following]` group with `following` empty, e.g. no author)
    // render real, non-empty text but must attach directly to whatever
    // content actually opens the entry — no separator, exactly as if the
    // label were not there. Tracks whether the item just written to
    // `entry_output` was such a label, so the *next* flush skips normal
    // separator logic instead of treating the label as preceding content.
    let mut suppress_next_separator = false;

    for (index, component) in proc_template.iter().enumerate() {
        let rendered = render_component_detailed::<F>(component);
        if rendered.text.is_empty() {
            continue;
        }

        if let Some((_, previous_component, previous)) =
            pending_component.replace((index, component, rendered))
        {
            append_or_suppress::<F>(
                &mut entry_output,
                &previous,
                suppress_next_separator,
                &default_separator,
                punctuation_in_quote,
                strong_terminal_comma_policy,
                close_quote,
            );
            suppress_next_separator = previous_component.label_only;
        }
    }

    if let Some((last_index, last_component, rendered)) = pending_component {
        let final_rendered = trim_trailing_only_suffix::<F>(
            last_component,
            rendered,
            last_index + 1 == proc_template.len(),
        );
        append_or_suppress::<F>(
            &mut entry_output,
            &final_rendered,
            suppress_next_separator,
            &default_separator,
            punctuation_in_quote,
            strong_terminal_comma_policy,
            close_quote,
        );
    }

    let bib_cfg = proc_template
        .first()
        .and_then(|c| c.bibliography_config.as_ref());
    if let Some(entry_suffix) = bib_cfg.and_then(|bib| bib.entry_suffix.as_ref()) {
        let realized_suffix = realize_bibliography_punctuation(
            first_component,
            Some(entry_suffix),
            DelimiterPunctuation::None,
            PunctuationPosition::Suffix,
        );
        // Mirrors the historical `Some(suffix) if !suffix.is_empty()` match
        // guard, now tested against the realized text rather than the enum,
        // so both `Custom("")` and `DelimiterPunctuation::None` land here.
        if !realized_suffix.is_empty() {
            let suffix = realized_suffix.text();
            let suffix_core = realized_suffix.core().unwrap_or('.');
            // The suffix is suppressed after a terminal URL/DOI by default; a
            // style may force it back on per link kind (IEEE: DOI, MLA: URL).
            let suppress = match terminal_link::<F>(&entry_output) {
                TerminalLink::Doi => !bib_cfg.is_some_and(|b| b.entry_suffix_after_doi),
                TerminalLink::Url => !bib_cfg.is_some_and(|b| b.entry_suffix_after_url),
                TerminalLink::None => false,
            };
            let moved_into_quote = !suppress
                && suffix_core == '.'
                && realized_suffix.tail().is_empty()
                && punctuation_in_quote
                && !entry_output.ends_with(suffix_core)
                && move_punctuation_into_quote::<F>(&mut entry_output, '.', close_quote);
            if !moved_into_quote && !suppress && !entry_output.ends_with(suffix_core) {
                entry_output.push_str(suffix);
            }
        }
    }

    cleanup_dangling_punctuation::<F>(&mut entry_output, strong_terminal_comma_policy);
    entry_output
}

/// Append a rendered component to `entry_output`, inserting spacing or the
/// `default_separator` according to bibliography house-style punctuation rules.
///
/// The separator logic inspects the boundary between the accumulated output
/// and the incoming `rendered` string; `punctuation_in_quote` controls whether
/// a period should be pulled inside a preceding closing quotation mark.
pub(crate) fn append_rendered_component<F: OutputFormat<Output = String>>(
    entry_output: &mut String,
    rendered: &RenderedComponent,
    default_separator: &RealizedPunctuation<'_>,
    punctuation_in_quote: bool,
    strong_terminal_comma_policy: citum_schema::options::StrongTerminalCommaPolicy,
    close_quote: &str,
) {
    if !entry_output.is_empty() {
        let last_char = last_visible_char::<F>(entry_output).unwrap_or(' ');
        let first_char = first_visible_char::<F>(&rendered.text).unwrap_or(' ');
        let sep_first_char = default_separator.core().unwrap_or('.');
        let trimmed_last = last_visible_non_space_char::<F>(entry_output).unwrap_or(' ');
        let ends_with_punctuation = is_final_punctuation(trimmed_last);
        // The incoming component already carries its own leading separator (e.g. ", " or "; ").
        let starts_with_separator = matches!(first_char, ',' | ';' | ':' | ' ' | '.' | '(');

        // The incoming component supplies its *own* leading punctuation (e.g. a
        // `prefix: ". Aired "` on the next component) rather than the leading
        // punctuation coming from `default_separator`. This must be checked
        // before `starts_with_separator` below, since such text always matches
        // that condition and would otherwise short-circuit before the quote can
        // be examined. `leading_movable_mark` compares the rendered string's
        // *visible* first character (not its raw one) against the mark, so a
        // semantic wrapper's markup preceding it in the raw string no longer
        // hides it.
        if punctuation_in_quote
            && let Some((mark, rest)) = leading_movable_mark::<F>(&rendered.text)
            && move_punctuation_into_quote::<F>(entry_output, mark, close_quote)
        {
            entry_output.push_str(&rest);
            return;
        }

        if starts_with_separator {
            // The rendered component is self-delimiting — don't add a separator.
            // Exception: an opening parenthesis needs a leading space unless already spaced.
            if first_char == '(' && !last_char.is_whitespace() && last_char != '[' {
                entry_output.push(' ');
            }
        } else if ends_with_punctuation {
            // English-compatible locales retain a comma after a strong terminal mark;
            // locales configured for collapsing retain only the terminal mark.
            if sep_first_char == ',' && is_strong_terminal(trimmed_last) {
                if strong_terminal_comma_policy
                    == citum_schema::options::StrongTerminalCommaPolicy::KeepBoth
                {
                    entry_output.push_str(default_separator.text());
                } else {
                    // `sep_first_char == ','` here is guaranteed by the outer
                    // condition, so the separator's core is exactly the comma
                    // being collapsed away — `tail()` reproduces the historical
                    // `strip_prefix(',')` without a fallible check.
                    entry_output.push_str(default_separator.tail());
                }
            } else if !last_char.is_whitespace() {
                entry_output.push(' ');
            }
        } else if punctuation_in_quote
            && (sep_first_char == '.' || sep_first_char == ',')
            && move_punctuation_into_quote::<F>(entry_output, sep_first_char, close_quote)
        {
            // Punctuation-in-quote: pull the leading period or comma of the
            // separator inside the closing quotation mark, then append the rest
            // of the separator (e.g. the trailing space). Mirrors the citation
            // path in `render/citation.rs::push_delimiter`.
            entry_output.push_str(default_separator.tail());
        } else if !last_char.is_whitespace() && !first_char.is_whitespace() {
            // Both sides are non-space — insert the configured separator between them.
            entry_output.push_str(default_separator.text());
        } else if !last_char.is_whitespace()
            && first_char.is_whitespace()
            && default_separator.core() == Some('.')
            && !ends_with_punctuation
        {
            // The next component leads with whitespace and the separator is period-prefixed:
            // supply the missing period so the gap doesn't swallow the sentence boundary.
            entry_output.push('.');
        }
    }

    let _ = write!(entry_output, "{}", rendered.text);
}

/// Render processed templates into a final bibliography string using a specific format.
#[must_use]
pub fn refs_to_string_with_format<F: OutputFormat<Output = String>>(
    proc_entries: Vec<ProcEntry>,
    annotations: Option<&HashMap<String, String>>,
    annotation_style: Option<&AnnotationStyle>,
) -> String {
    refs_to_string_slice_with_format::<F>(&proc_entries, annotations, annotation_style)
}

/// Render borrowed processed templates into a final bibliography string using a specific format.
#[must_use]
pub fn refs_to_string_slice_with_format<F: OutputFormat<Output = String>>(
    proc_entries: &[ProcEntry],
    annotations: Option<&HashMap<String, String>>,
    annotation_style: Option<&AnnotationStyle>,
) -> String {
    let fmt = F::default();
    let mut rendered_entries = Vec::with_capacity(proc_entries.len());

    for entry in proc_entries {
        let mut entry_output = render_entry_body_with_format::<F>(entry);
        let proc_template = &entry.template;

        // Apply annotation if present
        if let Some(annotations) = annotations
            && let Some(annotation_text) = annotations.get(&entry.id)
        {
            let style = annotation_style.cloned().unwrap_or_default();

            // Render annotation text through markup format if enabled
            let rendered = match style.format {
                AnnotationFormat::Djot => render_djot_inline(annotation_text, &fmt),
                AnnotationFormat::Plain => annotation_text.clone(),
                AnnotationFormat::Org => render_org_inline(annotation_text, &fmt),
            };

            let rendered = rendered.trim();

            if !rendered.is_empty() {
                let annotation_output = fmt.text(rendered);
                entry_output.push_str(&fmt.annotation(annotation_output));
            }
        }

        if fmt.visible_text(&entry_output).trim().is_empty() {
            continue;
        }

        // Resolve entry URL if whole-entry linking is enabled
        let entry_url = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .and_then(|cfg| cfg.links.as_ref())
            .and_then(|links| {
                use citum_schema::options::LinkAnchor;
                if matches!(links.anchor, Some(LinkAnchor::Entry)) {
                    // We need the reference to resolve the URL.
                    // This is a bit tricky as ProcEntry doesn't have the reference.
                    // But we can look it up from the bibliography if we had access to it.
                    // For now, let's see if any component in the template has a URL resolved.
                    proc_template.iter().find_map(|c| c.url.as_deref())
                } else {
                    None
                }
            });

        rendered_entries.push(fmt.entry(&entry.id, entry_output, entry_url, &entry.metadata));
    }

    fmt.finish(fmt.bibliography(rendered_entries))
}

/// Classification of a bibliography entry's terminal token, used to decide
/// whether the `entry_suffix` period applies (per-style URL/DOI policy).
#[derive(PartialEq)]
enum TerminalLink {
    None,
    Url,
    Doi,
}

/// Classify whether an entry ends in a DOI, a plain URL, or neither.
fn terminal_link<F: OutputFormat<Output = String>>(output: &str) -> TerminalLink {
    let visible = F::default().visible_text(output);
    let trimmed = visible.trim_end_matches('.').trim_end();
    let last = trimmed.rsplit_once(' ').map_or(trimmed, |(_, last)| last);
    let is_doi = last.contains("doi.org/")
        || last.starts_with("doi:")
        || (last.starts_with("10.") && last.contains('/'));
    if is_doi {
        TerminalLink::Doi
    } else if last.starts_with("https://") || last.starts_with("http://") {
        TerminalLink::Url
    } else {
        TerminalLink::None
    }
}

/// Dangling-punctuation patterns to collapse, tried in order at each fixed-point step.
const DANGLING_PUNCTUATION_PATTERNS: [(&str, &str); 13] = [
    (", .", "."),
    (", ,", ","),
    (": .", "."),
    ("; .", "."),
    // NOTE: Removed (".,", ".") pattern - it was too aggressive and removed legitimate
    // component suffixes like "S.," from author initials. In Citum, component suffixes are
    // explicit and well-defined, so we don't have the CSL 1.0 dual-punctuation issue.
    //
    // A full-width (".，"/".："/".；") equivalent was tried and reverted for the
    // same reason: it stripped legitimate abbreviation periods ("Inc.，",
    // "D.C.：", "Colo.：") wherever they preceded a CJK delimiter. The actual
    // Jr./Sr. suffix case (`gbt7714.8.3.2:4`) is fixed at the source instead —
    // see `format_single_name`'s suffix handling in
    // `values/contributor/names.rs`, which strips a suffix's own trailing
    // period for styles that don't want name-suffix punctuation.
    (" ,", ","),
    (" ;", ";"),
    (" :", ":"),
    (" .", "."),
    (",  ", ", "),
    (". .", "."),
    (".. ", ". "),
    ("..", "."),
    ("  ", " "), // Double space to single
];

/// Strong-terminal/comma pairs suppressed by locale policy.
const STRONG_TERMINAL_COMMA_PATTERNS: [(&str, &str); 3] = [("!,", "!"), ("?,", "?"), ("…,", "…")];

/// Collapse dangling/duplicated punctuation (`". ."` → `"."`, doubled spaces,
/// etc.) without corrupting interleaved markup, URLs, or attribute values.
///
/// The pattern table above is matched against the format `F`'s *visible*
/// projection of `output` only. A match's replacement is written at the
/// raw position of the match's first visible byte, and the match's other
/// visible bytes are deleted from the raw string — any markup interleaved
/// between them (e.g. a LaTeX `\emph{Title.}` boundary sitting between the
/// separator's leading space and its period) is left untouched. Re-derives
/// the visible projection after every edit (entries are short, so this is
/// cheap) since an edit can expose a new match.
#[allow(
    clippy::string_slice,
    reason = "byte ranges come from OutputFormat::visible_runs, which always yields char boundaries"
)]
fn cleanup_dangling_punctuation<F: OutputFormat<Output = String>>(
    output: &mut String,
    strong_terminal_comma_policy: citum_schema::options::StrongTerminalCommaPolicy,
) {
    loop {
        let (visible, raw_pos) = visible_projection::<F>(output);

        let locale_pattern = if strong_terminal_comma_policy
            == citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal
        {
            STRONG_TERMINAL_COMMA_PATTERNS
                .iter()
                .find_map(|&(pat, repl)| visible.find(pat).map(|idx| (pat, repl, idx)))
        } else {
            None
        };
        let Some((pat, replacement, visible_at)) = locale_pattern.or_else(|| {
            DANGLING_PUNCTUATION_PATTERNS
                .iter()
                .find_map(|&(pat, repl)| visible.find(pat).map(|idx| (pat, repl, idx)))
        }) else {
            break;
        };

        let matched_raw_positions: Vec<usize> = (visible_at..visible_at + pat.len())
            .filter_map(|k| raw_pos.get(k).copied())
            .collect();
        if matched_raw_positions.len() != pat.len() {
            // Projection/pattern mismatch (shouldn't happen for ASCII patterns
            // against char-boundary-safe runs) — bail rather than risk a bad edit.
            break;
        }

        apply_minimal_raw_edit(output, &matched_raw_positions, replacement);
    }
}

/// Rewrite `output` so the raw byte at `positions[0]` is replaced by
/// `replacement` and the raw bytes at `positions[1..]` are deleted, leaving
/// every other byte (including any interleaved markup) untouched.
fn apply_minimal_raw_edit(output: &mut String, positions: &[usize], replacement: &str) {
    let Some((&front, rest)) = positions.split_first() else {
        return;
    };
    let drop: std::collections::HashSet<usize> = rest.iter().copied().collect();

    let mut new_output = String::with_capacity(output.len() + replacement.len());
    for (pos, ch) in output.char_indices() {
        if pos == front {
            new_output.push_str(replacement);
        } else if !drop.contains(&pos) {
            new_output.push(ch);
        }
    }
    *output = new_output;
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::render::component::ProcTemplateComponent;
    use crate::render::djot::Djot;
    use crate::render::html::Html;
    use crate::render::latex::Latex;
    use crate::render::markdown::Markdown;
    use crate::render::typst::Typst;
    use citum_schema::template::{Rendering, TemplateComponent, WrapConfig, WrapPunctuation};
    use rstest::rstest;

    /// Decompose a literal separator for tests exercising
    /// [`append_rendered_component`]/[`component_starts_new_sentence`]
    /// directly, standing in for the realized value those functions receive
    /// from `render_entry_body_components_with_format` in production.
    fn sep(text: &str) -> RealizedPunctuation<'static> {
        RealizedPunctuation::new(text.to_string().into())
    }

    /// A rendered component, standing in for the detailed render
    /// [`append_rendered_component`] receives from
    /// `render_entry_body_components_with_format` in production. A leading
    /// `.`/`,` in `text` is detected the same way production does — from the
    /// rendered text's own visible content — so no separate "with mark"
    /// variant is needed.
    fn rendered(text: &str) -> RenderedComponent {
        RenderedComponent {
            text: text.to_string(),
        }
    }

    #[test]
    fn terminal_link_classifies_url_doi_and_plain_text() {
        // given a DOI in url form, doi: form, or bare 10.x form → Doi
        assert!(
            terminal_link::<PlainText>("Author. Title. https://doi.org/10.1/x")
                == TerminalLink::Doi
        );
        assert!(terminal_link::<PlainText>("Author. Title. doi:10.1038/abc") == TerminalLink::Doi);
        assert!(terminal_link::<PlainText>("Author. Title. doi: 10.1038/abc") == TerminalLink::Doi);
        // given a plain URL → Url
        assert!(
            terminal_link::<PlainText>("Author. Title. https://example.com/page")
                == TerminalLink::Url
        );
        // given prose with no terminal link → None
        assert!(terminal_link::<PlainText>("Author. Title. Publisher, 2020") == TerminalLink::None);
        // a trailing period is ignored when classifying
        assert!(
            terminal_link::<PlainText>("Author. https://example.com/page.") == TerminalLink::Url
        );
    }

    #[test]
    fn test_component_starts_new_sentence_at_entry_start() {
        assert!(component_starts_new_sentence::<PlainText>(
            "",
            "Edited by Grimm, Jacob",
            &sep(". "),
            false,
            "\u{201D}"
        ));
    }

    #[test]
    fn test_component_starts_new_sentence_after_period() {
        assert!(component_starts_new_sentence::<PlainText>(
            "Collected Essays.",
            "edited by Grimm, Jacob",
            &sep(". "),
            false,
            "\u{201D}"
        ));
    }

    #[test]
    fn test_component_does_not_start_new_sentence_after_colon() {
        assert!(!component_starts_new_sentence::<PlainText>(
            "Collected Essays:",
            "edited by Grimm, Jacob",
            &sep(". "),
            false,
            "\u{201D}"
        ));
    }

    #[test]
    fn test_bibliography_separator_suppression() {
        use citum_schema::options::{BibliographyConfig, Config};

        let config = Config::default();
        let bibliography_config = BibliographyConfig {
            separator: Some(". ".into()),
            entry_suffix: Some(String::new().into()),
            ..Default::default()
        };

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Variable(
                citum_schema::template::TemplateVariable {
                    variable: citum_schema::template::SimpleVariable::Publisher,
                    rendering: Rendering::default(),
                    ..Default::default()
                },
            ),
            template_index: None,
            value: "Publisher1".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let c2 = ProcTemplateComponent {
            template_component: TemplateComponent::Variable(
                citum_schema::template::TemplateVariable {
                    variable: citum_schema::template::SimpleVariable::PublisherPlace,
                    rendering: Rendering {
                        prefix: Some(". ".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            template_index: None,
            value: "Place".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let entries = vec![ProcEntry {
            id: "id1".to_string(),
            template: vec![c1, c2],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }];
        let result = refs_to_string(entries);
        assert_eq!(result, "Publisher1. Place");
    }

    #[rstest]
    #[case::label_only_component_suppresses_the_separator(true, "[15]Title Text")]
    #[case::ordinary_first_component_keeps_the_separator(false, "[15]. Title Text")]
    fn given_a_leading_component_when_label_only_flag_varies_then_separator_follows_it(
        #[case] label_only: bool,
        #[case] expected: &str,
    ) {
        // A bibliography numeric-label component (the renderer's synthetic
        // synthetic `[label, following]` group with `following` empty --
        // e.g. no author) renders real, non-empty text ("[15]") but must
        // never engage normal separator logic with whatever comes next.
        // Contrasted against an ordinary component of otherwise identical
        // shape, which *does* get the normal separator.
        use citum_schema::options::{BibliographyConfig, Config};

        let config = Config::default();
        let bibliography_config = BibliographyConfig {
            separator: Some(". ".into()),
            entry_suffix: Some(String::new().into()),
            ..Default::default()
        };

        let label = ProcTemplateComponent {
            template_component: TemplateComponent::Number(citum_schema::template::TemplateNumber {
                number: citum_schema::template::NumberVariable::CitationNumber,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            template_index: None,
            value: "[15]".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: true,
            label_only,
        };

        let content = ProcTemplateComponent {
            template_component: TemplateComponent::Title(citum_schema::template::TemplateTitle {
                title: citum_schema::template::TitleType::Primary,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            template_index: None,
            value: "Title Text".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let entries = vec![ProcEntry {
            id: "id1".to_string(),
            template: vec![label, content],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }];
        let result = refs_to_string(entries);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_suppression_after_parenthesis() {
        use citum_schema::options::{BibliographyConfig, Config};

        let config = Config::default();
        let bibliography_config = BibliographyConfig {
            separator: Some(", ".into()),
            entry_suffix: Some(String::new().into()),
            ..Default::default()
        };

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Contributor(
                citum_schema::template::TemplateContributor {
                    contributor: citum_schema::template::ContributorRole::Editor.into(),
                    rendering: Rendering {
                        wrap: Some(WrapConfig {
                            punctuation: WrapPunctuation::Parentheses,
                            inner_prefix: None,
                            inner_suffix: None,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            template_index: None,
            value: "Eds.".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let c2 = ProcTemplateComponent {
            template_component: TemplateComponent::Title(citum_schema::template::TemplateTitle {
                title: citum_schema::template::TitleType::Primary,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            template_index: None,
            value: "Title".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let entries = vec![ProcEntry {
            id: "id1".to_string(),
            template: vec![c1, c2],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }];
        let result = refs_to_string(entries);
        assert_eq!(result, "(Eds.), Title");
    }

    #[test]
    fn test_punctuation_in_quote_pulls_comma_inside_closing_quote() {
        // given a quoted article title followed by a comma-delimited separator
        // (IEEE house style: separator ", ", punctuation-in-quote enabled)
        let mut entry_output = String::from("\u{201C}Deep Learning\u{201D}");
        // when the next component (the journal title) is appended
        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered("Nature"),
            &sep(", "),
            true,
            Default::default(),
            "\u{201D}",
        );
        // then the comma is pulled inside the closing quotation mark
        assert_eq!(entry_output, "\u{201C}Deep Learning,\u{201D} Nature");
    }

    #[test]
    fn test_punctuation_in_quote_pulls_period_inside_closing_quote() {
        // given a quoted title followed by a period-delimited separator
        let mut entry_output = String::from("\u{201C}Deep Learning\u{201D}");
        // when the next component is appended
        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered("Nature"),
            &sep(". "),
            true,
            Default::default(),
            "\u{201D}",
        );
        // then the period is pulled inside the closing quotation mark (unchanged behaviour)
        assert_eq!(entry_output, "\u{201C}Deep Learning.\u{201D} Nature");
    }

    #[test]
    fn test_punctuation_in_quote_disabled_leaves_comma_outside_quote() {
        // given punctuation-in-quote disabled
        let mut entry_output = String::from("\u{201C}Deep Learning\u{201D}");
        // when the next component is appended with a comma separator
        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered("Nature"),
            &sep(", "),
            false,
            Default::default(),
            "\u{201D}",
        );
        // then the comma stays outside the closing quotation mark
        assert_eq!(entry_output, "\u{201C}Deep Learning\u{201D}, Nature");
    }

    #[rstest]
    #[case('.', "period")]
    #[case(',', "comma")]
    fn append_rendered_component_moves_next_component_own_leading_mark_inside_closing_quote(
        #[case] mark: char,
        #[case] label: &str,
    ) {
        // The next component's own rendering leads with the mark (e.g. the
        // Chicago `broadcast` variant's `prefix: ". Aired "` on the following
        // date component) — the leading punctuation comes from `rendered`,
        // not from `default_separator`.
        let mut entry_output = "\u{201C}The Universe in a Nutshell\u{201D}".to_string();

        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered(&format!("{mark} Aired September 28")),
            &sep(", "),
            true,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(
            entry_output,
            format!("\u{201C}The Universe in a Nutshell{mark}\u{201D} Aired September 28"),
            "{label}-led component text should move inside the quote"
        );
    }

    #[test]
    fn append_rendered_component_moves_next_component_own_leading_mark_behind_a_semantic_span() {
        // Under `Html` (with semantics on, the default), `apply_component_semantics`
        // wraps the *already-affixed* output in a `<span class="...">` — so the
        // rendered component's raw first character is markup, never the mark
        // itself. `leading_movable_mark` compares the *visible* first character
        // (not the raw one) and must still find and relocate it.
        use crate::render::html::Html;

        let mut entry_output =
            r#"<span class="citum-title">“The Universe in a Nutshell”</span>"#.to_string();

        append_rendered_component::<Html>(
            &mut entry_output,
            &rendered(r#"<span class="citum-issued">. Aired September 28</span>"#),
            &sep(", "),
            true,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(
            entry_output,
            "<span class=\"citum-title\">\u{201C}The Universe in a Nutshell.\u{201D}</span>\
<span class=\"citum-issued\"> Aired September 28</span>"
        );
    }

    #[test]
    fn append_rendered_component_reads_the_visible_last_char_not_the_markups_raw_last_byte() {
        // Regression (Copilot review, PR #1135): under `Html`, a raw
        // `entry_output.chars().last()` is always the closing tag's `>`,
        // never whitespace -- even when the *visible* content genuinely
        // ends in a space (e.g. a contributor component's own trailing
        // space before its semantic span closes). Reading the raw last
        // byte would wrongly treat this boundary as "both sides non-space"
        // and insert a second separator on top of the trailing space
        // already there.
        use crate::render::html::Html;

        let mut entry_output = r#"<span class="citum-author">Smith </span>"#.to_string();

        append_rendered_component::<Html>(
            &mut entry_output,
            &rendered("2020"),
            &sep(". "),
            false,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(
            entry_output,
            r#"<span class="citum-author">Smith </span>2020"#
        );
    }

    #[test]
    fn append_rendered_component_leaves_next_component_own_leading_mark_outside_quote_when_disabled()
     {
        let mut entry_output = "\u{201C}The Universe in a Nutshell\u{201D}".to_string();

        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered(". Aired September 28"),
            &sep(", "),
            false,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(
            entry_output,
            "\u{201C}The Universe in a Nutshell\u{201D}. Aired September 28"
        );
    }

    #[test]
    fn append_rendered_component_moves_separator_led_mark_behind_trailing_latex_markup() {
        // The *separator*-driven move (not the component's own leading mark):
        // `entry_output` already ends with an `\emph{...}` wrapping the closing
        // quote, so the quote glyph is not the raw string's last character.
        // `move_punctuation_into_quote` must still find it via the visible
        // projection under `Latex`.
        use crate::render::latex::Latex;

        let mut entry_output = r"\emph{“Deep Learning”}".to_string();

        append_rendered_component::<Latex>(
            &mut entry_output,
            &rendered("Nature"),
            &sep(", "),
            true,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(entry_output, "\\emph{“Deep Learning,”} Nature");
    }

    #[test]
    fn append_rendered_component_moves_mark_inside_a_locale_specific_close_quote() {
        // A French-style guillemet close quote rather than the en-US curly
        // quote — the hardcoded '"'/'\u{201D}' match this replaces would never
        // fire for this glyph.
        let mut entry_output = "«Titre»".to_string();

        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered("Suite"),
            &sep(", "),
            true,
            Default::default(),
            "»",
        );

        assert_eq!(entry_output, "«Titre,» Suite");
    }

    #[test]
    fn strong_terminal_comma_policy_controls_bibliography_separator() {
        for terminal in ['!', '?', '…'] {
            let mut keep_both = format!("Title{terminal}");
            append_rendered_component::<PlainText>(
                &mut keep_both,
                &rendered("Next"),
                &sep(", "),
                false,
                citum_schema::options::StrongTerminalCommaPolicy::KeepBoth,
                "\u{201D}",
            );
            assert_eq!(keep_both, format!("Title{terminal}, Next"));

            let mut keep_terminal = format!("Title{terminal}");
            append_rendered_component::<PlainText>(
                &mut keep_terminal,
                &rendered("Next"),
                &sep(", "),
                false,
                citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal,
                "\u{201D}",
            );
            assert_eq!(keep_terminal, format!("Title{terminal} Next"));
        }
    }

    #[test]
    fn keep_terminal_policy_preserves_bibliography_separator_tail() {
        let mut entry_output = "Title?".to_string();
        append_rendered_component::<PlainText>(
            &mut entry_output,
            &rendered("Next"),
            &sep(",\u{00A0}"),
            false,
            citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal,
            "\u{201D}",
        );

        assert_eq!(entry_output, "Title?\u{00A0}Next");
    }

    #[test]
    fn test_html_bibliography_structure() {
        use crate::render::html::Html;
        use citum_schema::template::TemplateTerm;

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Term(TemplateTerm::default()),
            value: "Reference Content".to_string(),
            ..Default::default()
        };

        let entries = vec![ProcEntry {
            id: "ref-1".to_string(),
            template: vec![c1],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }];

        let result = refs_to_string_with_format::<Html>(entries, None, None);
        assert_eq!(
            result,
            "<div class=\"citum-bibliography\">\n<div class=\"citum-entry\" id=\"ref-ref-1\">Reference Content</div>\n</div>"
        );
    }

    #[test]
    fn test_component_suffix_preserved_elsevier_harvard() {
        use citum_schema::options::{BibliographyConfig, Config};

        // Elsevier Harvard: author component has suffix `, ` and date has suffix `.`
        // Expected: "Hawking, S., 1988." (comma from author suffix preserved)
        let config = Config::default();
        let bibliography_config = BibliographyConfig {
            separator: Some(". ".into()),
            entry_suffix: Some(".".into()),
            ..Default::default()
        };

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Contributor(
                citum_schema::template::TemplateContributor {
                    contributor: citum_schema::template::ContributorRole::Author.into(),
                    rendering: Rendering {
                        suffix: Some(", ".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            template_index: None,
            value: "Hawking, S.".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let c2 = ProcTemplateComponent {
            template_component: TemplateComponent::Date(citum_schema::template::TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                rendering: Rendering {
                    suffix: Some(".".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            template_index: None,
            value: "1988".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let entries = vec![ProcEntry {
            id: "hawking1988".to_string(),
            template: vec![c1, c2],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }];
        let result = refs_to_string(entries);
        // The comma from author's suffix should be preserved
        assert_eq!(result, "Hawking, S., 1988.");
    }

    #[test]
    fn test_terminal_component_suffix_suppressed_when_following_component_is_empty() {
        use citum_schema::options::{BibliographyConfig, Config};

        let config = Config::default();
        let bibliography_config = BibliographyConfig {
            separator: Some(". ".into()),
            entry_suffix: Some(String::new().into()),
            ..Default::default()
        };

        let date = ProcTemplateComponent {
            template_component: TemplateComponent::Date(citum_schema::template::TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                rendering: Rendering {
                    suffix: Some(", ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            template_index: None,
            value: "2024".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let pages = ProcTemplateComponent {
            template_component: TemplateComponent::Number(citum_schema::template::TemplateNumber {
                number: citum_schema::template::NumberVariable::Pages,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            template_index: None,
            value: String::new(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let result = refs_to_string(vec![ProcEntry {
            id: "book-without-pages".to_string(),
            template: vec![date, pages],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }]);

        assert_eq!(result, "2024");
    }

    #[allow(
        clippy::too_many_lines,
        reason = "rendering fixture exercises a full punctuation case"
    )]
    #[test]
    fn test_html_separator_logic_uses_visible_punctuation() {
        use crate::render::html::Html;
        use citum_schema::options::{BibliographyConfig, Config};
        use citum_schema::template::{
            NumberVariable, SimpleVariable, TemplateNumber, TemplateVariable,
        };

        let config = Config {
            ..Default::default()
        };
        let bibliography_config = BibliographyConfig {
            separator: Some(". ".into()),
            entry_suffix: Some(String::new().into()),
            ..Default::default()
        };

        let volume_issue = ProcTemplateComponent {
            template_component: TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Volume,
                rendering: Rendering {
                    emph: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }),
            template_index: None,
            value: "322(10)".to_string(),
            prefix: None,
            suffix: None,
            ref_type: Some("article-journal".to_string()),
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let pages = ProcTemplateComponent {
            template_component: TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Pages,
                rendering: Rendering {
                    prefix: Some(", ".into()),
                    suffix: Some(".".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            template_index: None,
            value: "891–921".to_string(),
            prefix: None,
            suffix: None,
            ref_type: Some("article-journal".to_string()),
            config: Some(config.clone().into()),
            bibliography_config: Some(bibliography_config.clone().into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let doi = ProcTemplateComponent {
            template_component: TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Doi,
                rendering: Rendering {
                    prefix: Some("https://doi.org/".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            template_index: None,
            value: "10.1002/andp.19053221004".to_string(),
            prefix: None,
            suffix: None,
            ref_type: Some("article-journal".to_string()),
            config: Some(config.into()),
            bibliography_config: Some(bibliography_config.into()),
            url: None,
            item_language: None,
            quote_marks: Default::default(),
            sentence_initial: false,
            pre_formatted: false,
            label_only: false,
        };

        let result = refs_to_string_with_format::<Html>(
            vec![ProcEntry {
                id: "einstein1905".to_string(),
                template: vec![volume_issue, pages, doi],
                metadata: crate::render::format::ProcEntryMetadata::default(),
            }],
            None,
            None,
        );

        assert!(
            !result.contains("322(10)</i></span>. <span class=\"citum-pages\">, 891–921."),
            "separator should not inject a period before pages: {result}"
        );
        assert!(
            !result.contains("891–921.</span>. <span class=\"citum-doi\">"),
            "separator should not inject a period before DOI: {result}"
        );
        assert!(
            result.contains(
                "<span class=\"citum-pages\">, 891–921.</span><span class=\"citum-doi\">"
            ) || result.contains(
                "<span class=\"citum-pages\">, 891–921.</span> <span class=\"citum-doi\">"
            ),
            "HTML output should preserve pages punctuation without duplicate separators: {result}"
        );
    }

    fn make_entry(id: &str, value: &str) -> ProcEntry {
        ProcEntry {
            id: id.to_string(),
            template: vec![ProcTemplateComponent {
                template_component: TemplateComponent::Variable(
                    citum_schema::template::TemplateVariable {
                        variable: citum_schema::template::SimpleVariable::Publisher,
                        rendering: Rendering::default(),
                        ..Default::default()
                    },
                ),
                template_index: None,
                value: value.to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: None,
                url: None,
                bibliography_config: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                label_only: false,
            }],
            metadata: crate::render::format::ProcEntryMetadata::default(),
        }
    }

    #[test]
    fn test_annotation_appended_after_entry() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "ref1".to_string(),
            "A useful overview of the topic.".to_string(),
        );

        let style = AnnotationStyle::default();

        let result = refs_to_string_with_format::<PlainText>(
            vec![make_entry("ref1", "Some Publisher")],
            Some(&annotations),
            Some(&style),
        );

        assert!(
            result.contains("Some Publisher"),
            "entry text should appear: {result}"
        );
        assert!(
            result.contains("A useful overview of the topic."),
            "annotation should appear: {result}"
        );
        // Blank line separator: entry text followed by \n\n
        assert!(
            result.contains("\n\nA useful overview"),
            "annotation should be separated by blank line: {result}"
        );
    }

    #[test]
    fn test_no_annotation_when_id_absent() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "other-ref".to_string(),
            "Annotation for someone else.".to_string(),
        );

        let style = AnnotationStyle::default();

        let result = refs_to_string_with_format::<PlainText>(
            vec![make_entry("ref1", "Some Publisher")],
            Some(&annotations),
            Some(&style),
        );

        assert!(
            !result.contains("Annotation for someone else."),
            "annotation for a different ref should not appear: {result}"
        );
    }

    #[test]
    fn test_no_annotations_when_none_supplied() {
        let result = refs_to_string_with_format::<PlainText>(
            vec![make_entry("ref1", "Some Publisher")],
            None,
            None,
        );

        assert!(
            result.contains("Some Publisher"),
            "entry should render normally: {result}"
        );
        // No extra blank lines beyond entry separator
        let blank_line_count = result.matches("\n\n").count();
        assert!(
            blank_line_count <= 1,
            "should not have spurious blank lines: {result}"
        );
    }

    // ── Cross-backend punctuation-boundary regressions (bean csl26-ztxq) ────
    // DESIGN_PRINCIPLES §7: backends may differ only in markup, not in
    // citation logic. See docs/architecture/audits/2026-07-04_CITUM_ENGINE_REVIEW_PART2.md
    // finding 13.

    #[test]
    fn visible_text_is_identical_across_backends_for_the_same_logical_content() {
        // Each markup backend's emph() wraps "Title." in its own markup; the
        // visible text must be identical across all of them. `PlainText` is
        // excluded: it has no markup lexer to strip — its `emph()` output
        // (`_Title._`) *is* the literal plain-text rendering, not markup
        // hiding "Title.", so the parity claim doesn't apply to it.
        assert_eq!(
            Html.visible_text(&Html.emph("Title.".to_string())),
            "Title."
        );
        assert_eq!(
            Latex.visible_text(&Latex.emph("Title.".to_string())),
            "Title."
        );
        assert_eq!(
            Typst.visible_text(&Typst.emph("Title.".to_string())),
            "Title."
        );
        assert_eq!(
            Markdown.visible_text(&Markdown.emph("Title.".to_string())),
            "Title."
        );
        assert_eq!(
            Djot.visible_text(&Djot.emph("Title.".to_string())),
            "Title."
        );
    }

    #[test]
    fn append_rendered_component_does_not_double_punctuate_an_emphasized_latex_title() {
        // Regression for finding 13: `\emph{Title.}` ends in a raw `}`, but its
        // *visible* last char is the period. append_rendered_component must see
        // that (via first_visible_char/last_visible_non_space_char) and not
        // additionally insert the ". " separator's period, which used to
        // produce "\emph{Title.}. Next" (rendering as "Title.. Next").
        let mut entry_output = Latex.emph("Title.".to_string());
        append_rendered_component::<Latex>(
            &mut entry_output,
            &rendered("Next"),
            &sep(". "),
            false,
            Default::default(),
            "\u{201D}",
        );

        assert_eq!(Latex.visible_text(&entry_output), "Title. Next");
        assert!(
            !Latex.visible_text(&entry_output).contains(".."),
            "no doubled period, got: {entry_output}"
        );
    }

    #[test]
    fn cleanup_dangling_punctuation_collapses_across_a_latex_markup_boundary() {
        // A literal doubled period straddling an `\emph{...}` boundary (e.g.
        // from an explicitly authored suffix) must still collapse to one
        // period, and the emph markup must survive intact.
        let mut output = r"\emph{Title.}. Next".to_string();
        cleanup_dangling_punctuation::<Latex>(&mut output, Default::default());

        assert_eq!(Latex.visible_text(&output), "Title. Next");
        assert!(
            output.contains(r"\emph{Title"),
            "emph markup must survive: {output}"
        );
    }

    #[test]
    fn cleanup_dangling_punctuation_never_touches_the_href_target() {
        // The URL inside \href{...} is not visible text; a dangling-punctuation
        // pattern inside it must survive even though the identical pattern
        // outside the link gets collapsed.
        let mut output = r"\href{https://example.com/a, .b}{Link}, .".to_string();
        cleanup_dangling_punctuation::<Latex>(&mut output, Default::default());

        assert!(
            output.contains("https://example.com/a, .b"),
            "href target must be untouched: {output}"
        );
        assert_eq!(Latex.visible_text(&output), "Link.");
    }

    #[test]
    fn cleanup_dangling_punctuation_collapses_across_a_typst_markup_boundary() {
        let mut output = "#emph[Title.]. Next".to_string();
        cleanup_dangling_punctuation::<Typst>(&mut output, Default::default());

        assert_eq!(Typst.visible_text(&output), "Title. Next");
        assert!(
            output.contains("#emph[Title"),
            "emph markup must survive: {output}"
        );
    }

    #[test]
    fn cleanup_dangling_punctuation_applies_locale_policy_across_markup() {
        let mut latex = r"\emph{Title!}, Next".to_string();
        cleanup_dangling_punctuation::<Latex>(
            &mut latex,
            citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal,
        );
        assert_eq!(Latex.visible_text(&latex), "Title! Next");
        assert!(latex.contains(r"\emph{Title!}"));

        let mut typst = "#emph[Title…], Next".to_string();
        cleanup_dangling_punctuation::<Typst>(
            &mut typst,
            citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal,
        );
        assert_eq!(Typst.visible_text(&typst), "Title… Next");
        assert!(typst.contains("#emph[Title…]"));
    }
}
