---
# csl26-nrks
title: Fixture-cache invalidation is whole-file, not per-entry
status: completed
type: task
priority: normal
tags:
    - test-infrastructure
    - scripts
    - architecture
created_at: 2026-08-16T23:42:13Z
updated_at: 2026-08-17T00:16:52Z
---

Found while adding a regression fixture for csl26-h9jy (a same-initial
givenname-disambiguation bug). Adding 2 harmless reference entries to
tests/fixtures/references-expanded.json -- referenced by no existing
citation of any other style -- regenerated ~2,845 citeproc-js oracle
snapshots (~186k lines) and cascaded into three more independently
pinned baselines going stale in sequence, each discovered only by a
separate CI job going red. Dropped that fixture change entirely rather
than carry the cost (see PR #1201 / bean csl26-h9jy's "Follow-up
verification (not committed)" note) -- the bug already had full native
Rust regression-test coverage, so nothing was actually lost by not
committing the fixture. But the *mechanism* is worth fixing on its own
merits, independent of that specific bug.

## Root cause 1: whole-file hash, not per-entry

`scripts/oracle-snapshot.js`'s cache key is a single SHA-256 over the
*entire contents* of both fixture files concatenated:

```js
function fixtureHash(refsFixture, citationsFixture) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(refsFixture));
  h.update(fs.readFileSync(citationsFixture));
  return h.digest('hex').slice(0, 16);
}
```

`references-expanded.json` is `DEFAULT_REFS_FIXTURE` in
scripts/lib/style-verification.js -- the fallback for every style
without an explicit `fixture_family`, i.e. nearly the whole corpus.
Any single-byte change anywhere in either fixture file invalidates
every style's cached snapshot, regardless of whether that style's
citations reference anything that changed. The invalidation key is
coarser than the actual unit of change (which citation/reference IDs
a given style's fixture entries actually touch).

## Root cause 2: no single regeneration entrypoint

A fixture edit fans out into at least four independently-pinned
artifacts, discoverable only by pushing and watching CI fail one job
at a time:
1. tests/snapshots/csl/*.json (oracle-snapshot.js --all)
2. Registered coverage-audit manifests' pinned fixture hashes
   (docs/architecture/audits/*/manifest.yaml, via
   style-coverage-review.js --update-manifest)
3. scripts/report-data/embedded-parity-baseline.json (hand-assembled
   from a report-core.js --all-features run; no dedicated generator
   script found)
4. scripts/report-data/oracle-top10-baseline.json (a *separate* tool,
   oracle-batch-aggregate.js, against raw styles-legacy/ CSL)

The `oracle-refresh` just recipe (`node scripts/oracle-batch-aggregate.js
styles-legacy/ --top 10`) doesn't even pass `--save`, so it doesn't
actually write the baseline it's presumably named to refresh.

## Root cause 3 (unconfirmed -- needs someone with history context)

embedded-parity-baseline.json (built from report-core.js, tier=embedded
styles) and oracle-top10-baseline.json (built from oracle-batch-aggregate.js,
raw styles-legacy/ CSL) track overlapping citation/bibliography
pass-counts for overlapping style sets, via two independent code paths
with two different JSON shapes, checked by two different scripts in two
different CI steps. Could be intentional (one exercises the migration
pipeline end-to-end, the other the already-embedded style directly) or
accidental duplication from two separate initiatives -- not verified
either way this session.

## Scope
- [x] Investigated whole-file hash re-keying -- REFUTED, not fixed. Measured:
      `oracle-snapshot.js` calls `engine.updateItems(Object.keys(testItems))`,
      so citeproc-js renders every fixture entry into every style's
      bibliography (apa: 46/47 rows, 29 never cited). Adding/editing any
      entry in the shared default genuinely changes every snapshot; the
      whole-file hash is the correct granularity, not too coarse. Full
      writeup, measurement, and rejected alternatives in
      docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md. The
      actionable outcome is a policy, not code: put regression fixtures in a
      scoped fixture family (`FIXTURE_SET_REFS` in
      scripts/lib/style-verification.js), not the shared default.
- [x] Added `just fixture-refresh` (justfile) -- single entrypoint for
      snapshots + coverage-audit manifests always, plus the two ratcheted
      baselines behind an explicit `baselines=yes` flag (they stay
      dedicated-PR-only per baselines/README.md).
- [x] Audited embedded-parity-baseline.json vs oracle-top10-baseline.json --
      confirmed intentional, not duplication: one exercises the migration
      pipeline against raw styles-legacy/ CSL, the other gates hand-tuned
      embedded YAML directly. Documented in the audit so it doesn't need
      re-deriving. Keeping both.
- [x] Fixed `just oracle-refresh` -- now passes `--save` and drives its style
      list from the baseline's own `metadata.styles` instead of `--top 10`,
      so it actually writes the file it claims to refresh and refreshes the
      same style set `check-oracle-regression.js` checks against.

## Summary of Changes

- `docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md` -- full
  investigation, measurement, rejected alternatives, and fan-out map.
- `scripts/derive-parity-baseline.js` (+ test) -- new generator for
  `embedded-parity-baseline.json`, which previously had none.
- `justfile` -- fixed `oracle-refresh`; added `fixture-refresh` fan-out
  recipe.
- `baselines/README.md` -- points at the new recipe.
- No fixture, snapshot, manifest, or baseline was regenerated in this PR.

Follow-ups filed rather than folded in: `csl26-u87d` (2 styles fail
oracle-snapshot rendering with a locale-loading error), `csl26-y36e`
(blocked by `csl26-u87d`; ~2,800 stale version-1 snapshots need a dedicated
regeneration commit -- discovered via a real `just fixture-refresh` dry run
that was reverted before committing).
