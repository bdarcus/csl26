---
# csl26-5753
title: Implement true by-cite per-position given-name expansion ceiling
status: completed
type: feature
priority: normal
tags:
    - engine
    - disambiguation
    - citation
created_at: 2026-08-16T18:30:22Z
updated_at: 2026-08-17T12:21:37Z
---

Follow-up from csl26-8nrt. docs/specs/DISAMBIGUATION.md §2.1.1 was corrected
2026-08-16: name disambiguation always compares against every reference in the
document, for all givenname-disambiguation-rule values, including by-cite. The
by-cite implementation that violated this (csl26-lvib, 2026-06-02) has been
removed. As a result, by-cite and all-names are now behaviorally identical in
Citum -- neither implements the escalation-cap semantics real by-cite has in
citeproc-js.

citeproc-js's actual by-cite behavior (scripts/node_modules/citeproc/citeproc_commonjs.js):
- Ambiguity detection is always registry-wide (CSL.Registry.ambigcites),
  regardless of givenname-disambiguation-rule.
- by-cite is rewritten to all-names for *position selection* purposes
  (`if (gdropt === "by-cite") { gdropt = "all-names"; }`).
- What actually varies is an escalation *ceiling*: `this.givensMax = 2` when
  by-cite + disambiguate-add-givenname are both active, plus a `request_base`
  floor check. This caps how far a single rendered cite is forced to expand --
  e.g. a cite showing two visible authors isn't forced to add given names for
  a third author hidden behind et-al in that same cite -- without narrowing
  which references are compared to detect the collision in the first place.

Citum's ProcHints model expresses given-name expansion as an all-or-nothing
flag (expand_given_names) plus a primary-only restriction
(expand_given_names_primary_only). There is no way to express "expand given
names for these specific rendered positions in this specific cite, capped at
N, per citeproc-js's request_base/givensMax logic" -- implementing true
by-cite requires that finer-grained model.

## Scope
- [x] Design the per-position/per-cite expansion representation: `ProcHints.expand_given_names_full_positions: Option<Vec<bool>>`, index-aligned to the rendered name list. The uniform `expand_given_names` flag still governs whether a position's given name is revealed at all -- this is Citum's own simplification, not a citeproc-js match: citeproc-js keeps that reveal itself per position too (its `request_base` floor), which Citum doesn't yet model (tracked as csl26-7jej). This field only overrides per-position depth (initials vs full).
- [x] Ported the escalation-cap search as a recursive left-to-right positional resolver (`Disambiguator::select_by_cite_resolution` / `resolve_by_cite_positions` in `processor/disambiguation.rs`), rather than a literal port of citeproc-js's base/betterbase state machine -- behaviorally equivalent for every fixture checked, including cross-position joint splitting (CrossNestedNames).
- [x] Added native fixtures mirroring `disambiguate_ByCiteMinimalGivennameExpandMinimalNames` and `disambiguate_ByCiteGivennameExpandCrossNestedNames` exactly; re-split the by-cite test that previously documented the uniform-escalation divergence as intentional (it's now minimal, matching the oracle).
- [x] Updated `docs/specs/DISAMBIGUATION.md` §2.1.1 acceptance criteria and changelog, plus `docs/reference/DISAMBIGUATION.md`.

## Summary of Changes

Implemented true per-position given-name escalation for `GivennameRule::ByCite`:

- **`ProcHints.expand_given_names_full_positions: Option<Vec<bool>>`** (`values/mod.rs`) -- `None` for every other rule (byte-identical fallback to the existing uniform `expand_given_names_full`); `Some(positions)` for by-cite, one bool per author position.
- **`Disambiguator::select_by_cite_resolution`** tries strategy 1 (name-count growth via `disambiguate-add-names`) first, exactly like every other rule, then hands any still-colliding family bucket to `resolve_by_cite_positions`, a recursive left-to-right search that escalates one position at a time, only committing an escalation when it actually reduces the bucket's remaining collisions, and only grows the shown name count when strategy 1 is enabled.
- **Rendering** (`values/contributor/names.rs`, `positional_expand`) reads the per-position override when present, falling back to the uniform flags otherwise.
- **Bug caught by a corpus sweep**, not by unit tests: an early version grew the shown name count unconditionally, even when `disambiguate-add-names` was off. `report-core.js --all-features` (worktree-based before/after diff against `main`) caught 3 regressed styles (elsevier-harvard, elsevier-vancouver-author-date, gb-t-7714-2025-author-date) rendering full given-name expansions where the oracle prefers plain year-suffix. Fixed by gating the n-growth step on `flags.add_names`. Re-swept after the fix: **zero regressions, one genuine +2-entry gain** (gb-t-7714-2025-author-date bibliography, given names now correctly capped at initials instead of over-escalating to full).
- **Tests:** all 1330 `citum-engine` unit/integration tests pass; 2 new native tests added (`disambiguation_givenname_escalation_is_minimal_per_position`, `disambiguation_givenname_escalation_splits_positions_independently_per_collision`), mirroring the official CSL test suite's `disambiguate_ByCiteMinimalGivennameExpandMinimalNames`/`disambiguate_ByCiteGivennameExpandCrossNestedNames` fixtures exactly.
- **Gate:** `just pre-commit` (fmt, clippy -D warnings, full nextest — 2588 tests) passes clean.

csl26-jdp6 (name order/punctuation divergence, unrelated) is intentionally left untouched for a separate pass.
