# Status-Aware Date Fallback Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-08-30
**Supersedes:** (none)
**Related:** `docs/specs/LOCALE_MESSAGES.md`, `docs/policies/LOCALIZATION_INTEGRITY.md`,
`csl26-qmxw` (bean), `crates/citum-engine/src/values/date.rs`
(`render_date_fallback_chain`), `crates/citum-schema-style/src/options/date_fallback.rs`

## Purpose

A reference with no `issued` date but a `status` (`forthcoming`, `in press`,
`ahead of print`, …) should print that status word in the date slot instead
of falling through to the locale's generic "n.d." term. Citum currently has
no way to express this: the options-level date-fallback chain that decides
what to render when `issued` is absent can only try another date variable or
a locale message — it cannot render a reference's own `status` field. Every
Chicago-family style (author-date and notes) needs this; right now they all
silently render "n.d." for a forthcoming or in-press work.

## Scope

In scope: a new date-fallback candidate that renders a `status`-shaped
variable, and wiring it into `chicago-author-date-18th`'s own
`date-fallback` configuration.

Out of scope: `chicago-notes-18th` and
`chicago-shortened-notes-bibliography-core` have the identical CSL pattern
(`styles-legacy/chicago-notes.csl:864,4238` — the same
`<else-if variable="status">` branch) and need the same fix, but are left
for a follow-up bean — the same-instant fix would extend this PR beyond
`csl26-qmxw`'s own acceptance bar (four `chicago-author-date-18th`
fixture references). Also out of scope: any change to
`TemplateConditionField` — see Rejected Alternatives.

## Evidence

`styles-legacy/chicago-author-date.csl`'s `date`/`date-short` macros
(bibliography and note forms, lines 766–881) both structure the date slot as
an if/else-if/else chain:

```xml
<if variable="issued">
  ...
</if>
<else-if variable="event-date">...</else-if>
<else-if variable="available-date">...</else-if>
<else-if variable="status">
  <!-- Print the status variable rather than use generic CSL terms (`in press`, etc.) -->
  <text text-case="capitalize-first" variable="status"/>
  <text variable="year-suffix"/>
</else-if>
...
<else>
  <text form="short" term="no date"/>
  <text variable="year-suffix"/>
</else>
```

The CSL comment is explicit about *why*: print the reference's own status
text, not a locale term. `date-sort-group` (the same file, a few lines
below) documents the same three-way distinction as a sort-ordering rule too
("1. items with dates (= 0) 2. `no date` items (= 1) 3. items with `status`
… (= 2)") — CMOS orders status-only entries between dated and undated ones,
useful primary-source evidence for a future bibliography-sort spec, noted
here in case that work picks it up.

On the Citum side:

- `status` exists on the reference types that need it
  (`crates/citum-schema-data/src/reference/types/structural.rs:238, 798,
  1062`) and is already a renderable `SimpleVariable`
  (`SimpleVariable::Status => reference.status()`,
  `crates/citum-engine/src/values/variable.rs:293`).
- `term.forthcoming` exists in `en-US.yaml:980`, but it is the *wrong*
  mechanism — it's a generic CSL term ("forthcoming"), not the reference's
  actual `status` text ("in press", "ahead of print", …), which is exactly
  what the CSL comment above warns against substituting.
- Today, `chicago-author-date-18th.yaml` sets `date-fallback: standard`
  (`options/date_fallback.rs`'s `DateFallbackPreset::Standard`), whose only
  candidate is the locale's short "no date" message
  (`DateFallbackRulePreset::Standard` → `message_no_date()`). A reference
  with `status: forthcoming` and no `issued` falls straight through to
  that message, rendering "n.d." — the reported bug.
- `render_date_fallback_chain`
  (`crates/citum-engine/src/values/date.rs:1154`) already tries each
  candidate in a resolved list in order and returns the first that renders;
  it's built to be extended with new candidate kinds. `DateFallbackCandidate`
  is presently a closed `Date | Message` enum
  (`crates/citum-schema-style/src/options/date_fallback.rs:23`) — neither
  arm can render a bare reference variable.
- `DateFallbackRule::Candidates(Vec<DateFallbackCandidate>)` already lets a
  style author write an **explicit** candidate list in YAML instead of a
  named preset (`DateFallbackEntry::Explicit`,
  `crates/citum-schema-style/src/options/date_fallback.rs:313`) — so no new
  preset is needed, only a new candidate *kind* to put in that list.

## Design

Add a third `DateFallbackCandidate` arm:

```rust
pub enum DateFallbackCandidate {
    Date(DateFallbackDate),
    Message(DateFallbackMessage),
    Variable(DateFallbackVariable),   // new
}

pub struct DateFallbackVariable {
    pub variable: SimpleVariable,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}
```

`to_template_component()` maps `Variable` to `TemplateComponent::Variable`
(already an existing, general component kind — used directly in templates
today, e.g. `- variable: publisher`), carrying the candidate's `rendering`
through unchanged.

`render_date_fallback_chain` needs one small extension: the disambiguation
year-suffix append logic already special-cases
`matches!(component, TemplateComponent::Message(_))` to *append* the
suffix after the rendered text (as opposed to the `Date` candidate's
*inline* placement inside its own wrap). A `Variable` candidate follows the
CSL pattern exactly — `<text variable="status"/>` then a separate
`<text variable="year-suffix"/>` — so it needs the same append treatment as
`Message`. That match arm becomes
`matches!(component, TemplateComponent::Message(_) | TemplateComponent::Variable(_))`.

`chicago-author-date-18th.yaml` moves from the `standard` preset to an
explicit equivalent, with the status candidate inserted before the
terminal no-date message:

```yaml
options:
  date-fallback:
    first-issued:
      default:
        - variable: status
          text-case: capitalize-first
        - message: term.no-date
          form: short
```

(`text-case: capitalize-first` matches the CSL's bibliography-form
`text-case="capitalize-first"`; the note-form's `text-case="lowercase"`
variant is out of scope here since only the bibliography slot is fixed —
see Scope.)

## Rejected Alternatives

**A generic `TemplateConditionField::Status` + `render-when` gating on the
template's own date component**, instead of extending the fallback chain.
Rejected: the bug is entirely inside `render_date_fallback_chain`'s
existing candidate-list mechanism — the date slot already has a working,
tested, priority-ordered fallback system built for exactly this shape of
problem ("if the primary thing is missing, try each of these in order").
Reaching for a *second*, independently-authored `render-when` chain outside
that mechanism would duplicate its ordering and its disambiguation-suffix
handling, and would require the style author to hand-roll the
issued-present / status-present / neither branching that
`DateFallbackRule::Candidates` already expresses as an ordered list. It
also does not fit the schema as naturally: `TemplateConditionField`'s
existing variants are all citation-shape concerns (contributor/date/title
presence for macro branching elsewhere in a template), not date-slot
fallback priority.

**Routing to `term.forthcoming`** (the existing locale term) instead of the
`status` variable. Rejected per the CSL source's own comment: it loses the
reference's actual status text ("in press", "ahead of print", …), printing
the same generic word regardless of what the reference says.

## Implementation Notes

- No `STYLE_SCHEMA_VERSION` bump — this is an additive, backward-compatible
  schema change (see `just schema-gen` in the CLAUDE.md pre-commit gate).
- `render_date_fallback_chain`'s doc comment (`values/date.rs:1130-1153`)
  already documents the `Message`/`Date` split; it needs one added sentence
  covering the new `Variable` arm's append behavior.
- Regenerate `docs/schemas/` and the data-model reference docs in the same
  commit as the implementation (`just schema-gen`), per CLAUDE.md.

## Acceptance Criteria

- [ ] `DateFallbackCandidate::Variable` added to
      `crates/citum-schema-style/src/options/date_fallback.rs`, schema
      regenerated.
- [ ] `render_date_fallback_chain` renders a `Variable` candidate and
      appends the year-suffix disambiguation letter after it, matching the
      `Message` candidate's behavior.
- [ ] `chicago-author-date-18th.yaml`'s bibliography date fallback renders
      `Forthcoming.` (capitalized, from the reference's own `status` field)
      instead of `n.d.` for the four `chicago-18th.json` fixture references
      carrying `status: forthcoming` (`V54M6HLX`, `JXGCXGLD`, `9RPXBW6V`,
      `94SYPMEQ`).
- [ ] No other oracle entry in `chicago-author-date-18th`'s corpus moves.
- [ ] Unit tests: BDD-named, `#[rstest]` with 2+ cases (a `status`-present
      entry and a genuinely undated entry falling through to "n.d."),
      `assert_eq!` on the captured rendered string.

## Changelog

- v1.0 (2026-08-30): Initial draft, `csl26-qmxw`.
