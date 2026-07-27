---
# csl26-6p3d
title: 'docs(reference): publish generated reference docs as HTML, strip PR-scoped prose'
status: completed
type: task
priority: normal
created_at: 2026-07-27T22:05:54Z
updated_at: 2026-07-27T22:26:04Z
---

Follow-up to f4209452 (data model reference docs PR). Three fixes: (1) generated reference docs (NATIVE_FORMAT.md, BIBLATEX_MAPPING.md, generated/DATA_MODEL_FIELDS.md, generated/CSL_JSON_MAPPING.md) currently publish as raw markdown on docs.citum.org instead of themed HTML via build-doc-pages.js; (2) DATA_MODEL.md's 'Follow-ups (not in scope here)' section and bean-ID references leaked into outward-facing docs (also in generated BIBLATEX_MAPPING.md via tables.rs source comments); (3) docs/reference.html and docs/schemas/index.html link to FEATURES.md which never landed on main (lost in an abandoned branch, commit 98d3925b), causing a 404. See plan at /home/bruce/.claude/plans/followup-to-earlier-docs-delegated-ripple.md for full detail.

## Summary of Changes

- `scripts/build-doc-pages.js`: added the 4 generated reference docs to `PAGES`; added link-rewriting (raw `.md` cross-links between PAGES entries now resolve to the rendered `.html`) and table-shell wrapping (wide tables get `.doc-table-shell` scroll instead of a bare `<table>`); strips the generated-file 'do not edit' banner comment before rendering. Both fixes apply to the 4 pre-existing PAGES entries too (design-principles.html, migration-strategy.html, type-addition-policy.html), fixing latent same-set .md cross-links and un-shelled tables there as a side effect.
- `scripts/package.json`: added `build:pages" script.
- `.github/workflows/compat-report.yml`: runs `build:pages` before `build:layout` (ordering matters -- build-doc-pages.js emits the LAYOUT_NAV markers build-layout.js fills).
- Untracked the generated reference HTML (`git rm --cached docs/reference/data-model.html`, added `.gitignore` rules) -- matches how behavior-report.html/migration-behavior-report.html are already handled. Markdown sources stay committed (PR review surface + CI drift gate).
- Repointed `docs/reference.html` and `docs/schemas/index.html` cards/links to the new `.html` outputs.
- Removed PR-scoped meta-commentary: `DATA_MODEL.md`'s 'Follow-ups (not in scope here)' section and all bean-ID references (`csl26-7ab8`, `csl26-6eoi`, `csl26-11h2`) from both the hand-written doc and the generator sources (`tables.rs` field notes, `build-data-model-reference.js`'s 'Not Yet Mapped' header) -- fixed at the source, then regenerated via `just schema-gen`.
- Removed the two dangling `FEATURES.md` links from `docs/reference.html` and `docs/schemas/index.html`. Follow-up: csl26-157x (restore the registry).
- New `scripts/check-doc-links.js`: walks `docs/**/*.html`, resolves every relative `.md`/`.html` href against the file's directory, fails on missing targets. Two documented exceptions (behavior-report.html, migration-behavior-report.html -- built by a workflow this job doesn't run) plus one pre-existing unrelated break (`docs/news/index.html`, filed as csl26-hlfh) carved out explicitly with a comment pointing at the bean. Wired into `ci.yml`'s `hygiene-data-model-docs` job (after `build-doc-pages.js` + `build-author-guide.js`, since the checked pages are no longer all committed).
- Verified: `just pre-commit` green (2272 tests); manually re-ran the CI drift gate and the new link checker against a simulated fresh-checkout sequence; confirmed the link checker actually fails when the `FEATURES.md` link is reintroduced.
- Filed csl26-157x (restore FEATURES.md registry -- content decision, not mine to make), csl26-lcnr (RIS mapping reference + TYPE_SYSTEM_ARCHITECTURE.md Draft status, previously tracked only as DATA_MODEL.md prose), csl26-hlfh (pre-existing docs/news/index.html 404 found incidentally while building the link checker).
