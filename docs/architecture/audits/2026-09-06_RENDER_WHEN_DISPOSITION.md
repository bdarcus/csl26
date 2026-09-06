# `render-when` Disposition: Freeze, Add `alternatives:`, Defer Work-Form Routing

- **Date:** 2026-09-06
- **Bean:** `csl26-h8ja`
- **Question:** A wave-3 parity pass wanted three new `render-when`
  condition fields (`url`, `pages`, `publisher-place`) to fix 54 embedded
  rows. That conflicts with a standing project position — `csl26-7652`
  already rejected a locator-kind condition field on the grounds that it
  "relocates CSL's procedural `<choose>` branching into the Citum template
  layer, which this project has deliberately kept declarative." Given that,
  what should happen to `render-when` itself: extend it anyway, remove it, or
  something else — and what unblocks the parity work that motivated the
  question?

## Two examples that look identical and aren't

**Chicago's multivolume title** (`chicago-author-date-18th.yaml:416-425`):

```yaml
- group:
  - variable: volume-title
    emph: true
  render-when:
    field-present: volume-title
    field-absent: part-number-non-numeric
- group:
  - title: primary
  render-when:
    field-absent: volume-title
```

Read as a sentence: *a multivolume work is titled by its own volume title;
failing that, by the work's title.* The field being tested, `volume-title`,
is the same field the branch renders. This is a fallback — try one thing,
then another — wearing a conditional's syntax.

**Chicago's editor form for numbered volumes** (`chicago-author-date-18th.yaml:457-470`):

```yaml
- group:
  - contributor: editor
    form: verb
    name-order: given-first
  - title: parent-monograph
    emph: true
  render-when:
    field-absent: volume-or-issue
- group:
  - contributor: editor
    form: verb
    name-order: given-first
  render-when:
    field-present: volume-or-issue
```

Read as a sentence: *a serial-numbered container renders its editor
differently from an unnumbered one.* `volume-or-issue` never appears in
either branch's rendered content — it is a switch on one property of the
reference that reroutes rendering of an unrelated one. This is a structural
policy decision, and no fallback list can express it: there's nothing to fall
back *from*.

Both blocks use the identical `render-when` primitive. They are not the same
kind of thing, and that distinction is the whole finding.

## What the inventory shows

`render-when` appears 125 times across the embedded style corpus: 123 in
Chicago (`chicago-author-date-18th` 48, `chicago-notes-18th` 34,
`chicago-shortened-notes-bibliography-core` 29,
`taylor-and-francis-chicago-author-date-core` 12), 2 in
`gb-t-7714-2025-base`, and **zero in the exemplar tier** —
`citum-migrate` does not emit this feature (`RENDER_WHEN_CONTRACT.md` states
this as a scope decision, not an omission).

Every use was classified by one test: does the field the condition tests
appear as a rendered component inside the branch it guards?

| Shape | Count | Meaning |
|---|---|---|
| **A** — tested field renders inside its own `field-present` branch | 49 | fallback (try this, then that) |
| **B** — tested field does not render inside its `field-present` branch | 25 | structural policy gate |
| **C** — `field-absent`-only branch, pairing with an A or a B | 50 | the "else" of one of the above |
| **D** — compound, two fields tested together | 1 | (`part-number` present, `volume-title` absent) |

The decisive number: grouping by field rather than by use, the six
highest-volume fields — `volume-title` (31 uses), `volume-or-issue` (24),
`part-number-non-numeric` (16), `genre` (16), `title` (12),
`part-number-numeric` (9) — **108 of the 125 uses** — are each a *mix* of A
and B shapes. `genre`, for instance, guards a genre-then-episode-number
fallback in one place and an unrelated title/date arrangement in another.
No field can be retired by one replacement; "swap `render-when` for a single
better primitive" is not an option this data supports.

## Recommendation: freeze the vocabulary, add one new primitive, defer the rest

Extending `render-when` is rejected on the same grounds as `csl26-7652`: it
adds more procedural test-then-branch surface to the template layer, which
DESIGN_PRINCIPLES §4 states plainly ("templates replace procedural
conditionals... generic spines first"). The three fields wave 3 wanted
(`url`, `pages`, `publisher-place`) would all have been used to build A-shape
fallbacks — exactly the case that has a declarative answer already.

Removing `render-when` outright is rejected for now: the 25 B-shape uses have
no declarative replacement today, and inventing one is a real design problem
(see below), not a mechanical migration.

The recommended path splits the two shapes and ships only the one that's
ready.

### `alternatives:` — an ordered candidate list (ready to spec)

The A shapes are all instances of one existing pattern in the codebase,
generalized: an ordered list of candidate renderings, first non-empty one
wins, no predicate over source data at all. Three domain-specific versions
of exactly this already exist and are accepted project idiom:

- `Substitute.candidates: [editor, translator]` with
  `otherwise: {message: term.anonymous}` (`options/substitute.rs:101`)
- `DateFallbackCandidate::{Date, Message, Variable}` (`options/date_fallback.rs:22`)
- `ArticleJournalNoPageFallback::Doi` (`options/bibliography.rs:136`) — whose
  own spec text concedes it is "a small external bibliography parameter,"
  narrower than the general case

`alternatives:` is the general primitive those three keep independently
reinventing. It's declarative in the sense the project cares about: it names
an ordered set of *things to try*, not a *test to evaluate*.

One correction to avoid overclaiming in the spec: `Substitute` carries
role-slot semantics — a substituted editor inherits the author's name
formatting, label, and sort position — that a bare candidate list doesn't
have. `alternatives:` is the shape underneath all three, not a drop-in
replacement for `Substitute` itself.

**This also fixes an open, unrelated bug by construction.** `csl26-x79y`
found that `render-when: field-present: author` reads the raw author field,
so it silently misfires on any reference where an editor has been
role-substituted into the author slot — `RENDER_WHEN_CONTRACT.md` states as
policy that conditions "do not inspect substitution results." An
`alternatives:` list tests *output*, not raw presence, so it has no
equivalent blind spot.

Verified before recommending it, and corrected after a Codex adversarial
review of the resulting spec found the first pass incomplete: group
suppression in the engine is already CSL-correct — `values/list.rs:70`
treats a `group:` as empty when it produced no non-term content, so a
branch like `[message: pattern.chicago-of, number: part-number]` already
goes empty on its own when `part-number` is absent. That much held up. What
the first pass missed is that this rule lives entirely inside
`TemplateGroup::values` and has no bearing on render-when, variable-once
tracking, or substitution bookkeeping — those live on `Renderer`
(`crates/citum-engine/src/processor/rendering/mod.rs` and `grouped/core.rs`),
a different layer the review draft never named. `alternatives:` still needs
no *new* "did this render" concept, but it does need to be implemented
against the right existing one: `Renderer`'s per-component dispatch, not
`values::list.rs` in isolation. See `docs/specs/ALTERNATIVES.md`'s
Implementation Notes and `csl26-8b4a` for the corrected account.

It also directly answers two of wave 3's three parity needs (see the
companion `docs/specs/MEDIUM_DESIGNATOR.md` for the third, which is a marker,
not a fallback, and doesn't fit here):

- `[place unknown]` (7 rows): `alternatives: [{variable: publisher-place},
  {message: term.place-unknown}]`.
- NLM's DOI-when-no-page-or-volume rule (11 rows): generalizes
  `ArticleJournalNoPageFallback` into "render the detail block; if it would be
  empty, render DOI instead" rather than adding a second narrow one-off.

And it opens a migration path `RENDER_WHEN_CONTRACT.md` explicitly closed:
that spec refuses to let `citum-migrate` emit `render-when` at all. A CSL
`<choose><if variable="X">…<else>…` where `X` appears in the `if` branch is
exactly A-shaped, and `alternatives:` can accept it as a migration target
where `render-when` never could.

### Work-form routing — not ready

The 25 B shapes are one coherent domain idea with no name yet in the schema:
whether a reference is a numbered volume, a titled volume, a named part, or
an unnumbered whole changes how several unrelated fields (editor form,
container title, page prefix) are arranged. That's squarely
`csl26-x61x` ("Chicago: volume, issue, and series grammar," open, high
priority) territory, and `docs/specs/INPUT_REFERENCE_CLASS_DISCRIMINATOR.md`
is the existing home for this class of distinction — but no design exists
yet for this specific case.

This is the actual blocker on removing `render-when`: until work-form routing
has a declarative shape, the 25 B-shape uses have nowhere to go.

## Recommended sequencing

1. Freeze the `render-when` field vocabulary now (`RENDER_WHEN_CONTRACT.md`
   v1.2) — existing 125 uses keep working, no new fields are added.
2. Spec and land `alternatives:`.
3. Design work-form routing under `csl26-40n4`, using the B-shape inventory
   above as the forcing-case set.
4. Migrate Chicago's 123 uses to the two new primitives per-style, then
   deprecate `render-when`.

Step 4 is explicitly **not** part of this decision's follow-on work: eight
Chicago beans are in-progress right now (`csl26-0u0f`, `adka`, `ax22`,
`j1wp`, `jxco`, `rrsb`, `s2kt`, `wtaq`), and rewriting the substrate under
them would collide with all of it. That migration belongs to `csl26-40n4`
once step 3 has a design.

## Rejected alternatives

- **Extend `render-when`'s field vocabulary** (the wave-3 trigger). Rejected:
  repeats the exact objection `csl26-7652` already raised for a different
  field; every field wave 3 needed was A-shaped and has a cleaner declarative
  answer.
- **Remove `render-when` immediately, migrate everything now.** Rejected: no
  design exists for the 25 B-shape uses; forcing a migration without one
  would mean inventing ad hoc per-style workarounds, which is worse than the
  status quo.
- **Leave `render-when` open for future extension.** Rejected: extension
  criteria in `RENDER_WHEN_CONTRACT.md` already gate on "not a stand-in for a
  distinction an option, preset, or type-variant should own instead" — every
  field this session evaluated failed that test. Freezing makes that
  judgment explicit instead of re-litigating it per proposal.

## Follow-on beans

- `csl26-h8ja` (this decision) — parent task tracking the spec/doc work.
- `csl26-zmxt` (under `csl26-40n4`): design work-form routing for the B-shape
  inventory above (volume-or-issue / part-number-numeric /
  part-number-non-numeric editor and container routing).
- `csl26-x79y`: cross-linked — resolved by construction once `alternatives:`
  ships and Chicago's author-substitution gates migrate to it.
- `csl26-x61x`: cross-linked as the eventual home for work-form routing.
- `csl26-zs9y`: root cause reclassified from "needs a render-when field" to
  "needs `alternatives:`" for the `[place unknown]` case and "needs a medium
  designator option" (see `docs/specs/MEDIUM_DESIGNATOR.md`) for the
  `[Internet]` case. The NLM-DOI case originally grouped with these is
  **not** `alternatives:`-shaped after all (see `csl26-8b4a` below) — it
  routes to `csl26-8z39` instead.
- `csl26-8b4a`: resolves a Codex adversarial review of `ALTERNATIVES.md` and
  `MEDIUM_DESIGNATOR.md` — corrected both specs' evaluation/integration
  details and this record's own "Verified before recommending it" claim
  above (see that section). Produced two further beans:
  - `csl26-2hr4`: a pre-existing tracker-merge-before-empty-check quirk in
    plain `group:` rendering, found while specifying `alternatives:`'s
    tracker semantics. Unrelated to `render-when`'s disposition; filed under
    the engine-review epic.
  - `csl26-8z39`: extends `ArticleJournalNoPageFallback` (not
    `alternatives:`) to cover NLM/CSE's DOI-if-no-page-or-volume rule.

## Evidence appendix

- `TemplateConditionField`: `crates/citum-schema-style/src/template.rs:1822`
- `render-when` validation: `crates/citum-schema-style/src/style/validation.rs:509`
- condition evaluation: `crates/citum-engine/src/values/mod.rs:76-88`
- group suppression (`None` on no non-term content):
  `crates/citum-engine/src/values/list.rs:70`
- existing candidate-list precedents: `options/substitute.rs:101`,
  `options/date_fallback.rs:22`, `options/bibliography.rs:136`
- per-field use counts (present-has-field / present-lacks-field / absent-only):

  | Field | Total | A (present, has field) | B (present, lacks field) | C (absent-only) |
  |---|---|---|---|---|
  | volume-title | 31 | 9 | 4 | 18 |
  | volume-or-issue | 24 | 4 | 6 | 14 |
  | part-number-non-numeric | 16 | 5 | 6 | 5 |
  | genre | 16 | 6 | 2 | 8 |
  | title | 12 | 6 | 1 | 5 |
  | part-number-numeric | 9 | 3 | 6 | 0 |
  | part-number | 5 | 0 | 1 | 4 |
  | collection-title | 5 | 3 | 0 | 2 |
  | author | 4 | 2 | 0 | 2 |
  | recipient | 3 | 3 | 0 | 0 |
  | original-published | 2 | 1 | 0 | 1 |
  | original-title | 2 | 1 | 0 | 1 |
  | original-publisher | 2 | 0 | 0 | 2 |
  | issued | 2 | 2 | 0 | 0 |
  | archive-location | 2 | 1 | 0 | 1 |
  | number-of-volumes | 2 | 1 | 0 | 1 |
  | doi | 1 | 0 | 0 | 1 |
  | publisher | 1 | 1 | 0 | 0 |
  | editor | 1 | 1 | 0 | 0 |
