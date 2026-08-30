---
# csl26-qmxw
title: 'Chicago: status: forthcoming renders as n.d. instead of Forthcoming'
status: in-progress
type: bug
priority: normal
created_at: 2026-08-30T13:28:56Z
updated_at: 2026-08-30T19:12:58Z
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
