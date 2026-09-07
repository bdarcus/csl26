# Alternatives Specification

**Status:** Draft
**Version:** 1.4
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `docs/specs/RENDER_WHEN_CONTRACT.md`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`, `csl26-h8ja`,
`csl26-x79y`, `csl26-zs9y`, `csl26-8b4a`, `csl26-2hr4`, `csl26-la9t`, `csl26-ro72`,
`csl26-57a7`, `csl26-zmxt`

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
- resource-budget accounting for the candidate list;
- an explicit v1 placement restriction (see "v1 placement restriction"
  below) and the follow-up work needed to lift it;
- interaction with `Substitute` and `DateFallbackCandidate` (naming, not
  merging).

Out of scope:

- predicates, field-presence tests, or any condition on source data —
  `render-when` continues to own that, frozen at its current vocabulary (see
  `RENDER_WHEN_CONTRACT.md` v1.2);
- the structural work-form routing problem (`volume-or-issue`,
  `part-number-numeric` / `part-number-non-numeric` editor/container
  routing) — that is a separate, not-yet-designed primitive; see the decision
  record's "Work-form routing" section. **Chicago's actual volume-title
  fallback is not a target for this spec** — see the Wire Contract
  section's "Rejected: Chicago's `volume-title` position";
- nesting `alternatives:` inside `alternatives:` — deferred to v2 alongside
  the placement restriction, not part of this spec's acceptance criteria;
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
  - variable: publisher-place
  - message: term.place-unknown
```

Reads as: *render the publisher place; if that produces nothing, render a
"place unknown" message instead.* This is T&F-CSE's actual rule
(`taylor-and-francis-council-of-science-editors-author-date.csl:77-83`:
`<if variable="publisher-place" match="none"><text value="[place
unknown]"/></if><else><text variable="publisher-place"/></else>`) — a clean,
verified two-way swap with no other condition attached to the same slot.
Compare to a `render-when` encoding of the same rule, which needs two
separate groups and a repeated field name:

```yaml
- group:
  - variable: publisher-place
  render-when:
    field-present: publisher-place
- group:
  - message: term.place-unknown
  render-when:
    field-absent: publisher-place
```

**Rejected: Chicago's `volume-title` position as a worked example.** It
looks like a two-way `alternatives: [volume-title, title: primary]` swap
but isn't: the literal YAML (`chicago-author-date-18th.yaml:416-430`) has a
**third**, interacting group — `field-present: part-number-non-numeric`
also renders `title: primary`, overriding `volume-title` even when
`volume-title` is present. `alternatives:` has no way to make a candidate
conditionally inapplicable other than by rendering it and checking for
output, so a present `volume-title` always wins, contradicting the real
rule. Chicago's volume-title position stays `render-when` and is a
`csl26-zmxt` (work-form routing) candidate, not this spec's. The
publisher-place example above was verified directly against the shipped
CSL and has no such third condition.

`alternatives:` is valid as a top-level template item and inside a `group`,
subject to the placement restriction below.

### v1 placement restriction

`alternatives:` is not valid everywhere a `TemplateComponent` is valid.

`TemplateComponent::Group` is pattern-matched in **21 files** across
`citum-engine` and `citum-schema-style` — not only the renderer. Three were
checked directly and confirmed to key on specific component *kinds* for
reasons that have nothing to do with rendering, walking recursively into
`Group` children and into `Message` argument trees wherever a message
carries nested `TemplateComponent` args:

- `crates/citum-engine/src/values/list.rs` (`is_term_based`) — has no
  `Alternatives` arm; an unmatched variant falls to its `_ => false` case,
  so a `group:` whose only data child is an `alternatives:` list is always
  treated as "has content" regardless of what the winning candidate turns
  out to be. This does not break the publisher-place worked example below
  (its winning candidate, `variable: publisher-place`, is genuinely
  content), but it means a `group:` wrapping an `alternatives:` list whose
  every candidate is term-only would not be suppressed the way an
  equivalent flat term-only `group:` is — `is_term_based` needs its own
  `Alternatives` arm (recursing the same way `Group` does) before that case
  is correct. Tracked in Acceptance Criteria.
- `crates/citum-engine/src/processor/rendering/grouped/component_predicates.rs` —
  a generic recursive `component_or_message_arg_contains` helper backs
  several structural finders (`Title`, `Date(Issued)`, `Number(Volume)`,
  `Variable(Url)`, `Variable(Doi)`, and others) used to locate "the title
  component," "the volume number," "the DOI," etc. for citation grouping
  and contributor-stripping. It recurses into *any* message's args, not a
  named subset, so a component of one of these kinds is invisible to it
  regardless of how deeply it is nested inside a message argument.
- `crates/citum-engine/src/processor/rendering/grouped/template_policy.rs` —
  filters an `article-journal` bibliography template by structurally
  locating its `Date(Issued)`/`Number(Volume)`/`Variable(Url)` components,
  and separately by walking for any `Message` whose name starts with
  `pattern.` (`template_has_pattern_message`) — the mechanism `csl26-8z39`
  extends for NLM's DOI rule.

The restriction this implies is about **candidate content, not candidate
position**: a consumer that looks for one of these kinds walks the whole
component tree it is given, so wrapping the kind inside `group:`,
`alternatives:`, or a message argument does not hide it from a positional
rule — only excluding the kind itself does.

The remaining 18 files were not individually audited. **v1 rejects any
`alternatives:` candidate that is, or recursively contains (through
`Group` children or `Message` args), any of the following:**

- a `Title` or `Contributor` component;
- a `Date(Issued)` component;
- a `Number(Volume)` component;
- a `Variable(Url)` or `Variable(Doi)` component;
- a `Message` whose name starts with `pattern.`;

and separately, regardless of content:

- nesting `alternatives:` inside another `alternatives:` (see Scope,
  deferred to v2).

This is a content restriction, checked by walking each candidate's own
subtree the same way the consumers above do — not a restriction on where
in the template `alternatives:` itself may sit. An `alternatives:` used at
the template's top level is fine as long as every candidate's subtree is
clean by this rule; one nested three levels inside a `group:` is not
exempt just because it's deeply nested.

The publisher-place worked example above satisfies this: `variable:
publisher-place` and `message: term.place-unknown` are both plain leaves,
neither is any of the listed kinds, and `term.place-unknown` is a
term-message with no nested component args, so nothing under it is hidden
from any of the three checked consumers.

Because this rule targets *kinds*, not a single position, it covers a
narrower slice of the 49 A-shape uses identified in the disposition audit:
any A-shape candidate that renders a date, volume number, URL, DOI, title,
or contributor is out of scope for v1 regardless of where it sits.
**Lifting this restriction requires a follow-up audit**
of the remaining 18 `TemplateComponent::Group`-matching files, adding an
`Alternatives` arm to every one that needs it — tracked in `csl26-57a7`.
Until that lands, treat any `alternatives:` candidate containing one of the
listed kinds as unverified, not merely unrecommended.

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
   components still depend on. (The v1 content restriction forbids
   `Contributor` and `Date(Issued)` candidates, so variable-once tracking on
   a plain `variable:` is the only reachable case of this rule in v1 — see
   the Acceptance Criteria behavior-test bullet.) Each candidate is
   evaluated against a cloned copy of that state; only the winning
   candidate's mutations are kept. (The
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

**Rejected: NLM's DOI rule as a worked example for replacing
`ArticleJournalNoPageFallback::Doi`** (`options/bibliography.rs:136`).
Reading NLM's shipped `access` macro precisely
(`styles-legacy/taylor-and-francis-national-library-of-medicine.csl:72-88`)
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

The A-shape uses this spec targets (49 candidates in the disposition audit,
though the v1 content restriction above covers only the subset whose
candidates contain none of the listed structurally-inspected kinds) have no
single slot in common. `volume-title`
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

Evaluation does not live in `crates/citum-engine/src/values/`.
`TemplateGroup::values` (`crates/citum-engine/src/values/list.rs`) has no
render-when handling and is not where variable-once tracking or
substitution bookkeeping happen — grepped, zero hits. Those live on
`Renderer`
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

The tracker-cloning rule in Evaluation step 4 — "discard a losing
candidate's tracker clone entirely, merge only the winner's" — is
necessary but not sufficient on its own. `render_group_component_with_format`
merges a
group's tracker mutations into its parent unconditionally, *before* checking
whether the group produced any output (`tracker.merge_from(group_tracker)`
runs above the subsequent `values?` empty-check), and it does this at
**every level of nesting**, not only at whichever call site happens to be
outermost. So if the *winning* `alternatives:` candidate is itself a
`group:` containing a nested empty sub-group, that sub-group's tracker
mutations are already merged into the candidate's own clone before
`alternatives:` ever inspects it — "merge only the winner's tracker" still
commits the pollution, because it happened one level deeper.

**`alternatives:` therefore cannot be implemented with correct tracker
isolation until `csl26-2hr4` is resolved** (fixed — moving the merge after
the empty-check — or explicitly proven safe as-is). This is now a blocking
prerequisite, not an independent, unrelated cleanup: see Acceptance
Criteria. Fixing the ordering bug in `render_group_component_with_format`
itself (rather than giving `alternatives:` a parallel, duplicate,
transactional evaluator that never calls the shared path) is the
recommended direction — it also corrects plain `group:` rendering on its
own merits, and avoids two different rendering behaviors for a group
depending on whether it happens to sit inside an `alternatives:` candidate.

**Resource-budget accounting.**
`TemplateResourceBudget::check_component`
(`crates/citum-schema-style/src/style/validation.rs:490-521`) increments a
component count and nesting depth for every `TemplateComponent`, and for
`Group` specifically recurses via `check_template(&group.group, ...,
depth + 1)` so a group's children count against
`MAX_TEMPLATE_COMPONENTS`/`MAX_TEMPLATE_NESTING_DEPTH`. `Alternatives` needs
an exactly parallel arm — `check_template(&alt.alternatives, ...,
depth + 1)` — or a style's `alternatives:` candidates would never be counted
at all, bypassing both existing safeguards regardless of the v1 placement
restriction (a flat, non-nested candidate list can still be made arbitrarily
large).

## Acceptance Criteria

- [ ] **Prerequisite:** `csl26-2hr4` resolved (the `render_group_component_with_format`
      tracker-merge-before-empty-check ordering fixed, or explicitly proven
      not to affect `alternatives:`). Not gate-able around — see
      Implementation Notes.
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
      is honored; a losing candidate whose `variable:` component would have
      marked that variable as already-rendered (variable-once tracking)
      does not affect a later template component that renders the same
      variable — the v1 content restriction forbids `Contributor` and
      `Date(Issued)` candidates, so variable-once tracking on a plain
      `variable:` is the reachable forcing case, not contributor/date
      consumption; **a winning candidate containing a suppressed/
      empty nested `group:`, followed by a component that depends on tracker
      state that nested group must not have touched** (the forcing case for
      the `csl26-2hr4` prerequisite above).
- [ ] Engine: `is_term_based` (`crates/citum-engine/src/values/list.rs:122`)
      gains an `Alternatives` arm — `alt.alternatives.iter().all(is_term_based)`,
      mirroring `Group`'s own recursion — so a `group:` wrapping an
      `alternatives:` list whose every candidate is term-only is suppressed
      the same way an equivalent flat term-only `group:` is today. Behavior
      test: a `group:` containing only an `alternatives:` of term-only
      candidates is suppressed; the same shape with one non-term-only
      candidate is not.
- [ ] Schema/validation: `TemplateResourceBudget::check_component` gains an
      `Alternatives` arm recursing through candidates (mirroring `Group`'s),
      so a style's candidate list counts against
      `MAX_TEMPLATE_COMPONENTS`/`MAX_TEMPLATE_NESTING_DEPTH`. Regression
      tests for both limits with an `alternatives:` list as the offending
      structure.
- [ ] Validation: reject `alternatives:` nested inside `alternatives:`, and
      reject any candidate that is or recursively contains (through `Group`
      children or `Message` args) a `Title`, `Contributor`, `Date(Issued)`,
      `Number(Volume)`, `Variable(Url)`, `Variable(Doi)`, or `pattern.*`
      `Message` component (v1 placement restriction) — with a rejection test
      for each listed kind, including one nested inside a `group:` and one
      nested inside a message argument.
- [ ] `just schema-gen` run, schema docs updated.
- [ ] At least one embedded style migrated as a worked example —
      T&F-CSE's publisher-place fallback
      (`taylor-and-francis-council-of-science-editors-author-date-core.yaml`,
      per the Wire Contract section) is the verified-shape candidate.
      Chicago's volume-title position and the NLM DOI case are explicitly
      not targets for this spec (see the Wire Contract section's "Rejected:
      Chicago's `volume-title` position" and "Relationship to existing
      candidate-list types" section's NLM-DOI rejection) — with a
      `report-core.js` diff showing 0 regressions.
- [ ] Status promoted to Active in the implementation commit. Lifting the v1
      placement restriction is separate follow-up work
      (`csl26-57a7`) and does not gate this spec's own promotion.

## Changelog

- v1.4 (2026-09-06): The v1.3 placement restriction mixed content and
  position (forbidding the primary title/contributor *slot* and the
  `article-journal` top level) while its own stated rationale was that
  structural consumers walk the whole component tree regardless of
  position. Rewrote it as a pure content restriction: any candidate that
  is or recursively contains (through `Group` children or `Message` args)
  a `Title`, `Contributor`, `Date(Issued)`, `Number(Volume)`,
  `Variable(Url)`, `Variable(Doi)`, or `pattern.*` `Message` is rejected,
  regardless of where the `alternatives:` sits. Added the `pattern.*`
  message check (`template_policy.rs`'s second structural walk, not
  previously listed). Updated Acceptance Criteria's rejection tests to
  match. Noted this narrows the spec's practical coverage of the 49
  A-shape uses to those whose candidates avoid all listed kinds. See
  `csl26-ro72`.
- v1.3 (2026-09-06): Corrected per a third Codex adversarial-review round:
  the Chicago volume-title worked example silently dropped a real,
  interacting third guard (`part-number-non-numeric`) and was not actually
  equivalent to the style's behavior — replaced with a re-verified T&F-CSE
  publisher-place example that has no such guard; Chicago's volume-title
  position is no longer a target for this spec. Withdrew the "valid
  anywhere" claim: `TemplateComponent::Group` is matched in 21 files for
  non-rendering purposes (grouping, contributor-stripping, article-journal
  template filtering confirmed directly), so v1 restricts placement to
  positions those checked consumers don't touch, with the remaining audit
  tracked in `csl26-57a7`. Added `TemplateResourceBudget` accounting for the
  candidate list, previously unaccounted for entirely. See `csl26-ro72`.
- v1.2 (2026-09-06): Corrected per a second Codex adversarial-review round:
  the v1.1 tracker-clone-and-discard rule only isolated at the
  `alternatives:` boundary, not recursively — a winning candidate's own
  nested `group:` can already have merged a suppressed sub-group's tracker
  mutations before `alternatives:` gets a say (the same
  merge-before-empty-check ordering v1.1 flagged as a `group:` quirk turns
  out to recur at every nesting depth). `csl26-2hr4` elevated from
  independent cleanup to a blocking prerequisite; added the corresponding
  regression-test case to Acceptance Criteria.
- v1.1 (2026-09-06): Corrected per a Codex adversarial review and follow-up
  verification: fixed the evaluation rule (leaf vs. group "did this render"
  semantics were conflated), fixed Implementation Notes to name the real
  integration point (`Renderer`/`grouped/core.rs`, not `values/`), added the
  tracker clone-and-discard rule, dropped the NLM-DOI worked example (routed
  to extending `ArticleJournalNoPageFallback` instead), noted
  `term.place-unknown` doesn't exist yet. See `csl26-8b4a`.
- v1.0 (2026-09-06): Initial draft.
