# CSLN Style Tier Status

> **Living document** — updated after each significant batch oracle run.
> Last updated: 2026-02-20
>
> **Oracle scoring:** Strict 12-scenario citation set (`tests/fixtures/citations-expanded.json`).
> Hard-fails on processor/style errors. Includes suppress-author, mixed locator/prefix/suffix
> edge cases. Run `node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10` to refresh.

## Top-10 Parent Styles

| Style | Dependents | Citations | Bibliography | Notes |
|-------|-----------|-----------|--------------|-------|
| apa | 783 | 12/12 | 31/31 ✅ | 100% fidelity |
| elsevier-with-titles | 672 | 12/12 | 32/32 ✅ | 100% fidelity |
| elsevier-harvard | 665 | 12/12 | 32/32 ✅ | 100% fidelity |
| elsevier-vancouver | 502 | 12/12 | 32/32 ✅ | 100% fidelity |
| springer-vancouver-brackets | 472 | 12/12 | 32/32 ✅ | 100% fidelity |
| springer-basic-author-date | 460 | 12/12 | 32/32 ✅ | 100% fidelity |
| springer-basic-brackets | 352 | 12/12 | 32/32 ✅ | 100% fidelity |
| springer-socpsych-author-date | 317 | 12/12 | 32/32 ✅ | 100% fidelity |
| american-medical-association | 293 | 12/12 | 32/32 ✅ | 100% fidelity |
| taylor-and-francis-chicago-author-date | 234 | 12/12 | 31/31 ✅ | 100% fidelity |

**Strict 100% citation match (top 10):** 10/10 styles
**Strict 100% bibliography match (top 10):** 10/10 styles

## Style Family Breakdown

### Author-Date (Tier 1 — Active)

Author-date styles targeting the 40% of the corpus now show full strict-match
coverage for the highest-impact parent set.

| Style | Status | Citation Hit Rate | Bibliography Hit Rate |
|-------|--------|------------------|-----------------------|
| apa-7th | ✅ Production | 12/12 | 31/31 |
| elsevier-harvard | ✅ Production | 12/12 | 32/32 |
| springer-basic-author-date | ✅ Production | 12/12 | 32/32 |
| taylor-and-francis-chicago-author-date | ✅ Production | 12/12 | 31/31 |

### Numeric (Tier 2 — Active)

Numeric styles cover ~57% of the corpus. The top Tier-1 numeric parents now pass
strictly, and a new Tier-2 wave has been migrated and enhanced.

| Style | Status | Notes |
|-------|--------|-------|
| elsevier-with-titles | ✅ Production | 12/12 citations, 32/32 bibliography |
| elsevier-vancouver | ✅ Production | 12/12 citations, 32/32 bibliography |
| springer-vancouver-brackets | ✅ Production | 12/12 citations, 32/32 bibliography |
| springer-basic-brackets | ✅ Production | 12/12 citations, 32/32 bibliography |
| american-medical-association | ✅ Production | 12/12 citations, 32/32 bibliography |
| ieee | ✅ Production | 12/12 citations, 32/32 bibliography |

#### Tier-2 Wave: Next 10 Priority Styles (2026-02-20)

| Style | Citation Hit Rate | Bibliography Hit Rate | Notes |
|-------|-------------------|-----------------------|-------|
| springer-mathphys-brackets | 12/12 | 32/32 | Full strict match |
| multidisciplinary-digital-publishing-institute | 12/12 | 32/32 | Full strict match |
| ieee | 12/12 | 32/32 | Citation template + locator support |
| nlm-citation-sequence-superscript | 12/12 | 32/32 | Full strict match |
| nlm-citation-sequence | 12/12 | 32/32 | Full strict match |
| karger-journals | 12/12 | 31/32 | One bibliography mismatch |
| institute-of-physics-numeric | 12/12 | 31/32 | One bibliography mismatch |
| biomed-central | 12/12 | 31/32 | One bibliography mismatch |
| thieme-german | 12/12 | 30/32 | Two bibliography mismatches |
| mary-ann-liebert-vancouver | 12/12 | 30/32 | Two bibliography mismatches |

Wave aggregate:
- Baseline: citations `68/120`, bibliography `313/320` (86.6% fidelity)
- Edited: citations `120/120`, bibliography `313/320` (98.4% fidelity)

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

- **beans:** `csl26-heqm` (top 10 at 100% fidelity), `csl26-gidg` (90% corpus match), `csl26-l2hg` (numeric triage)
- **docs:** `docs/architecture/SQI_REFINEMENT_PLAN.md`, `docs/reference/STYLE_PRIORITY.md`
- **CI:** `.github/workflows/ci.yml` — core fidelity gate (`check-core-quality.js`)
