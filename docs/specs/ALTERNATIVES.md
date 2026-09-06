# Alternatives Specification

**Status:** Draft
**Version:** 1.1
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `docs/specs/RENDER_WHEN_CONTRACT.md`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`, `csl26-h8ja`,
`csl26-x79y`, `csl26-zs9y`, `csl26-8b4a`

## Purpose

`alternatives:` is an ordered list of template components on a template
position. The first one that renders non-empty output wins; the rest are not
evaluated. It replaces the "fallback" half of `render-when` usage — cases
where a style tries one rendering, and failing that, another — with a
primitive that has no predicate at all: no field-presence test, no boolean
expression, just an ordered set of things to try.

This generalizes a pattern the schema already has three narrower versions of:
`Substitute.candidates` (contributor role fallback), `DateFallbackCandidate`
(issued-date fallback), and `ArticleJournalNoPageFallback` (a single
hardcoded DOI fallback for one reference type). See the companion decision
record for the evidence that motivated extracting the general case instead of
adding a fourth narrow one.

## Scope

In scope:

- the wire contract for a new `TemplateComponent::Alternatives` variant;
- evaluation order and "did this render" semantics, reusing the engine's
  existing per-component rendering dispatch (leaf and group alike) rather
  than defining a second success rule;
- tracker semantics for discarded candidates;
- interaction with `Substitute` and `DateFallbackCandidate` (naming, not
  merging);
- validation rules.

Out of scope:

- predicates, field-presence tests, or any condition on source data —
  `render-when` continues to own that, frozen at its current vocabulary (see
  `RENDER_WHEN_CONTRACT.md` v1.2);
- the structural work-form routing problem (`volume-or-issue`,
  `part-number-numeric` / `part-number-non-numeric` editor/container
  routing) — that is a separate, not-yet-designed primitive; see the decision
  record's "Work-form routing" section;
- migrating `Substitute` or `DateFallbackCandidate` onto this primitive —
  both carry role- or date-specific semantics (substitution slot formatting,
  message-vs-date-vs-variable candidate kinds) that a bare component list
  does not model. They stay as they are.
- `citum-migrate` emission — a first migrate target is plausible (see
  "Future migration target" below) but is not part of this spec.

## Design

### Wire contract

```yaml
- alternatives:
  - variable: volume-title
    emph: true
  - title: primary
```

Reads as: *render the volume title; if that produces nothing, render the
primary title instead.* Compare to today's `render-when` encoding of the same
rule, which needs two separate groups and a repeated field name:

```yaml
- group:
  - variable: volume-title
    emph: true
  render-when:
    field-present: volume-title
- group:
  - title: primary
  render-when:
    field-absent: volume-title
```

`alternatives:` is valid as a top-level template item, inside a `group`, and
inside `type-variants`/`bibliography.template` entries — anywhere a
`TemplateComponent` is valid today. Each entry in the list is itself a full
`TemplateComponent` (a leaf variable, a group, a message, another
`alternatives:` block, etc.) with its own `Rendering` (prefix/suffix/wrap/etc).

### Evaluation

1. Evaluate list entries in order.
2. An entry "renders" if it produces non-empty output under the **existing,
   unchanged** per-component rendering rule for whatever kind of component it
   is: a leaf component (variable, message, number, etc.) succeeds when its
   rendered value is non-empty, exactly as today; a `group:` entry succeeds
   under the group's own existing rule (which already discards a group whose
   only content is terms/literals with no real data behind them — see
   `crates/citum-engine/src/values/list.rs:70` for that group-specific
   heuristic). `alternatives:` does not define a second, competing notion of
   "did this render" — it reuses whichever rule already governs the entry's
   own component kind.

   **This has one direct authoring consequence:** a term-only or
   message-only fallback (e.g. a locale message like "place unknown") must be
   written as a **bare leaf component**, not wrapped in `group:`. A leaf
   message renders whenever it produces text, full stop; a `group:`
   containing only terms is suppressed by the rule above regardless of
   `alternatives:`, because that suppression is the group's own long-standing
   behavior for deciding whether it carries real content. Compare:

   ```yaml
   # Correct: bare leaf, renders "place unknown" whenever reached.
   - alternatives:
     - variable: publisher-place
     - message: term.place-unknown

   # Wrong: the second candidate is a term-only group, so it is suppressed
   # by list.rs's existing rule and never renders even when reached.
   - alternatives:
     - variable: publisher-place
     - group:
       - message: term.place-unknown
   ```

3. The first entry that renders wins. Its output, prefix, and suffix are used
   as-is. No further entries are evaluated (no side effects to worry about
   for the winner, but this also bounds cost: a long alternatives list does
   not evaluate every branch on every reference).
4. **Discarded candidates must not leave side effects.** Rendering a
   candidate — trying it and finding it empty, or trying it and discarding it
   because an earlier candidate already won — must not mark any variable as
   "already rendered," consume any contributor role, or otherwise mutate
   shared rendering state that the winning candidate or later template
   components still depend on. Each candidate is evaluated against a cloned
   copy of that state; only the winning candidate's mutations are kept. (The
   pre-existing `group:` rendering path does not currently guarantee this for
   its own children — see Implementation Notes — but `alternatives:` must not
   inherit that gap.)
5. If no entry renders, the `alternatives:` component itself renders nothing
   — same as a `group` with no content, so it is invisible to surrounding
   delimiter/join logic.

Unlike `render-when`, there is no notion of "field absent" to check up front:
the mechanism is purely "try, then try the next," discovered from actual
output rather than declared in advance. This is why it cannot express the
B-shape (structural policy) uses found in the disposition audit — those
require knowing *which* branch to pick before rendering anything, based on a
property that never appears in the rendered output at all.

The `term.place-unknown` message used above is illustrative, not existing
vocabulary: checked `crates/citum-schema-style/src/locale/message_ids.rs`,
no `place-unknown` term is defined today. Using this example in an actual
style requires the same locale-authoring step
`docs/specs/MEDIUM_DESIGNATOR.md` needs for its own access-phrase term —
adding a new term (or a style-scoped `messages:` override) via
`docs/guides/AUTHORING_LOCALES.md`'s existing mechanism — not part of this
spec's acceptance criteria.

### Validation

Style validation rejects:

- `alternatives: []`, an empty list (no-op, same rule as
  `render-when: {}`);
- `alternatives:` with exactly one entry (write the component directly
  instead — a single-entry list has no fallback behavior to express).

### Relationship to existing candidate-list types

`alternatives:` is a template-level primitive. `Substitute.candidates` and
`DateFallbackCandidate` are options-level and carry semantics specific to
their domain (a substituted editor inherits the author slot's name
formatting and sort position; a date fallback distinguishes date-shaped,
message-shaped, and variable-shaped candidates). `alternatives:` does not
replace them — it is the shape underneath all three, available directly in
templates for cases that are not contributor substitution or date fallback.

**Correction (2026-09-06):** an earlier draft of this section proposed
`alternatives:` as a replacement for `ArticleJournalNoPageFallback::Doi`
(`options/bibliography.rs:136`), citing NLM's DOI rule as a worked example.
That was wrong and has been removed. Reading NLM's shipped `access` macro
precisely (`styles-legacy/taylor-and-francis-national-library-of-medicine.csl:72-88`)
shows the rule is `if type="article-journal"` **and**
`if match="none" variable="page volume"` — a type-gated, field-presence
test, not "render the normal detail block and fall back to DOI if it happens
to be empty." The normal detail block includes `date: issued`, which is
present on nearly every reference, so an `alternatives:` encoding using it
as the first candidate would never fall through to DOI at all. This case
does not fit `alternatives:`'s output-based, no-predicate model — it needs a
declared condition evaluated *before* anything renders, which is exactly
what `ArticleJournalNoPageFallback` already is. The correct fix is
extending that existing, narrowly-scoped option to also test volume absence
(tracked separately; see `docs/specs/MEDIUM_DESIGNATOR.md`'s cross-reference
and its companion bean), not routing it through this primitive.

### Rejected: an options-level construct instead of a template component

The three precedents this spec generalizes are all options-level
(`Substitute`, `DateFallbackCandidate`, `ArticleJournalNoPageFallback`), so
the natural question is why this isn't a fourth one — say,
`bibliography.options.fallbacks: { volume-title: [...] }` — instead of a new
`TemplateComponent` variant.

The precedents work as named options because each anchors to exactly **one
semantic slot that exists in every reference regardless of style**: *the*
contributor position, *the* issued date, *the* article-journal detail block.
An option can afford to skip saying "where in the template" because there is
only one place it could mean.

The 49 A-shape uses this spec targets have no such single slot. `volume-title`
vs `title: primary` only matters inside Chicago's multivolume-chapter
shape; `collection-title`, `recipient`, `archive-location`, `original-title`,
and `publisher` each guard a different, unrelated position, found in a
different type-variant, with different surrounding prefix/suffix/emphasis
that belongs to that exact spot in that exact template. An options table
keyed by field name would need to smuggle back in everything a template
position already carries (which type-variant, what delimiter joins it to its
neighbors, what emphasis this specific style wants) — at which point it is a
template fragment wearing an options key, not a genuine cross-cutting policy.

The deciding cost, not just the modeling awkwardness: treating each field as
its own named option recreates exactly the one-off proliferation
`RENDER_WHEN_CONTRACT.md`'s extension criteria were trying to close off — a
new Rust type for every field a style author discovers needs a fallback,
forever. A template-level primitive is the one that lets a style express a
new fallback with no schema change at all.

### Future migration target

`RENDER_WHEN_CONTRACT.md` states that `citum-migrate` does not emit
`render-when`. `alternatives:` is a plausible target for future migrate work:
a CSL `<choose><if variable="X">…rendering X…<else>…</else></choose>` where
`X` appears inside the `if` branch is exactly the A-shape pattern this spec
covers. That is future work, not part of this spec's acceptance criteria.

## Implementation Notes

Expected shape: a new `TemplateComponent::Alternatives(TemplateAlternatives)`
variant, `TemplateAlternatives { alternatives: Vec<TemplateComponent> }`,
alongside the existing `Group`, `Variable`, `Message`, etc. variants in
`crates/citum-schema-style/src/template.rs:625`.

**Correction (2026-09-06):** an earlier draft of this section said
evaluation "lives in `crates/citum-engine/src/values/`, sharing
`TemplateGroup::values`." That named the wrong layer. `TemplateGroup::values`
(`crates/citum-engine/src/values/list.rs`) has no render-when handling and is
not where variable-once tracking or substitution bookkeeping happen —
grepped, zero hits. Those live on `Renderer`
(`crates/citum-engine/src/processor/rendering/mod.rs`, `TemplateComponentTracker`
at `:249`), specifically in `render_template_component_with_format` and
`render_group_component_with_format`
(`crates/citum-engine/src/processor/rendering/grouped/core.rs`) — the actual
per-template-position dispatch that already resolves nested `render-when`,
variable-once skipping, and substitution metadata correctly. `Renderer` is
confirmed shared by both citation and bibliography rendering
(`Renderer::new` called from `processor/bibliography/mod.rs:162,268` and
`processor/citation.rs:374`), so an `alternatives:` arm added to this same
dispatch — rather than to `values::` — covers both template kinds from one
implementation, and gets nested-group correctness for free by recursing
through the same per-component call these functions already make.

The tracker-cloning rule in Evaluation step 4 needs one implementation
caveat: `render_group_component_with_format` currently merges a group's
tracker mutations into its parent unconditionally, *before* checking whether
the group produced any output (`tracker.merge_from(group_tracker)` runs
above the subsequent `values?` empty-check). That is a pre-existing
behavior of plain `group:` rendering, out of scope for this spec to change
(see the follow-up bean cross-linked in Acceptance Criteria) — but
`alternatives:`'s own evaluator must not copy this pattern: discard a losing
candidate's tracker clone entirely, merge only the winner's.

## Acceptance Criteria

- [ ] Schema: `TemplateComponent::Alternatives` variant, validated (empty and
      single-entry rejected).
- [ ] Engine: evaluation order, first-non-empty-wins, implemented as an arm
      on `Renderer`'s existing per-component dispatch (not a `values::`-layer
      reimplementation) — see Implementation Notes.
- [ ] Engine: candidate evaluation uses a cloned tracker per attempt; only
      the winning candidate's tracker delta is merged back.
- [ ] Behavior tests: a bare-leaf term-only candidate succeeds; a `group:` of
      only terms inside a candidate is still suppressed (existing group
      semantics, unchanged); first entry renders; first entry empty falls
      through; all entries empty renders nothing; nested `alternatives:`
      inside `group` and vice versa; nested `render-when` inside a candidate
      is honored; a losing candidate that would have consumed a
      contributor/date does not affect the winning candidate or later
      template components.
- [ ] `just schema-gen` run, schema docs updated.
- [ ] At least one embedded style migrated as a worked example — Chicago's
      volume-title fallback (`chicago-author-date-18th.yaml:416-425`) is the
      verified-shape candidate; the NLM DOI case is explicitly not a target
      for this spec (see "Rejected: an options-level construct" section's
      correction) — with a `report-core.js` diff showing 0 regressions.
- [ ] Status promoted to Active in the implementation commit.

## Changelog

- v1.1 (2026-09-06): Corrected per a Codex adversarial review and follow-up
  verification: fixed the evaluation rule (leaf vs. group "did this render"
  semantics were conflated), fixed Implementation Notes to name the real
  integration point (`Renderer`/`grouped/core.rs`, not `values/`), added the
  tracker clone-and-discard rule, dropped the NLM-DOI worked example (routed
  to extending `ArticleJournalNoPageFallback` instead), noted
  `term.place-unknown` doesn't exist yet. See `csl26-8b4a`.
- v1.0 (2026-09-06): Initial draft.
