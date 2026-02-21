# CSLN Style Tier Status

> **Living document** — updated after each significant batch oracle run.
> Last updated: 2026-02-21
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
| karger-journals | 12/12 | 32/32 | Full strict match |
| institute-of-physics-numeric | 12/12 | 32/32 | Full strict match |
| biomed-central | 12/12 | 32/32 | Full strict match |
| thieme-german | 12/12 | 32/32 | Full strict match |
| mary-ann-liebert-vancouver | 12/12 | 32/32 | Full strict match |

Wave aggregate:
- Baseline: citations `120/120`, bibliography `313/320` (98.4% fidelity)
- Edited: citations `120/120`, bibliography `320/320` (100.0% fidelity)
- Auto-migrate rerun: citations `120/120`, bibliography `299/325` (94.2% fidelity)

#### Tier-2 Wave: Next 20 Priority Styles (2026-02-20)

| Style | Citation Hit Rate | Bibliography Hit Rate | Notes |
|-------|-------------------|-----------------------|-------|
| taylor-and-francis-national-library-of-medicine | 12/12 | 32/32 | Full strict match after locator citation fix |
| american-chemical-society | 12/12 | 32/32 | Full strict match |
| springer-fachzeitschriften-medizin-psychologie | 11/12 | 32/32 | One remaining mixed-visibility citation mismatch |
| springer-humanities-author-date | 12/12 | 30/32 | Disambiguation + short-author citation pass |
| landes-bioscience-journals | 12/12 | 31/32 | Citation template normalized |
| taylor-and-francis-council-of-science-editors-author-date | 12/12 | 31/32 | Disambiguation + locator citation pass |
| bmj | 12/12 | 31/32 | Citation template normalized |
| springer-physics-brackets | 12/12 | 11/32 | Citation fixed; bibliography still major outlier |
| frontiers | 12/12 | 32/32 | Full strict match after citation disambiguation tuning |
| baishideng-publishing-group | 12/12 | 31/32 | Citation template normalized |
| royal-society-of-chemistry | 12/12 | 15/33 | Citation fixed; bibliography still major outlier |
| association-for-computing-machinery | 12/12 | 31/32 | Locator citation formatting fixed |
| chicago-shortened-notes-bibliography | 12/12 | 31/31 | Patent type-template + personal-comm suppression added |
| nature | 12/12 | 31/32 | Citation template normalized |
| copernicus-publications | 12/12 | 31/32 | Disambiguation + short-author citation pass |
| springer-socpsych-brackets | 12/12 | 31/32 | Citation template normalized |
| american-society-of-civil-engineers | 12/12 | 31/32 | Disambiguation + short-author citation pass |
| cell | 12/12 | 31/32 | Citation template normalized |
| springer-mathphys-author-date | 12/12 | 31/32 | Disambiguation + short-author citation pass |
| begell-house-chicago-author-date | 12/12 | 29/32 | Citation disambiguation fixed |

Wave aggregate:
- Baseline: citations `157/240`, bibliography `584/641` (84.1% fidelity)
- Edited: citations `239/240`, bibliography `584/641` (93.4% fidelity)
- Auto-migrate rerun: citations `157/240`, bibliography `584/641` (84.1% fidelity)

#### Tier-2/3 Wave: Next 58 Priority Styles (2026-02-20)

Batch scope:
- Selected the next 58 parent styles from `csln-analyze --rank-parents` not
  already represented in `styles/`.
- Core style catalog expanded from `42` to `100` YAML styles.

Wave aggregate:
- Edited (presetized + citation-sync): citations `607/696`,
  bibliography `1722/1824` (91.5% fidelity)
- Auto-migrate rerun: citations `607/696`, bibliography `1720/1824`
  (91.4% fidelity)
- Delta vs rerun: citations `+0`, bibliography `+2`

Preset extraction across the 58-style wave:
- `options.dates` presetized in `58` styles (`long`/`short`/`numeric`)
- `options.titles` presetized in `39` styles (`humanities`/`journal-emphasis`)
- `options.substitute` presetized in `36` styles (`editor-*` variants)
- contributor presetization in `11` styles (`numeric-compact`/`numeric-medium`)

### Note Styles (Tier 3 — Partial)

Note styles (footnote-based) are ~19% of corpus. One is now production-quality;
full ibid/subsequent support requires `position` condition (not yet implemented).

| Style | Status | Notes |
|-------|--------|-------|
| chicago-shortened-notes-bibliography | ✅ Production | 13/13 citations, 31/31 bibliography |
| chicago-notes | ⏳ Queued | Requires `position` condition support |
| oscola | ⏳ Queued | Legal citation support needed |

### Author-Format (MLA)

| Style | Status | Notes |
|-------|--------|-------|
| modern-language-association | 🔄 In Progress | Being authored via @styleauthor |

## Embedded Styles (Built into Binary)

Twelve priority styles are compiled into the `csln` binary via `include_bytes!`
and can be used without any style file on disk.

```bash
# List all embedded styles
csln styles list

# Render using a builtin style
csln render refs --builtin apa-7th -b refs.json
```

| Style | Format | Dependents | Corpus Impact |
|-------|--------|-----------|---------------|
| apa-7th | author-date | 783 | 9.8% |
| elsevier-harvard | author-date | 665 | 8.3% |
| elsevier-with-titles | numeric | 672 | 8.4% |
| elsevier-vancouver | numeric | 502 | 6.3% |
| springer-basic-author-date | author-date | 460 | 5.8% |
| springer-vancouver-brackets | numeric | 472 | 5.9% |
| springer-basic-brackets | numeric | 352 | 4.4% |
| american-medical-association | numeric | 293 | 3.7% |
| ieee | numeric | 176 | 2.2% |
| taylor-and-francis-chicago-author-date | author-date | 234 | 2.9% |
| chicago-shortened-notes-bibliography | note | 38 | 0.5% |
| modern-language-association | author | — | — |

**Total embedded coverage: ~4,647 dependents (~58% of the 7,987-style ecosystem)**

Embedded locales: `en-US`, `de-DE`, `fr-FR`, `tr-TR` (~64 KB combined).
Total binary size increase: ~121 KB raw YAML (~1.9% of the 6.4 MB release binary).

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
