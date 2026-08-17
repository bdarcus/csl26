---
name: test-coverage
description: >
  Domain knowledge for test coverage in citum-core. Use this skill whenever
  you're implementing a feature, fixing a bug, writing tests, or claiming
  something is "tested" in citum-core. Trigger on any task involving: adding
  fixture data, writing Rust tests, running the oracle, validating style
  behavior, or checking whether a code path is exercised. This skill tells
  you what reference shapes are required to test each behavior, and provides
  a pre/post checklist so nothing slips through.
---

# Citum Test Coverage

## The core problem

500+ tests pass, but behaviors still slip through. The reason is almost
always one of two things:

1. The fixture data doesn't have the right *shape* to exercise the code path
   (e.g., testing "no-date" rendering but every reference has an `issued` field)
2. Citation scenarios only reference `article-journal` / `book` items, so
   citation rendering of `chapter`, `report`, `thesis`, etc. is untested

Use the audit script and this checklist before declaring something tested.

## Anti-overfitting rule

Passing tests are not enough. A new or changed test must make an independent
claim about behavior:

- Prefer expected values from a fixture, citeproc oracle output, a spec, a
  registered divergence, or a literal behavior contract.
- Do not derive `expected` from `actual`, `result`, `rendered`, or other output
  produced by the code under test.
- For behavior fixes, confirm the test would fail on the old behavior when
  practical. If that is not practical, say why in the PR or final report.
- Avoid changing exact assertions into `contains` assertions unless the behavior
  is intentionally partial, order-insensitive, or format-agnostic.
- If a fixture changes, state which missing shape it adds and which scenario
  exercises that shape.

The full shared bar — including when a test counts as *redundant* and should be
deleted rather than kept — lives in `docs/guides/CODING_STANDARDS.md`
§ "What makes a test worth keeping". Per-spec soundness state is tracked in
`docs/architecture/TEST_SOUNDNESS_STATUS.md`.

## Test Style

Before writing a test, pick the right style (full rule in
`docs/guides/CODING_STANDARDS.md` § "Test Style"):

| Scenario | Location | Style |
|----------|----------|-------|
| Single-function / pure logic | Inline `#[cfg(test)]` | Plain `#[test]` |
| Single-scenario integration | `tests/` | Plain `#[test]` |
| Parameterised cross-module behavior | `tests/` | `#[rstest]` + `given_…_when_…_then_…` |

BDD naming (`given_…_when_…_then_…`) is only for `#[rstest]` integration
tests with 2+ parameterised cases. Do not apply it to unit tests or
single-scenario integration tests.

## Quick audit

```bash
python scripts/audit-coverage.py          # text report
python scripts/audit-coverage.py --json   # machine-readable
python3 scripts/audit-rust-review-smells.py --changed
```

## Pre-test checklist

Before implementing or claiming a feature is tested, answer these questions:

- [ ] What reference *types* does this feature affect?
- [ ] For each type: does the fixture have an item of that type?
- [ ] What *field shapes* trigger the code path? (see matrix below)
- [ ] Does the fixture have an item with those fields populated?
- [ ] If testing citation rendering: does `citations-expanded.json` reference
      an item of the relevant type?
- [ ] What independent source defines the expected result: fixture, oracle,
      spec, divergence register, or literal behavior contract?
- [ ] Run `cargo nextest run` — do the tests actually exercise the path?

## Post-test checklist

After writing tests:

- [ ] Would the test fail against the old behavior, or is the exception
      documented?
- [ ] Are expected values independent of the actual output under test?
- [ ] Did you add fixture items if shapes were missing?
- [ ] Did you add citation scenarios if types were missing from
      `citations-expanded.json`?
- [ ] Did `python3 scripts/audit-rust-review-smells.py --changed` produce only
      reviewed advisory findings?
- [ ] Does `cargo nextest run` pass cleanly?
- [ ] If the feature touches oracle-level behavior: run
      `./scripts/workflow-test.sh styles-legacy/apa.csl` to sanity-check

---

## Feature → Required Reference Shapes

This is the core domain knowledge. For each feature, here are the fixture
shapes you *must* have to test it properly.

### Date rendering

| Shape needed | How to create it | Example item |
|---|---|---|
| Year-only | `"issued": {"date-parts": [[2020]]}` | ITEM-1, ITEM-2 |
| Year + month | `"issued": {"date-parts": [[2023, 6]]}` | ITEM-17 |
| Full date (Y-M-D) | `"issued": {"date-parts": [[1964, 7, 2]]}` | ITEM-16 |
| No date ("n.d.") | Omit `issued` entirely | **ITEM-33** |
| Accessed date | Add `"accessed": {"date-parts": [[...]]}` | ITEM-13 |

> **Watch out:** "n.d." rendering is the most commonly missed. If you're
> touching date logic, always verify ITEM-33 (no `issued`) is in your test
> path.

### Contributor rendering

| Shape needed | How to create it | Example item |
|---|---|---|
| Single author | One entry in `author` array | ITEM-1 |
| Two authors | Two entries | ITEM-6, ITEM-9 |
| Three authors | Three entries | ITEM-3 |
| 6+ authors (et-al) | Six or more | ITEM-7 |
| Corporate / institutional | `{"literal": "Org Name"}` | ITEM-5 |
| No author (anonymous) | Omit `author` | ITEM-15 |
| Editor only (no author) | `editor` array, no `author` | ITEM-14 |
| Author + editor | Both arrays populated | ITEM-4, ITEM-27 |
| Translator | `translator` array | ITEM-25 |

> **Watch out:** "editor-only" (`chapter` items where the book has editors
> but the chapter has an author) is the trickiest. ITEM-4 has both
> `author` and `editor` — needed for chapter-in-edited-volume rendering.

### Title rendering

| Shape needed | Example item |
|---|---|
| Title only (no container) | ITEM-2 (book) |
| Title + journal container | ITEM-1 (article-journal) |
| Title + book container (chapter) | ITEM-4 (chapter) |
| Non-Latin title / language field | ITEM-25 (language: en), ITEM-26 (language: la) |

### Locator / volume / page

| Shape needed | Example item |
|---|---|
| Volume + issue + page | ITEM-1 |
| Volume + page (no issue) | ITEM-3, ITEM-7 |
| Page range with hyphen | ITEM-3 (`"436-444"`) |
| Chapter page range | ITEM-4 |
| Edition | ITEM-6 |
| Report genre | ITEM-27 |
| Thesis genre | ITEM-11 |
| Patent number | ITEM-21 |

### Identifiers

| Shape needed | Example item |
|---|---|
| DOI | ITEM-1, ITEM-3 |
| URL (with accessed) | ITEM-13, ITEM-19 |
| URL (no accessed) | ITEM-24 |

---

## Reference type coverage

Current fixture coverage: **22 / 33 types** (66.7%). Run the audit script to
see the current state. Types below are in every fixture; note which ones
have citation scenarios too.

### Types with citation scenarios in `citations-expanded.json`

These types are tested in both citation and bibliography rendering:

- `article-journal` — ITEM-1, ITEM-3, ITEM-7, ITEM-29–32
- `book` — ITEM-2, ITEM-33 (no-date)
- `paper-conference` — (referenced via ITEM-7 disambiguation scenarios)
- `chapter` — ITEM-4 (`chapter-single` scenario)
- `report` — ITEM-5 (`report-single` scenario)
- `thesis` — ITEM-11 (`thesis-single` scenario)
- `webpage` — ITEM-13 (`webpage-single` scenario)

### Types with bibliography coverage only (no citation scenarios)

These are tested in bibliography rendering but NOT in citation formatting:

`article-magazine`, `article-newspaper`, `broadcast`, `dataset`,
`entry-encyclopedia`, `interview`, `legal_case`, `legislation`, `manuscript`,
`motion_picture`, `paper-conference`, `patent`, `personal_communication`,
`software`, `standard`, `treaty`

If you're adding behavior that affects how these types render **in citations**,
add a citation scenario to `citations-expanded.json` referencing the item.

### Types with no fixture at all

`bill`, `entry-dictionary`, `entry-legal`, `graphic`, `hearing`, `pamphlet`,
`post`, `post-weblog`, `regulation`, `song`, `speech`

If a bug report involves one of these types, add a fixture item first.

---

## Citation scenario coverage (`citations-expanded.json`)

| Scenario | What it tests |
|---|---|
| `single-item` | Basic single citation |
| `multi-item` | Cite cluster with two items |
| `with-locator` | Page locator |
| `multi-item-with-locators` | Multiple locators |
| `suppress-author` | Suppress author in narrative citation |
| `suppress-author-with-locator` | Combined |
| `locator-section-with-suffix` | Section locator + suffix text |
| `multi-item-with-prefix` | Prefix on item in cluster |
| `single-with-prefix-and-suffix` | Prefix + suffix |
| `et-al-single-long-list` | 4-author list triggers et-al |
| `et-al-with-locator` | Et-al + locator |
| `disambiguate-add-names-et-al` | Disambiguation by adding names |
| `disambiguate-year-suffix` | Disambiguation by year suffix |
| `chapter-single` | Chapter type in citation |
| `report-single` | Report type in citation |
| `thesis-single` | Thesis type in citation |
| `webpage-single` | Webpage type in citation |
| `no-date-single` | Item with no `issued` field |

---

## Adding fixture data

**First decide whether this belongs in the shared default at all.** Every
entry in `tests/fixtures/references-expanded.json` renders into all ~2,845
`tests/snapshots/csl/*.json` oracle snapshots (`oracle-snapshot.js` loads the
full entry set for every legacy style), so any edit to that file is a
corpus-wide diff — hundreds of thousands of lines — no matter how small the
change looks. See
[docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md](../../../docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md).

- **Narrow bug regression** (one style, or a handful): do not touch the
  shared default. Use a native Rust test instead (no CSL-JSON round-trip —
  see "Test Style" above), or, if oracle coverage is genuinely needed, a
  fixture scoped to a `fixture_family` (`scripts/lib/style-verification.js`)
  consumed only by the smaller `report-core.js` measurement set. Either way:
  zero snapshot churn.
- **Genuine corpus-wide coverage gap** (a reference-type shape or citation
  scenario the whole legacy corpus should exercise): add to
  `references-expanded.json` or the appropriate domain fixture as below. This
  is a deliberate, reviewed, corpus-wide change — give it its own commit and
  follow with `just fixture-refresh` (`just fixture-refresh yes` if it also
  moves a ratcheted baseline floor).

When it is the latter, use sequential `ITEM-N` IDs in the expanded fixture.

After adding items:
1. Update `tests/fixtures/coverage-manifest.json` under `reference_types`
2. If the item should be citation-tested, add a scenario to
   `tests/fixtures/citations-expanded.json`
3. Re-run `python scripts/audit-coverage.py` to confirm the gap is closed
