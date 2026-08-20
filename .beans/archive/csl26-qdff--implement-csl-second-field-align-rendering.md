---
# csl26-qdff
title: Implement CSL second-field-align rendering
status: completed
type: task
priority: deferred
tags:
    - csl
    - engine
    - bibliography
created_at: 2026-08-03T18:50:04Z
updated_at: 2026-08-20T23:55:19Z
---

Track CSL `second-field-align` support separately from declarative numeric label generation, including runtime bibliography layout and aligned rendering.

- [x] Define the runtime bibliography layout model.
- [x] Implement aligned label/body rendering for supported output formats.
- [x] Add migration, fixture, and parity coverage.

## Summary of Changes

`docs/specs/SECOND_FIELD_ALIGN.md` (spec PR, stacked below) pins the design.
This bean's implementation lands as `feat/second-field-align`, stacked on the
spec via `gh stack`:

- `crates/csl-legacy`: parses `<bibliography second-field-align="…">`.
- `crates/citum-schema-style`: new `second-field-align: flush|margin` option
  on `BibliographyConfig`/`BibliographyOptions`, wired through
  `to_bibliography_config`, `merge`, and inheritance (`schema-gen` run).
- `crates/citum-engine`: new `BibliographyLayout` runtime model and
  `OutputFormat::entry_slots` seam. Default impl is byte-identical to the
  prior unconditional marker/body fuse (`self.affix(marker, body, "")`), so
  every format except `Html` is unaffected by construction. `Html` emits
  sibling `citum-entry-marker`/`citum-entry-body` divs when alignment is
  declared. `hanging-indent` (previously parsed, migrated, and consumed by
  nothing) now renders a `citum-bibliography--hanging-indent` container
  class.
- `crates/citum-migrate`: extracts the new option; removed the stale
  "second_field_align is missing in my model read" placeholder comment.
- `bibliography_label_missing_separator_warnings` now treats a declared
  `second-field-align` as the affirmative signal that a flush marker is
  intentional, suppressing the warning.

**No shipped style declares `second-field-align`** — zero HTML/text output
change from that option alone. `hanging-indent`, however, is already declared
`true` on 11 shipped styles (7 embedded-core: apa-7th,
chicago-author-date-18th, chicago-shortened-notes-bibliography-core,
elsevier-harvard-core, gb-t-7714-2025-author-date,
modern-language-association, springer-basic-author-date-core; 3 other:
chicago-notes-bibliography-17th-edition,
international-journal-of-wildland-fire, mhra-notes; 1 experimental:
jm-turabian-multilingual) — closing that dead field is a real, intentional
HTML markup change for those styles' bibliography container (an added CSS
class only; visible text unchanged). Two integration tests
(`crates/citum-server/tests/rpc.rs`, `crates/citum-engine/tests/document.rs`)
asserted the old exact container markup for apa-7th/MLA and were updated.

**Verified:**
- `just pre-commit` (fmt, clippy `-D warnings`, `cargo nextest run`):
  2683/2683 passed.
- `node scripts/report-core.js --all-features`: 18/19 embedded-parity-tracked
  styles byte-identical to the checked-in baseline; the one diff
  (springer-basic-author-date, exactParity 53/67 → 54/67) is confirmed
  unrelated — already landed on `main` before this branch, at commit
  `c5e6fb15` ("feat(engine)!: render year-suffix collapse degrees"), which
  documents that exact delta in its own commit message.
- `just check-core-quality`: gate passed (35 styles, fidelity 1.0 for all,
  exact-parity ≥ baseline for 19 embedded-core styles).

**Follow-ups filed as separate beans, not done here** (mechanism-only scope,
per the plan): corpus adoption — declaring `second-field-align` on the ~12
shipped numeric styles whose CSL source actually carries the attribute.
