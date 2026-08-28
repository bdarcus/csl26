# Range/Collapse Model Specification

**Status:** Active
**Version:** 1.1
**Date:** 2026-08-28
**Supersedes:** (none — no prior spec covers this ground)
**Related:** `docs/architecture/audits/2026-08-26_RANGE_COLLAPSE_CONFIG.md`
(the evidence behind this spec — read that first), `docs/specs/EDTF_DATE_RANGE_FORMATTING.md`
(dates keep independent policy from pages, already shipped — see Decision 1),
`docs/adjudication/DIVERGENCE_REGISTER.md` (`div-017`, `div-009` — the
existing mechanism for intentional citeproc-js divergence, used in Decision
2 and Acceptance Criteria), `csl26-awlo` (epic), `csl26-cl4q` (open-ended
ranges, e.g. `"12ff"` — must fit this model), `csl26-rgys` (9 styles missing
citation-number abbreviation — implementation should wait for this spec),
`docs/policies/LOCALIZATION_INTEGRITY.md`

## Purpose

Citum abbreviates number ranges (`112–118` → `112–18`) correctly in one
place — the bibliography page count — and inconsistently everywhere else a
range can appear: citation locators, publication-date ranges, citation-number
lists, same-author suffix runs, and compound sub-entries.

**One range-rendering model: contextual inheritance, explicit overrides, and
semantic defaults.** Two separate questions this spec keeps apart:

- **Mechanism** — every range-producing surface uses the same
  configuration-resolution system, with an explicit override always
  available and nothing hardcoded at the call site. Decided: yes,
  unconditionally (Decision 1).
- **Default value inheritance** — what a surface gets when it doesn't
  override. Not automatically the same value everywhere: a citation number
  and a page number are both "numbers that can range," but they aren't the
  same *kind* of thing. Resolved by the semantic-class layer in Decision 1
  and the locator default in Decision 2.

A rendered range also has two independent formatting properties that this
spec keeps distinct rather than folding into one knob: **endpoint
abbreviation** (`112–118` → `112–18`) and the **delimiter** character
joining the two numbers (`–` vs `-` vs a locale-specific form). They often
travel together but are resolved through separate chains (appendix).

## Scope

Configuration resolution for closed numeric ranges — endpoint abbreviation
and delimiter selection — for page variables, citation locators,
citation-number sequences, compound sub-entry sequences, date ranges, and
same-author suffix sequences.

Out of scope: whether adjacent values collapse into a range at all
(`citation.collapse` and friends — untouched), open-ended ranges like
`"12ff"` (`csl26-cl4q`), and the distinct parsing/formatting logic date and
letter endpoints require beyond what they can share with numeric endpoints.

**Nothing here removes an override a style author already has.** Every
example below is the actual YAML. Locators, citation-number lists,
same-author suffixes, and compound entries currently have *no* field to
set their own abbreviation behavior — this spec gives them the same
override mechanism `mhra-notes.yaml` already uses for locators today
(`options.locators.range-format: expanded`).

## Decisions

| # | Decision | Resolution |
|---|---|---|
| 1 | One configuration-resolution mechanism for every range, organized by semantic class; generic field names instead of page-specific ones | `range-format` / `range-delimiter` replace `page-range-format` / `page-range-delimiter`; `identifier-range-delimiter` configures citation-number, compound-sub-label, and suffix ranges; defaults are set per semantic class, not flatly |
| 2 | Does the style-wide default apply to every locator kind, or only pages? | **All kinds**, by design. Per-kind override (already in the schema) is how a style opts a specific kind back out |
| 3 | Citation-scoped and bibliography-scoped range settings — fix or remove? | **Remove.** Zero of 17 styles that set `page-range-format` use a scoped copy distinct from the style-level value |

### 1. One mechanism, generic naming, semantic-class defaults

Today only the bibliography page count reads the style's declared
abbreviation rule; locators, citation-number lists, same-author suffixes,
and compound entries either have no field at all or a disconnected one.
Fix, with no change required to existing style YAML:

```yaml
# chicago-author-date-18th.yaml, today — bibliography abbreviates (1–13),
# the in-text locator doesn't, because it isn't wired to anything
options:
  page-range-format: chicago16
  locators: note
```

```yaml
# same file, after this spec — the locator now inherits chicago16 too;
# no YAML change needed to fix the bug
options:
  page-range-format: chicago16   # → range-format, see naming below
  locators: note
```

An author who wants one surface to diverge keeps the exact mechanism that
exists today — `mhra-notes.yaml` already writes this:

```yaml
options:
  locators:
    range-format: expanded    # explicit override, wins over any inherited default
```

**Naming.** `range-format` / `range-delimiter` replace `page-range-format` /
`page-range-delimiter` at the style-options level. Not a new convention —
`options.dates.range-format` (shipped) already uses the generic name, scoped
under `dates:`. This makes the schema consistent with what's already in
production rather than introducing a second pattern. `PageRangeFormat`
becomes a generically-named Rust type (appendix); `chicago16` is retained as
an accepted value spelling.

**Semantic classes** — a surface's default comes from its class, not one
flat value:

- **Numeric textual locators** — pages, chapters, paragraphs, notes,
  sections. Inherit the style-wide default (Decision 2: all kinds).
- **Dates/years** — independent policy, per `EDTF_DATE_RANGE_FORMATTING.md`
  (shipped). Shares the value vocabulary and algorithm
  (`values/date.rs:530` already calls `format_chicago_range_end`, the same
  function pages use), not the configuration field.
- **Identifier sequences** — citation numbers, compound-entry numbers.
  List positions, not prose numerals; no inherited bearing from the page/
  date elision rule. Defaults independently.
- **Suffix sequences** — `2020a–c`. Letter endpoints; shares only the
  delimiter/collapsing mechanism, not an abbreviation policy.

Endpoint abbreviation and delimiter resolution use separate chains. Numeric
textual locators resolve `kind override -> locator override -> style-wide
default -> Expanded`. Citation numbers resolve their own override and then
default to `Expanded`. Dates keep their independent configuration. Page and
locator delimiters resolve `range-delimiter -> locale page-range-delimiter`;
identifier and suffix delimiters resolve `identifier-range-delimiter -> en
dash`.

### 2. Locator default: all kinds

The style-wide default applies to every locator kind. The per-kind override
is how a style opts a *specific* kind back out — not how it opts one in:

```yaml
options:
  range-format: chicago16
  locators:
    kinds:
      chapter:
        range-format: expanded   # opts this one kind out of the style default
```

If this moves any style's oracle-verified output away from citeproc-js
(citeproc-js only abbreviates page locators), that's registered as an
intentional divergence in `docs/adjudication/DIVERGENCE_REGISTER.md`, the
project's existing mechanism for exactly this (`div-017`, `div-009`) — not
silently absorbed as a parity change.

### 3. Citation/bibliography-scoped range settings: removed

```yaml
# expressible today, unused in the corpus
citation:
  options:
    page-range-format: minimal
bibliography:
  options:
    page-range-format: chicago16
```

Zero of the 17 styles that set `page-range-format` use a scope-specific
value. Formatting policy attaches to the semantic type of a range, not to
the rendering region it appears in — a page count is a page count whether
it's in a citation or the bibliography. Remove
`CitationOptions`/`BibliographyOptions`'s copy of the field. This also
closes the "add a scoped delimiter" question — there's no scoped format
left for one to pair with.

## Acceptance Criteria

- [x] `range-format`/`range-delimiter` resolve for every surface in scope,
      with an explicit override available at each
- [x] `chi-chapter-locator` renders `112–18`, matching citeproc-js
- [x] `mhra-notes`' explicit `locators.range-format: expanded` still wins —
      explicit overrides beat every inherited default
- [x] `CitationOptions`/`BibliographyOptions` no longer carry a
      `page-range-format`/`range-format` field
- [x] `docs/guides/style-authoring/` documents the resulting model
- [x] Unintentional oracle-verified output changes are regressions and block
      the implementation PR. Any style where the all-kinds locator default
      (Decision 2) diverges from citeproc-js gets a `div-0NN` entry in
      `DIVERGENCE_REGISTER.md`, following the `div-017`/`div-009` pattern

---

## Appendix: implementation notes (for engineers)

Full surface inventory and evidence: see the audit doc linked above.

- **Naming.** `PageRangeFormat` → a generic type name (e.g.
  `NumberRangeFormat`); `chicago16` retained as an accepted spelling.
- **Dates stay a separate field.** `DateRangeFormat` is not merged into
  `range-format` — `dates.range-format` remains its own key, per the shipped
  EDTF spec. What unifies: value vocabulary and the underlying algorithm.
- **Delimiter chains.** Page variables and locators use
  `options.range-delimiter`, then the locale's
  `grammar-options.page-range-delimiter`. Citation-number, compound
  sub-label, and same-author suffix ranges use
  `options.identifier-range-delimiter`, then an en dash. The two defaults
  stay independent because AMA and ACS use a hyphen for page ranges while
  retaining an en dash for identifier ranges.
- **Locator kind gating**: `locators.kinds.<kind>.range-format` →
  `locators.range-format` → `options.range-format` → `Expanded`, applied to
  every kind (Decision 2). `LocatorConfig::range_format` is
  `Option<RangeFormat>`; the three `LocatorPreset::config()` arms set `None`.
  **Rejected:** resolving the style default at preset-injection time
  instead — the four styles with an explicit `locators:` map
  (`american-medical-association`, `oscola`, `oscola-no-ibid`,
  `mhra-notes`) never go through `LocatorPreset::config()` and would still
  miss the fallback.
- **Scoped-field removal**: delete `page_range_format` from
  `CitationOptions`/`BibliographyOptions` (`options/mod.rs:292`, `:415`) and
  their `to_config()` conversions (`:855-856`, `:989-990`).
- **Also found, unrelated to any decision above**: `pattern.page-range`
  (an MF2 locale message in 5 locale files) is dead — nothing reads it
  (audit incoherence 6). Delete it during implementation.
- **Rejected (general)**: resolving format/delimiter independently at each
  call site. Central resolution avoids duplicated precedence logic and
  hardcoded literals.
- **Architecture test.** The implementation should express all of the
  following through configuration and one shared resolution API, with no
  duplicated precedence logic across `number.rs`, `locator.rs`,
  `collapse.rs`, `year_suffix.rs`, and `date.rs`:

  ```yaml
  options:
    range-format: chicago
    identifier-range-delimiter: "–"
    locators:
      kinds:
        chapter:
          range-format: expanded   # opt one kind out
    citation-numbers:
      range-format: expanded       # identifier sequences stay their own default
    dates:
      range-format: expanded       # independent policy, per EDTF spec
  ```

- `just schema-gen` required in the implementation commit.

## Changelog

- v1.1 (2026-08-28): Activated after implementation. Added the independent
  `identifier-range-delimiter` resolution chain and marked the acceptance
  criteria complete.
- v1.0 (2026-08-28): Resolved for implementation — generic `range-format`/
  `range-delimiter` naming; locator default applies to all kinds with
  per-kind opt-out; citation/bibliography-scoped range settings removed
  (0/17 corpus usage). See PR #1235 for drafting history.
