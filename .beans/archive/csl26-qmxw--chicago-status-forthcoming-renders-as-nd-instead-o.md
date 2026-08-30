---
# csl26-qmxw
title: 'Chicago: status: forthcoming renders as n.d. instead of Forthcoming'
status: completed
type: bug
priority: normal
created_at: 2026-08-30T13:28:56Z
updated_at: 2026-08-30T20:35:36Z
---

Surfaced while row-accounting csl26-rrsb (year-suffix engine fix): several
chicago-author-date-18th bibliography entries carry a `status: forthcoming`
field (e.g. references V54M6HLX/JXGCXGLD/9RPXBW6V/94SYPMEQ in
tests/fixtures/test-items-library/chicago-18th.json — "Author, Jane Q.",
"Contributor, Anna.", "Faraday, Carry.", "Smith, Margaret.") but the date
slot renders the locale's "n.d." no-date term instead of CMOS's
"Forthcoming." term. Oracle: `Author, Jane Q. Forthcoming. Book Title.
Publisher.` Citum: `Author, Jane Q. n.d. Book Title. Publisher.`

These rows were classified under the analyze-parity-residuals.js "C
year-suffix letter" bucket (the single-letter-vs-empty regex the classifier
uses matches "n.d." vs "Forthcoming." superficially) but are not a
year-suffix defect at all — the reference's `status` field isn't consulted
by the date-rendering fallback chain (crates/citum-engine/src/values/date.rs)
for a forthcoming-status term, unlike real CSL/CMOS. Not touched by
csl26-rrsb's range-key or article-sort fixes.

Needs: trace whether `status` is read anywhere in the issued-date fallback
chain (effective_date_fallback_candidates / render_date_fallback_chain) and
whether Chicago's locale has a "forthcoming" term to route to; if not, this
is either an engine gap (status-aware date fallback) or a locale/YAML gap
(missing type-variant date-fallback for forthcoming status) — classify with
the conversion-layer pre-flight before touching YAML.

Design spec: docs/specs/STATUS_DATE_FALLBACK.md (Draft). Adds a new `DateFallbackCandidate::Variable` schema arm rather than a generic `TemplateConditionField::Status` — see spec's Rejected Alternatives.

## Summary of Changes

Added `DateFallbackCandidate::Variable` (`crates/citum-schema-style/src/options/date_fallback.rs`) — renders a reference's own variable (e.g. `status`) as a fallback candidate, alongside the existing `Date`/`Message` arms. `render_date_fallback_chain` (`crates/citum-engine/src/values/date.rs`) extended to append the year-suffix disambiguation letter after a `Variable` candidate the same way it does for `Message`.

`chicago-author-date-18th.yaml`'s `bibliography.options.date-fallback` moved from the `standard` preset to its explicit equivalent with a `variable: status, text-case: capitalize-first` candidate inserted ahead of the terminal `message: term.no-date` — matching `chicago-author-date.csl`'s own `<else-if variable="status">` branch (CSL prints the reference's actual status text, not a generic term). Scoped to the bibliography date slot only — the citation-form macro (`date-short` in the source CSL) uses `text-case="lowercase"` for the same text, a distinct follow-up not covered here (noted in the style file and spec).

Verified against real citeproc-js: all 4 `chicago-18th.json` fixture references carrying `status: forthcoming` (`V54M6HLX`, `JXGCXGLD`, `9RPXBW6V`, `94SYPMEQ`) now render `Forthcoming.` and exactly match the oracle; 0 other entries in the 393-item corpus moved.

Spec: `docs/specs/STATUS_DATE_FALLBACK.md` (Active). Schema regenerated (`just schema-gen`).

Verification: `cargo build --workspace`, `cargo check --workspace --all-targets --all-features`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` all clean. `cargo nextest run -p citum-engine -p citum-schema-style`: 1939/1939 pass (both touched crates). The full-workspace `cargo nextest run` could not be completed in this session — it was repeatedly interrupted by the environment before finishing (not a test failure; every partial run up to interruption showed passing tests, and the touched crates' own scoped run is clean). A full-workspace nextest run should be re-confirmed once the branch is pushed and CI runs it.

Closes csl26-qmxw.
