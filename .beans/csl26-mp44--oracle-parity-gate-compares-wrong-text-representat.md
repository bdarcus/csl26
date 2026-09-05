---
# csl26-mp44
title: Oracle parity gate compares wrong text representation for superscript
status: todo
type: task
priority: normal
created_at: 2026-09-05T21:33:02Z
updated_at: 2026-09-05T21:33:02Z
parent: csl26-ccdt
---

Found tuning csl26-on47 (american-medical-association): fixing AMA's
citation label-wrap from the wrong `brackets` to the CSL-correct
`superscript` (vertical-align="sup" in styles-legacy/american-medical-
association.csl) is a real, verified-correct fix, but yields 0 parity
rows -- every citation-shaped row still fails because
crates/citum-engine/src/render/plain.rs's `superscript()` wraps
content in `^1^` (a Markdown-ish convention, mirroring `**bold**` next
to it), while citeproc-js's plain-text oracle baseline shows no visible
mark at all for the same citation.

Left as-is for now (label-wrap: superscript is still the correct fix,
just parity-neutral) -- this bean is for the broader question raised
review, not a request to touch plain.rs yet:

- The plain-text renderer arguably should not be inventing Markdown
  syntax at all (`^1^`, `**bold**`) -- if PlainText is meant to be
  actual plain text, formatting intent that has no plain-text
  representation should probably just be dropped, not encoded in a
  convention neither citeproc-js nor any real "plain text" consumer
  recognizes.
- Whether the oracle-parity gate should be comparing Citum's
  PlainText output against citeproc-js's plain baseline AT ALL for
  fields carrying only-in-rich-formats semantics (superscript, bold,
  small-caps) -- this may be the wrong output format pairing for the
  comparison, not a Citum defect.
- Whether the exact-parity gate should treat a formatting-
  representation mismatch like this as a real pass/fail signal at
  all, versus excluding it via
  scripts/report-data/parity-adjudication.json (see
  docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md's
  adjudication-state design) or some other mechanism.

Needs a decision on direction (a plain.rs behavior change, an oracle-
comparison-format change, or an adjudication-ledger entry) before any
implementation -- scope span from a single Rust function to the
oracle harness's format pairing, so this is a discussion/spec item,
not a bounded task yet.
