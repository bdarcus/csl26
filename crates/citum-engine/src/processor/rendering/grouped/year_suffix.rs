/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering for `YearSuffixCollapse::Merged` / `::Ranged`: merging and
//! ranging same-year disambiguation suffixes inside a same-author collapsed
//! group (`Smith (2020a, b)` / `Smith (2020a–c)`). See
//! `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13.

use super::super::{GroupRenderParams, Renderer};
use crate::render::format::{PunctuationPosition, realize_punctuation};
use crate::values::ScriptClass;
use citum_schema::options::PunctuationRealization;
use citum_schema::template::DelimiterPunctuation;
use citum_schema::{SameAuthorCollapse, YearSuffixCollapse};

impl Renderer<'_> {
    /// The 1-based disambiguation-group index for `item_id`, when that item
    /// is currently rendering a year-suffix disambiguator.
    ///
    /// Mirrors the exact predicate
    /// `crate::values::date::compute_disamb_suffix_label` uses to decide
    /// whether to append a suffix letter at all, so a merge run only ever
    /// forms over items that actually rendered one.
    pub(super) fn year_suffix_group_index(&self, item_id: &str) -> Option<u32> {
        let hints = self.hints.get(item_id)?;
        if !hints.disamb_condition {
            return None;
        }
        let use_suffix = self
            .config
            .effective_processing()
            .config()
            .disambiguate
            .as_ref()
            .is_some_and(|disambiguate| disambiguate.year_suffix);
        use_suffix.then_some(hints.group_index as u32)
    }

    /// Resolve `SameAuthorCollapse::delimiter`, realized for script/locale.
    pub(super) fn resolve_same_author_delimiter(
        &self,
        same_author: Option<&SameAuthorCollapse>,
        script: ScriptClass,
        realization: Option<&PunctuationRealization>,
    ) -> Option<String> {
        let punctuation = same_author?.delimiter.as_ref()?;
        Some(
            realize_punctuation(
                punctuation,
                script,
                realization,
                PunctuationPosition::Separator,
            )
            .into_owned(),
        )
    }

    /// Resolve the delimiter joining a merged/ranged suffix run, per
    /// `SAME_AUTHOR_COLLAPSE.md` §13: `delimiter`, then
    /// `year_suffix_delimiter`, then the realized `multi_cite_delimiter`
    /// (default `"; "`).
    pub(super) fn resolve_year_suffix_join_delimiter(
        &self,
        same_author: Option<&SameAuthorCollapse>,
        multi_cite_delimiter: Option<&DelimiterPunctuation>,
        script: ScriptClass,
        realization: Option<&PunctuationRealization>,
    ) -> String {
        if let Some(delimiter) =
            self.resolve_same_author_delimiter(same_author, script, realization)
        {
            return delimiter;
        }
        if let Some(punctuation) = same_author.and_then(|cfg| cfg.year_suffix_delimiter.as_ref()) {
            return realize_punctuation(
                punctuation,
                script,
                realization,
                PunctuationPosition::Separator,
            )
            .into_owned();
        }
        multi_cite_delimiter
            .map(|punctuation| {
                realize_punctuation(
                    punctuation,
                    script,
                    realization,
                    PunctuationPosition::Separator,
                )
                .into_owned()
            })
            .unwrap_or_else(|| "; ".to_string())
    }

    /// Merge adjacent same-year suffix tokens inside `item_parts` per
    /// `same_author.year_suffix`'s degree (`Merged`/`Ranged`).
    ///
    /// No-op — returns `item_parts` unchanged — when `year_suffix` is
    /// `Separate`, when the group carries a locator (`group_has_locator`;
    /// the existing locator-escalation mechanism owns that interaction, see
    /// §13), or when no run in `suffix_indices` is mergeable.
    ///
    /// `suffix_indices` must be the same length as `item_parts` and
    /// index-aligned — see `render_group_item_parts_with_format`.
    pub(super) fn merge_year_suffix_parts(
        &self,
        item_parts: Vec<String>,
        suffix_indices: &[Option<u32>],
        same_author: &SameAuthorCollapse,
        group_has_locator: bool,
        suffix_delimiter: &str,
    ) -> Vec<String> {
        if group_has_locator || matches!(same_author.year_suffix, YearSuffixCollapse::Separate) {
            return item_parts;
        }
        let ranged = matches!(same_author.year_suffix, YearSuffixCollapse::Ranged);
        merge_suffix_runs(
            item_parts,
            suffix_indices,
            ranged,
            suffix_delimiter,
            self.identifier_range_delimiter(),
        )
    }

    /// Resolve `params.spec.collapse` as `SameAuthorCollapse`, merge any
    /// year-suffix runs in `item_parts`, and resolve the same-author
    /// collapse delimiter override -- the three steps
    /// `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13 adds ahead of the existing
    /// join-delimiter resolution in
    /// `render_fallback_grouped_citation_with_format`. Returns the
    /// (possibly merged) item parts alongside the resolved delimiter
    /// override, `None` when the style doesn't declare `delimiter`.
    pub(super) fn apply_same_author_year_suffix(
        &self,
        item_parts: Vec<String>,
        suffix_indices: &[Option<u32>],
        group_has_locator: bool,
        params: &GroupRenderParams<'_>,
        script: ScriptClass,
        realization: Option<&PunctuationRealization>,
    ) -> (Vec<String>, Option<String>) {
        let same_author_collapse = match &params.spec.collapse {
            Some(citum_schema::CitationCollapse::SameAuthor(config)) => Some(config),
            _ => None,
        };
        let item_parts = if let Some(same_author) = same_author_collapse {
            let suffix_join_delimiter = self.resolve_year_suffix_join_delimiter(
                Some(same_author),
                params.spec.multi_cite_delimiter.as_ref(),
                script,
                realization,
            );
            self.merge_year_suffix_parts(
                item_parts,
                suffix_indices,
                same_author,
                group_has_locator,
                &suffix_join_delimiter,
            )
        } else {
            item_parts
        };
        let same_author_delimiter =
            self.resolve_same_author_delimiter(same_author_collapse, script, realization);
        (item_parts, same_author_delimiter)
    }
}

/// A maximal run of adjacent same-year suffix tokens sharing one `base`.
struct MergeableRun {
    /// The rendered text with each item's own suffix letter stripped.
    base: String,
    /// The stripped suffix letters, in order (`["a", "b", "c"]`).
    letters: Vec<String>,
    /// Exclusive end index of the run in `item_parts`.
    end: usize,
}

/// Merge every mergeable run in `item_parts`, per `ranged`'s degree.
///
/// `Ranged` only collapses a run to a range at 3+ consecutive suffixes,
/// matching citeproc-js's arity (`CSL.NumericBlob.prototype.checkNext`);
/// shorter runs render identically to `Merged`. Pure function — no
/// `Renderer` state needed — kept separate from
/// `Renderer::merge_year_suffix_parts` so it's directly testable.
fn merge_suffix_runs(
    item_parts: Vec<String>,
    suffix_indices: &[Option<u32>],
    ranged: bool,
    suffix_delimiter: &str,
    range_delimiter: &str,
) -> Vec<String> {
    let mut result = Vec::with_capacity(item_parts.len());
    let mut index = 0;
    while index < item_parts.len() {
        let Some(run) = mergeable_run(&item_parts, suffix_indices, index) else {
            if let Some(part) = item_parts.get(index) {
                result.push(part.clone());
            }
            index += 1;
            continue;
        };
        let suffix = if ranged && run.letters.len() >= 3 {
            match (run.letters.first(), run.letters.last()) {
                (Some(first), Some(last)) => format!("{first}{range_delimiter}{last}"),
                _ => run.letters.join(suffix_delimiter),
            }
        } else {
            run.letters.join(suffix_delimiter)
        };
        result.push(format!("{}{suffix}", run.base));
        index = run.end;
    }
    result
}

/// Find the maximal mergeable run starting at `start`, if any.
///
/// A run requires at least two members: a lone suffixed item has nothing to
/// merge with and is left untouched by the caller.
fn mergeable_run(
    item_parts: &[String],
    suffix_indices: &[Option<u32>],
    start: usize,
) -> Option<MergeableRun> {
    let group_index = suffix_indices.get(start).copied().flatten()?;
    let letter = crate::values::int_to_letter(group_index)?;
    let base = item_parts
        .get(start)?
        .strip_suffix(letter.as_str())?
        .to_string();

    let mut letters = vec![letter];
    let mut previous_index = group_index;
    let mut end = start + 1;
    while let Some(candidate_index) = suffix_indices.get(end).copied().flatten() {
        // Require strict adjacency, not just a shared base: a, c, d sharing
        // one year but skipping b (e.g. this cluster cites a and c/d from a
        // larger disambiguation group but not b) must not merge into one
        // run -- "a-d" would falsely imply b was cited too.
        if candidate_index != previous_index + 1 {
            break;
        }
        let Some(candidate_letter) = crate::values::int_to_letter(candidate_index) else {
            break;
        };
        let Some(candidate_base) = item_parts
            .get(end)
            .and_then(|part| part.strip_suffix(candidate_letter.as_str()))
        else {
            break;
        };
        if candidate_base != base {
            break;
        }
        letters.push(candidate_letter);
        previous_index = candidate_index;
        end += 1;
    }

    (letters.len() >= 2).then_some(MergeableRun { base, letters, end })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[rstest]
    #[case::merged_two_letters(
        false,
        &["2019a", "2019b"],
        &[Some(1), Some(2)],
        &["2019a, b"]
    )]
    #[case::merged_three_letters_stay_listed(
        false,
        &["2019a", "2019b", "2019c"],
        &[Some(1), Some(2), Some(3)],
        &["2019a, b, c"]
    )]
    #[case::ranged_two_letters_still_merged(
        true,
        &["2019a", "2019b"],
        &[Some(1), Some(2)],
        &["2019a, b"]
    )]
    #[case::ranged_three_letters_collapse_to_range(
        true,
        &["2019a", "2019b", "2019c"],
        &[Some(1), Some(2), Some(3)],
        &["2019a–c"]
    )]
    #[case::different_years_do_not_merge(
        false,
        &["2019a", "2020a"],
        &[Some(1), Some(1)],
        &["2019a", "2020a"]
    )]
    #[case::lone_suffixed_item_is_untouched(
        false,
        &["2019a", "2024"],
        &[Some(1), None],
        &["2019a", "2024"]
    )]
    #[case::run_stops_at_a_year_change_then_resumes(
        false,
        &["2019a", "2019b", "2021a"],
        &[Some(1), Some(2), Some(1)],
        &["2019a, b", "2021a"]
    )]
    #[case::non_consecutive_suffixes_sharing_a_base_do_not_merge(
        false,
        &["2019a", "2019c"],
        &[Some(1), Some(3)],
        &["2019a", "2019c"]
    )]
    #[case::ranged_non_consecutive_run_never_forms_a_false_range(
        true,
        &["2019a", "2019c", "2019d"],
        &[Some(1), Some(3), Some(4)],
        &["2019a", "2019c, d"]
    )]
    #[case::a_gap_in_the_middle_splits_the_run(
        false,
        &["2019a", "2019b", "2019d"],
        &[Some(1), Some(2), Some(4)],
        &["2019a, b", "2019d"]
    )]
    fn given_a_suffix_run_when_merged_then_matches_expected_parts(
        #[case] ranged: bool,
        #[case] input: &[&str],
        #[case] suffix_indices: &[Option<u32>],
        #[case] expected: &[&str],
    ) {
        let result = merge_suffix_runs(parts(input), suffix_indices, ranged, ", ", "\u{2013}");

        assert_eq!(
            result,
            expected
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_a_ranged_suffix_run_when_delimiter_is_configured_then_it_is_used() {
        let result = merge_suffix_runs(
            parts(&["2019a", "2019b", "2019c"]),
            &[Some(1), Some(2), Some(3)],
            true,
            ", ",
            "~",
        );

        assert_eq!(result, vec!["2019a~c".to_string()]);
    }
}
