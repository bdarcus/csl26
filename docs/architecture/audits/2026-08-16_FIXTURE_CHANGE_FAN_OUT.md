# Fixture-Change Fan-Out: How to Add a Fixture Without a Corpus-Wide Diff

- **Date:** 2026-08-16
- **Bean:** `csl26-nrks`
- **Question:** adding 2 harmless reference entries to
  `tests/fixtures/references-expanded.json` while working `csl26-h9jy`
  regenerated ~2,845 citeproc-js oracle snapshots (~186k lines) and then
  cascaded into three more independently-pinned baselines going stale one CI
  job at a time, each discovered only when a separate CI job went red. Why
  does a 2-entry change produce a 187k-line diff, and how do we stop that
  from happening again?

## The answer: it's where you add the entry, not the cache key

`oracle-snapshot.js`'s cache key (`fixtureHash`, `:103`) hashes exactly two
files: the shared default references and citations fixtures. Nothing else is
an input. An entry added to any *other* file — a fixture scoped to a
`fixture_family`, or a native Rust test with no CSL-JSON fixture at all — is
therefore never read by that hash, and **zero of the 2,845 snapshots
invalidate**. The 187k-line diff wasn't a caching bug; it was a consequence
of adding the entries to `references-expanded.json`, which every one of
those 2,845 legacy styles' oracle snapshots is generated against.

**The lever that controls blast radius already existed and was simply not
used: a regression entry belongs in a scoped fixture family, never in the
shared default.** `scripts/lib/style-verification.js` already defines
`FIXTURE_SET_REFS` / `FIXTURE_SET_CITATIONS` (`humanities-note`,
`secondary-roles`, and others), selected per style via `fixture_family` in
`scripts/report-data/verification-policy.yaml`. `references-expanded.json` is
`DEFAULT_REFS_FIXTURE` — the fallback for every style without an explicit
family, i.e. nearly the whole corpus. Editing it is, by definition, a
corpus-wide change; that was true before this audit and remains true after.
A same-initials givenname-disambiguation regression fixture like the one
that prompted this bean belongs in a family fixture (or a new one) scoped to
the style(s) it's regressing, not in the shared default — or, since the bug
already had full native Rust regression coverage, no oracle fixture at all.

This is now the enforced default: `.claude/skills/test-coverage/SKILL.md`'s
"Adding fixture data" section, which previously told contributors to add to
`references-expanded.json` unconditionally, now leads with this decision
rule.

The rest of this document is supporting evidence for that answer (why
re-keying the cache instead wouldn't have helped) and the separate fan-out
and baseline-duplication questions the bean also raised.

## Why re-keying the cache wouldn't have helped

The bean's original hypothesis was that `scripts/oracle-snapshot.js`'s cache
key — a single SHA-256 over the entire contents of both fixture files — is
coarser than the real unit of change, and that re-keying it
per-style-per-citation-id (or at minimum per referenced entry-set) would make
a fixture addition scale with what actually changed rather than with corpus
size. That's a different fix from "add it elsewhere," and it doesn't hold up
on its own terms: it's not that the hash is wrong, it's that *every entry in
the shared default is a real input to every style's rendered bibliography*,
so no cache key over that file's contents can be narrower than "changed."

`renderWithCiteprocJs` (`scripts/oracle-snapshot.js:166`) calls:

```js
const engine = new CSL.Engine(sys, styleXml);
engine.updateItems(Object.keys(testItems));
```

`updateItems` is called with **every ID in the loaded references fixture**,
not the IDs a style's citations actually reference. citeproc-js therefore
renders a bibliography row for every fixture entry, cited or not, for every
style. Measured directly against the committed snapshot:

```
$ node -e "
const refs = require('./tests/fixtures/references-expanded.json');
const ids = Object.keys(refs).filter(k => k !== 'comment');
const cites = require('./tests/fixtures/citations-expanded.json');
const used = new Set();
for (const c of cites) for (const i of c.items) used.add(i.id || i);
const snap = require('./tests/snapshots/csl/apa.json');
console.log('refs entries:', ids.length);
console.log('citations:', cites.length, 'distinct cited ids:', used.size);
console.log('apa bibliography rows:', snap.bibliography.length);
console.log('bib rows never cited:', snap.bibliography_ids.filter(id => !used.has(id)).length);
"
refs entries: 47
citations: 20 distinct cited ids: 17
apa bibliography rows: 46
bib rows never cited: 29
```

47 fixture entries produce 46 bibliography rows (one entry is filtered by the
style), and 29 of those 46 rows — nearly two-thirds — belong to references no
citation in `citations-expanded.json` ever mentions. They exist purely to
give the oracle bibliography-only reference-type coverage (dates, contributor
role combinations, unusual field shapes) that the citation set doesn't
exercise. Since every style's bibliography is rendered from the full entry
set, adding, removing, or editing **any** entry in
`references-expanded.json` genuinely changes every style's rendered output.
The whole-file hash over that fixture is the correct granularity for what it
protects — not an over-eager invalidation.

### Rejected alternatives

**Scope `updateItems()` to cited IDs only.** This would let the cache key
narrow to the entries a style's citations actually touch. Cost: bibliography
rendering would drop from 46 rows to 17 (the distinct cited IDs) per style,
for every one of the ~2,845 snapshots — destroying exactly the uncited
reference-type coverage the fixture was built to carry, and still requiring
a one-time rewrite of all 2,845 snapshots to apply the narrower rendering.
Rejected: it trades away test coverage to buy a caching property nothing
downstream currently needs.

**Canonicalize the hash input instead of hashing raw bytes.** `loadFixtures`
(`scripts/oracle-snapshot.js:144`) already strips the fixture's `comment` key
before use, but `fixtureHash` (`:103`) hashes the raw file bytes, so editing
`comment`, reordering keys, or reformatting whitespace still invalidates
every snapshot even though nothing citeproc-js sees has changed. This is a
real, narrow gap — but closing it only protects non-semantic edits; it does
nothing for the case that actually triggered this bean (adding real entries),
and still costs a one-time rewrite of all 2,845 snapshots to re-pin the new
hash. Left as a candidate for a future PR, not bundled here since it doesn't
address the bean's original incident.

## The fan-out map (Root cause 2)

A fixture edit — scoped or not — still fans out into independently-pinned
artifacts, previously discoverable only by pushing and watching CI fail one
job at a time:

1. `tests/snapshots/csl/*.json` — regenerated by `oracle-snapshot.js --all`.
2. Registered coverage-audit manifests
   (`docs/architecture/audits/*/manifest.yaml`, registered in
   `scripts/report-data/report-provenance.yaml`'s `coverage_audits` list) —
   each manifest pins `fixtures.references.sha256` **and**
   `expected-observations`, both of which move when a referenced fixture
   moves. An entrypoint to regenerate these already existed but wasn't wired
   into a single command: `scripts/refresh-style-coverage-audits.js` iterates
   every registration and re-runs `style-coverage-review.js` for each.
3. `scripts/report-data/embedded-parity-baseline.json` — hand-assembled from
   a `report-core.js --all-features` run; no generator script existed before
   this change (see below).
4. `scripts/report-data/oracle-top10-baseline.json` — written by
   `oracle-batch-aggregate.js --save`, but the `just oracle-refresh` recipe
   that's presumably meant to drive it never passed `--save` (fixed in this
   change; see the companion `justfile` diff).

This change adds `just fixture-refresh` as the single entrypoint that owns
steps 1–4 (baselines 3 and 4 gated behind the explicit
`just fixture-refresh yes` form — see "What did not change" below), and
`scripts/derive-parity-baseline.js` as the missing generator for artifact 3.

## Root cause 3, answered: the two baselines measure different pipelines, not the same thing twice

`oracle-top10-baseline.json` and `embedded-parity-baseline.json` track
overlapping citation/bibliography pass-counts for overlapping style *names*
(`apa`/`apa-7th`, `ieee`, `cell`/`elsevier-*`), via two different code paths,
checked by two different CI steps — which looked like it could be accidental
duplication from two separate initiatives. It isn't:

- `oracle-top10-baseline.json` is written by `oracle-batch-aggregate.js`
  running against **raw `styles-legacy/*.csl`** for the 10 hard-pinned
  `PRIORITY_STYLES` (`oracle-batch-aggregate.js:31-49`). Its metadata records
  `styleSelector: "explicit"` with that pinned style list — it exercises the
  **migration pipeline end-to-end**: legacy CSL 1.0 XML → `citum-migrate` →
  render, and catches migration-converter regressions.
- `embedded-parity-baseline.json` is written from `report-core.js
  --all-features`, filtered to `tier === "embedded"` — the 19 curated,
  hand-tuned `styles/embedded/*.yaml` files. It gates exact-parity floors on
  the **already-migrated, hand-authored style** directly, with no migration
  step in the loop, and is what `check-core-quality.js --parity-baseline`
  enforces per `docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md`.

Same author styles, same oracle (citeproc-js), different subject under test.
Verdict: **keep both**, and this document is the confirmation that the split
is intentional rather than an artifact of scope drift, so a future audit
doesn't need to re-derive it.

## What changed

- `scripts/derive-parity-baseline.js` (new) — generates
  `embedded-parity-baseline.json` from a `report-core.js --all-features`
  report, closing the "no dedicated generator script found" gap in fan-out
  artifact 3. Reproduces the file's existing shape field-for-field
  (`generated`/`commit`/`source`/`purpose`/`styles{...}`), since
  `.github/workflows/fidelity.yml` and `check-core-quality.js` both read that
  shape positionally.
- `justfile`: `oracle-refresh` now passes `--save` and derives its style list
  from the baseline's own `metadata.styles` instead of `--top 10`, so a
  refresh actually writes the file it claims to refresh, and refreshes the
  same style set the regression check (`check-oracle-regression.js`) verifies
  against.
- `justfile`: new `fixture-refresh` recipe — the single fan-out entrypoint
  described above.

## What did not change

- The whole-file hash in `fixtureHash` (`scripts/oracle-snapshot.js:103`) —
  confirmed correct for the entries it protects; see "Why re-keying the cache
  wouldn't have helped" above.
- `embedded-parity-baseline.json` and `oracle-top10-baseline.json` remain two
  files — confirmed intentional; see "Root cause 3" above.
- `baselines/README.md`'s policy that the two ratcheted CI floors are
  refreshed "only in dedicated baseline PRs with a short before/after summary
  and justification" — `fixture-refresh` still requires the explicit
  `just fixture-refresh yes` form to touch either one; it does not make
  refreshing them casual.
- No fixture file, snapshot, manifest, or baseline was regenerated by this
  change itself — this is tooling and documentation only.

## Known pre-existing gap surfaced while verifying this change

Running `node scripts/oracle-snapshot.js --all` end-to-end (to verify
`fixture-refresh`) found two styles that currently fail to render and were
never regenerated after the snapshot format moved to `version: 2`:
`etudes-chinoises.csl` and `organon.csl`, both failing with `Cannot read
properties of undefined (reading 'strings')` — a locale-loading error,
unrelated to this change. The full snapshot corpus also still contains a
large number of stale `version: 1` entries predating that bump. Neither is
addressed here (reproducing and fixing the locale bug is a separate task; a
full corpus regeneration is exactly the kind of large, reviewed change this
audit's fan-out tooling exists to make deliberate, not something to fold
into a tooling PR). Filed as follow-up work, not fixed in place: bean
`csl26-u87d` for the locale-loading error, `csl26-y36e` (blocked by
`csl26-u87d`) for the version-1 snapshot regeneration.

## Adjacent, not absorbed

Bean `csl26-fvpo` ("oracle-snapshot: include locale files in staleness hash")
is a related but distinct gap in the same cache key — snapshots are keyed on
`fixture_hash + csl_hash` only, so editing `scripts/locales-*.xml` silently
leaves stale snapshots that citeproc-js would render differently. Left
tracked separately rather than folded in here, since it's a different input
to the same key, not a fan-out or granularity question.
