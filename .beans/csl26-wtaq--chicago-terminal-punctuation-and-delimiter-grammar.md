---
# csl26-wtaq
title: 'Chicago: terminal punctuation and delimiter grammar'
status: in-progress
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - punctuation
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-09-04T13:37:12Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 221 entries, punctuation-only divergence (delimiter choice, terminal period vs comma, etc). Supersedes csl26-vf5x (cluster 3, container-title terminal punctuation before volume/issue -- that bean's own 2026-08-08 re-check found the described pattern 'not found in current failures, likely stale') and overlaps csl26-yqma (cluster 4, name-list conjunction punctuation -- re-check found 'largely unconfirmable'). Both superseded beans' original hypotheses are subsumed by this larger, freshly measured class. Touches all four Chicago variants.

## Session scope (2026-09-04)

Landing the terminal-mark collision fix here first (PR 1 of a stack): `append_rendered_component`/`move_punctuation_into_quote` in `crates/citum-engine/src/render/bibliography.rs` doesn't run the `is_terminal_punctuation` + `resolve_punctuation_collision` check the citation path (`render/citation.rs:78-84`) already has, so `?"` + entry-suffix period doesn't collapse and abbreviation-ending periods double (`Jr..`). 10 confirmed sole-cause row flips across the family. See /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md for full plan.

## Summary of Changes (PR 1, this session)

Fixed a terminal-mark collision defect in the bibliography-render punctuation-in-quote path: `append_rendered_component`/`append_entry_suffix` (crates/citum-engine/src/render/bibliography.rs) inserted a separator's or entry-suffix's leading period/comma into a closing quote unconditionally, without checking whether the character just inside the quote was already terminal punctuation -- doubling marks like `?"` -> `?".`. Added `char_before_insertion_point` (render/punctuation.rs) and gated both call sites on `ends_with_close_quote` so the collision-absorb check only fires when a genuine closing quote is present (an earlier ungated version regressed american-medical-association by swallowing a dangling-suffix period unrelated to any quote -- caught by the full-portfolio per-entry diff, not the aggregate). Also fixed a sibling self-delimiting-component collision case in `append_rendered_component`'s `starts_with_separator` branch (e.g. "Jr.." from a name suffix meeting a group's own literal ". " prefix).

Measured: 10 sole-cause row flips found in initial triage; 7 landed after excluding rows masked by PlainText's own emph markup (`_..._`), a separate, already-tracked and deliberately-scoped gap (bean csl26-ztxq, status completed) -- not reopened here. Family: 546/1,629 -> 553/1,629 exact parity (chicago-author-date-18th 215->218/542, taylor-and-francis-chicago-author-date 215->218/542, chicago-shortened-notes-bibliography 86->87/473, chicago-notes-18th unchanged). Zero regressions across all 35 embedded styles (full per-entry diff, not aggregate). Full pre-commit gate green: fmt, clippy -D warnings, cargo nextest run (2,754/2,754). Regenerated scripts/report-data/embedded-parity-baseline.json.

Remaining terminal-punctuation/delimiter rows in this bean's original 221-row class are mostly YAML template gaps (missing date detail, contributor ordering, etc.), not further instances of this specific engine defect -- bean stays open, scoped down.
