# CSLN Style Tier Status

> **Living document** — updated after each significant batch oracle run.
> Last updated: 2026-02-19
>
> **Oracle scoring:** Strict 8-scenario citation set (`tests/fixtures/citations-expanded.json`).
> Hard-fails on processor/style errors. Includes suppress-author, mixed locator/prefix/suffix
> edge cases. Run `node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10` to refresh.

## Top-10 Parent Styles

| Style | Dependents | Citations | Bibliography | Notes |
|-------|-----------|-----------|--------------|-------|
| apa | 783 | 7/8 | 27/27 ✅ | Gold standard; 1 suppress-author edge case |
| elsevier-with-titles | 672 | 7/8 | 25/28 | Close; volume-pages delimiter |
| elsevier-harvard | 665 | 7/8 | ~25/28 | Similar to elsevier-with-titles |
| springer-basic-author-date | 460 | 7/8 | 26/28 | Strong |
| ieee | 176 | — | — | Numeric; not yet hand-authored |
| nature | — | — | — | Not yet hand-authored |
| chicago-author-date | — | — | — | Not yet hand-authored |
| american-medical-association | — | — | — | Not yet hand-authored |
| vancouver | — | — | — | Numeric; not yet hand-authored |

**Strict 100% citation match (top 10):** 0/10 styles
**Strict 100% bibliography match (top 10):** 1/10 styles (APA)

## Style Family Breakdown

### Author-Date (Tier 1 — Active)

Hand-authored styles targeting the 40% of the corpus that use author-date formatting.

| Style | Status | Citation Hit Rate | Bibliography Hit Rate |
|-------|--------|------------------|-----------------------|
| apa-7th | ✅ Production | 7/8 | 27/27 |
| elsevier-harvard | 🔄 In progress | 7/8 | ~25/28 |
| springer-basic-author-date | 🔄 In progress | 7/8 | 26/28 |

### Numeric (Tier 2 — Pending)

Numeric styles cover ~57% of the corpus. Output-driven inference shows mild bibliography
regression for this class (-1.2pp vs XML baseline in n=100 benchmark). Triage tracked in
bean `csl26-l2hg`.

| Style | Status | Notes |
|-------|--------|-------|
| elsevier-with-titles | 🔄 In progress | Inference-first; bibliography gap |
| ieee | ⏳ Queued | Needs hand-authoring |
| vancouver | ⏳ Queued | Needs hand-authoring |

### Note Styles (Tier 3 — Future)

Note styles (footnote-based) are ~19% of corpus. Not yet targeted.

| Style | Status | Notes |
|-------|--------|-------|
| chicago-notes | ⏳ Queued | Requires `position` condition support |
| oscola | ⏳ Queued | Legal citation support needed |

## Refresh Instructions

```bash
# Generate fresh batch report (top 10 styles)
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10

# Generate core quality report (used by CI gate)
node scripts/report-core.js > /tmp/core-report.json

# Check against CI baseline
node scripts/check-core-quality.js \
  --report /tmp/core-report.json \
  --baseline scripts/report-data/core-quality-baseline.json
```

## Related

- **beans:** `csl26-heqm` (top 10 at 100% fidelity), `csl26-gidg` (90% corpus match), `csl26-l2hg` (numeric regression)
- **docs:** `docs/architecture/SQI_REFINEMENT_PLAN.md`, `docs/reference/STYLE_PRIORITY.md`
- **CI:** `.github/workflows/ci.yml` — core fidelity gate (`check-core-quality.js`)
