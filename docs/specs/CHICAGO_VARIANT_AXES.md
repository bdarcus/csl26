# Chicago Variant Axes: Mapping `style-variant-builder` onto Citum Inheritance

**Status:** Draft
**Version:** 1.0
**Date:** 2026-08-07
**Supersedes:** (none)
**Related:** [`CHICAGO_FAMILY_STRATEGY.md`](./CHICAGO_FAMILY_STRATEGY.md),
[`STYLE_INHERITANCE.md`](./STYLE_INHERITANCE.md),
[`STYLE_PRESET_ARCHITECTURE.md`](./STYLE_PRESET_ARCHITECTURE.md),
[`NOTE_SHORTENING_POLICY.md`](./NOTE_SHORTENING_POLICY.md),
beans `csl26-ztl9`, `csl26-adka`, `csl26-cfgw`, `csl26-4xg9`, external:
[`citation-style-language/style-variant-builder`](https://github.com/citation-style-language/style-variant-builder)

## Purpose

Andrew Dunning, maintainer of the CSL Chicago and APA style families,
[announced](https://forums.zotero.org/discussion/125429/implementation-of-the-chicago-manual-of-style-18th-edition)
the CMOS 18th-edition rollout and described the tooling behind it:
[`style-variant-builder`](https://github.com/citation-style-language/style-variant-builder),
a Python build tool that generates a family of related CSL styles from one
shared template plus a set of `.diff` patch files, one per variant. Citum's
Chicago-family styles (`chicago-author-date-18th.yaml`,
`chicago-notes-18th.yaml`, and their children) were migrated from that
family. This spec answers a direct question raised while reviewing that
migration: **does Citum's own inheritance mechanism — a child style's
`extends:` field, plus typed patches on individual reference-type templates —
already do the job `style-variant-builder` does, without needing an external
tool?**

The answer is yes, and this spec exists to show the correspondence
concretely enough that someone who knows the CSL side — the diffs, the
template, the macro-swap idiom — can read straight across to the Citum side.
It is a **mapping document, not an implementation**: it derives the
independent ways the 74 Chicago `.diff` files vary from their shared
template (an *axis map*), states which of those variations Citum's existing
mechanism already expresses and which it does not, and records two real
gaps found while checking, each with a proposed fix direction. It does not
implement any of the 74 variants as
Citum styles; that is separate, later work, once Citum's own Chicago styles
themselves reach exact-text parity — rendering byte-identical to
citeproc-js's output, not just passing a looser pass/fail check (see
[`CHICAGO_FAMILY_STRATEGY.md`](./CHICAGO_FAMILY_STRATEGY.md)).

## Scope

In scope:

- how `style-variant-builder`'s template-plus-diff model and Citum's
  `extends:`-plus-template-patch model correspond, with worked examples
  from both sides
- the axis map: the independent dimensions of variation found across the 74
  Chicago diffs, and which Citum mechanism expresses each one
- two gaps found while checking axis coverage against the current schema,
  each with a proposed fix direction tracked as its own bean — not
  implemented in this spec
- one real defect surfaced by this comparison in a shipped Citum style,
  corroborated by CMOS18 §13.37, and tracked as a bean (not fixed in this
  spec)
- a recommendation on which citation system and which axis are worth
  building next
- portfolio tier and licensing for any future variant styles

Out of scope:

- implementing any of the 74 variants as Citum styles (a later, separate
  piece of work — see Implementation Notes)
- authoring the CSL citation systems Citum has no style for at all: full
  notes-and-bibliography (the CSL template's own default) and the two
  Bluebook-style in-text forms — see axis 1 below
- any change to `chicago-18-base.yaml`, `chicago-author-date-18th.yaml`,
  `chicago-notes-18th.yaml`, or the defect-cluster plan in
  `CHICAGO_FAMILY_STRATEGY.md`
- implementing either gap's proposed fix — each needs its own docs-first
  spec PR (this project's policy for schema changes) before any code lands;
  this spec only proposes a direction and opens the tracking bean

## Design

### Two ways to express "like this style, but..."

CSL 1.0 has no inheritance construct. A CSL style file is a complete,
self-contained XML document; there is no way for one `.csl` file to say "I
am this other file, except for these three macros." `style-variant-builder`
exists to simulate that relationship outside the format:

1. One hand-authored **template** (`templates/chicago-template.csl`, 286 KB)
   carries every variant's logic inline, often as commented-out alternative
   branches next to the active one — for example, a macro chosen for the
   notes-and-bibliography system sits beside a commented-out call to the
   author-date equivalent, tagged with a comment like `<!-- for author-date:
   -->`.
2. Each named variant starts as a working copy of that template
   (`development/<variant>.csl`) with the right branches switched on.
3. `make diffs` runs `diff` between the template and each development copy,
   producing a `.diff` file (`diffs/<variant>.diff`) — an ordinary unified
   diff, the same format `git diff` or `patch` use.
4. `make final` reapplies each `.diff` to the template with `patch`, then
   prunes macros the resulting variant no longer references.

The named variant that ships — `chicago-notes-bibliography-no-url.csl`, say
— is a complete, independent CSL document. Nothing in the file itself
records that it came from a template plus a two-line change; that
relationship exists only in the separate `style-variant-builder` repository,
as an artifact of the build, not as something the CSL format or any CSL
processor can see.

Citum's `extends:` does the equivalent job natively, inside the format, and
resolves it live rather than compiling it ahead of time:

1. A child style's YAML document says `extends: chicago-notes-18th`
   directly — the relationship is data, present in the file, not
   external build-tool state.
2. Where a CSL variant's diff patches a specific macro used by specific
   reference types (book, journal article, and so on), Citum's own patch
   mechanism — `modify:`, `remove:`, `add:` operations under a `match:`
   selector, defined by `TemplateVariantDiff`
   (`crates/citum-schema-style/src/template.rs:698`) — expresses the same
   shape: "take the inherited template for this reference type and change
   these specific parts of it."
3. Resolution happens once, at load time, in
   `crates/citum-schema-style/src/style/overlay.rs` and
   `crates/citum-schema-style/src/template/resolution.rs` — there is no
   `patch` step and no macro-pruning pass, because nothing was duplicated in
   the first place.

The part that is not obvious from reading the schema alone, and is the
actual finding behind this spec, is that step 2 works **across** the
`extends:` boundary. A child style can patch a reference-type template it
never itself authored — one it only has because it inherited it from its
parent. When it does, Citum looks up the parent's already-resolved version
of that template and uses it as the base to patch against
(`style/resolution.rs:180` captures the parent's resolved templates before
the child's own content is applied; `template/resolution.rs:236-268`,
`resolve_variant_parent_template`, is where a patch with no local match
falls back to that captured parent version) — the same relationship `patch`
expresses when it applies a `.diff` against `chicago-template.csl`, except
decided once at load time from data already in both files, not by an
external build step reading a third file. This is exercised by existing
tests, not speculative: `crates/citum-schema-style/tests/bdd_inheritance.rs:53-130`
(`test_template_variant_inheritance`, cases `override_rendering`,
`add_component_before`, `remove_component`) construct exactly this
shape — a parent style plus a child that patches one of the parent's
reference-type templates — and assert the patched result.

| CSL 1.0 / `style-variant-builder` concept | Citum equivalent |
|---|---|
| Shared template file, `chicago-template.csl` | The style at the top of an `extends:` chain, e.g. `chicago-18-base.yaml` / `chicago-notes-18th.yaml` |
| Named variant `.csl` file, fully independent once built | Child style with `extends: <parent>` |
| `.diff` file recording the variant's delta from the template | `type-variants` entry using `modify`/`remove`/`add` instead of a full template replacement |
| Commented-out alternative branch, switched on by hand | The specific value a `modify:` operation sets, or the component an `add:`/`remove:` operation targets |
| `make final`'s `patch` step | Load-time resolution (`style/overlay.rs`, `template/resolution.rs`) |
| `make final`'s macro-pruning pass | Not needed — nothing was duplicated to prune; the child document contains only its own delta |
| The relationship between template and variant, visible only in the separate build-tool repo | The relationship, visible in the child's own `extends:` field |

### Worked example: a genuinely small variant (URL suppression)

`diffs/chicago-notes-bibliography-no-url.diff`'s entire behavioral content,
once the `<info>` metadata lines (title, id, links, summary — the majority
of every diff in the corpus) are set aside, is two lines commented out:

```diff
-      <text macro="source-DOI-URL"/>
+      <!-- <text macro="source-DOI-URL"/> -->
```

repeated at the two macros (`source-date-accessed-DOI-URL-note`,
`source-date-accessed-DOI-URL-bib`) that call it. In Citum terms, that is a
`remove:` operation matching `{variable: url}` (and `{variable: doi}`, where
DOI also renders), authored once per affected reference type in a child
style — no template copy, no macro to prune. The scale of that per-type
repetition, and why some sites resist it entirely, is discussed under Gap A
and Gap B below.

### Worked example: a variant that turns out not to be one (subsequent-note form)

Several diffs — `-subsequent-ibid`, `-subsequent-author`,
`-subsequent-title` — change which macro a Chicago notes style's `<citation>`
points to for citations after the first. Comparing
`citation-notes-full-subsequent-author-title` (the template's own default)
against `citation-notes-full-subsequent-ibid`
(`templates/chicago-template.csl:6663-6687`) shows what actually changes:

```xml
<macro name="citation-notes-full-subsequent-author-title">
  <choose>
    <if position="subsequent">
      <text macro="citation-notes-shortened-author-title"/>
    </if>
    <else><text macro="citation-notes-full"/></else>
  </choose>
</macro>
<macro name="citation-notes-full-subsequent-ibid">
  <choose>
    <if position="ibid-with-locator">
      <text macro="citation-notes-shortened-ibid"/>
    </if>
    <else-if position="ibid"><text term="ibid"/></else-if>
    <else-if position="subsequent">
      <text macro="citation-notes-shortened-author-title"/>
    </else-if>
    <else><text macro="citation-notes-full"/></else>
  </choose>
</macro>
```

The default macro never tests `position="ibid"` at all — an immediately
repeated citation renders identically to any other later citation of the
same source. Only the `-subsequent-ibid` variant adds the branches that test
for immediate repetition and render "Ibid." CSL needs a second whole
document for this because the position test has to be written inline, per
style, inside whichever macro that style has selected.

Citum's analog is not a template patch at all — it's a `citation.ibid` block
declared directly on the style. This is the real block from
`chicago-notes-18th.yaml:47-54`:

```yaml
citation:
  ibid:
    note-start-text-case: capitalize-first
    suffix: ""
    delimiter: ", "
    template:
    - message: term.ibid
    - variable: locator
      suffix: .
```

A style that wants the plain CSL default instead — no "Ibid." ever — simply
omits this block; a style that wants it declares it, as this one does. There
is no second file either way.

Citum already tracks note-position adjacency in the processor itself,
independent of any style: from citation history alone, it knows whether a
citation repeats the immediately preceding note's single source — with the
same locator, or a different one — or is separated from it by a citation to
another source
(`docs/specs/NOTE_SHORTENING_POLICY.md`, "Processor Invariants"). A style
opts into rendering an immediate repeat as "Ibid." by declaring a
`citation.ibid` block; a style that omits it renders an immediate repeat the
same way it renders any other later citation, via `citation.subsequent`
(same doc, rule 4). So in Citum, "does this style use Ibid for immediate
repeats" is not a separate document — it is whether one style declares a
`citation.ibid` block, and both behaviors are reachable from the same parent
style depending on whether a child declares that block or leaves it out.
This is the clearest illustration in the corpus of a difference in kind, not
just mechanism: CSL materializes "with Ibid" and "without Ibid" as two
independent files because it has to; Citum expresses the same choice as one
optional block a style either supplies or omits.

This comparison surfaced a defect in how Citum Chicago styles currently
handle subsequent citations, which is recorded in bean `csl26-adka`.

### The axis map

Each of the 74 Chicago `.diff` files, once `<info>` metadata is set aside,
reduces to a combination of independent variation axes. Nine axes account
for the corpus:

| # | Axis | CSL values | Citum mechanism |
|---|---|---|---|
| 1 | Citation system | full notes-and-bibliography, notes-only, shortened notes-and-bibliography, author-date, in-text (Bluebook-style), in-text shortened(-title) | **Not something a child style can add** — each value is a distinct citation grammar with its own style document, not a variant of a shared one. Citum has three of the six today: notes-only (`chicago-notes-18th.yaml`), shortened notes-and-bibliography (`chicago-shortened-notes-bibliography-core.yaml`), and author-date (`chicago-author-date-18th.yaml`). Missing: full notes-and-bibliography — the CSL template's own default — and the two in-text forms. See Scope. |
| 2 | Subsequent-note form | full author-title (template default), ibid, author-only, title-only | `citation.subsequent` / `citation.ibid` blocks — see worked example above. These carry their own template content, so a child expressing this axis needs a structural patch, not a plain option |
| 3 | URL/DOI presence | present (default) / suppressed (`-no-url`) | `remove:` (or `modify:` with `suppress: true`) per reference type; some sites can't be reached this way at all — see Gap A and Gap B |
| 4 | Access dates | conditional (default) / always shown (`-access-dates`) | template patch on the sites using the `pattern.accessed-date` locale message (e.g. `chicago-author-date-18th.yaml:520,553,695`) |
| 5 | Archive-place ordering | default / archive place first | template `add:`/`remove:` reordering |
| 6 | Publisher place | omitted (default) / shown, optionally with label+page | template `add:` |
| 7 | Edition vintage | 18th-edition rules (default) / `-classic` (CMOS17-era) / `-17th-edition` | mixed options + template patches |
| 8 | Annotation | none (default) / annotated / annotated with abstract | `add:` an abstract/annotation component |
| 9 | Subsequent-author substitute (the em-dash convention for repeated authors in a bibliography) | on / off | plain option, `subsequent-author-substitute` in `bibliography.options` (Rust: `crates/citum-schema-style/src/options/bibliography.rs:38`) |

Axis 9 is the one true plain-option axis in the corpus — a child style
expressing it needs only a one-line option, no template content. Every
other axis Citum can currently express — 2 through 8 — needs a structural
template patch, because CSL's diffs at those axes touch specific rendered
components for specific reference types. Axis 1 is out of scope: it is
new-style authoring work, not something an `extends:` child can add, because
each value is a genuinely different citation grammar, not a delta on a
shared one. Axis 7 mixes option and structural changes closely enough that
it is also out of scope for a first pass.

### Two gaps found while checking axis coverage

Both were found by checking real code paths against real diff content while
working out axis 3 (`-no-url`). Both look like accidental gaps in the schema
rather than intentional constraints, and each has a proposed fix direction
below — but implementing either is a schema and engine change, and per this
project's policy needs its own docs-first spec PR before any code lands.
Each is tracked as its own bean; neither is implemented here.

**Gap A — no style-wide switch for URL/DOI text, only per-reference-type
control.** `LinksConfig.url` / `LinksConfig.doi`
(`crates/citum-schema-style/src/options/mod.rs:605-655`) feed
`resolve_effective_url`
(`crates/citum-engine/src/values/variable.rs:433-455`), which decides
whether a rendered URL/DOI becomes a clickable link — not whether it renders
as text at all. Hiding one URL is already possible: a `modify:` operation
can set `suppress: true` on the matched component
(`Rendering.suppress`, `crates/citum-schema-style/src/template.rs:143`), the
same field the schema's own docs give as an example for "don't show
publisher for journals." So a `-no-url` child style is buildable today; it
just has to author that once per reference type instead of once for the
whole style — `chicago-author-date-18th.yaml` alone has such sites at lines
258, 285, 303, 399–400, 478, 512, 531, 565, 600, 653, 681–682, 707, 724, and
more.

This reads as an oversight, not a design choice: `subsequent-author-substitute`
(axis 9) already shows the project is willing to give a single, common,
style-wide stylistic choice its own option, and suppressing URLs entirely is
at least as common across academic citation styles. Proposed direction: a
new boolean, e.g. `links.show-text` (default true), checked in `variable.rs`
alongside the existing hyperlink logic, so that when false the URL/DOI
variable resolves to no value at all — the same way an absent reference
field already does — instead of only gating the hyperlink. Tracked as bean
`csl26-cfgw`.

**Gap B — the style-wide fallback template can be replaced but not patched.**
Every style has one fallback bibliography template used for any reference
type that has no template of its own. That field, `BibliographySpec.template`
(`crates/citum-schema-style/src/style/sections/bibliography.rs:40`), and its
citation-side counterpart, `CitationSpec.template`
(`crates/citum-schema-style/src/style/sections/citation.rs:56`), are both a
plain list of components — not the `Full`-or-`Diff` type
(`TemplateVariant`, `crates/citum-schema-style/src/template.rs:653`) that
reference-type templates already use. So `modify`/`remove`/`add` only work
per reference type; under `extends:` the fallback template is whole-replace
only, confirmed by `test_explicit_null_clears_bibliography_template`
(`crates/citum-schema-style/tests/bdd_inheritance.rs:448`). This matters for
axis 3 concretely: `chicago-author-date-18th.yaml`'s own fallback
bibliography template renders `variable: doi` and `variable: url`
unconditionally as its final two components (lines 1041–1043), and a child
style reproducing `-no-url` cannot remove just those two — only copy the
entire 41-line fallback template, which then stops tracking future changes
to the parent's version.

This also reads as an oversight: nothing in `STYLE_INHERITANCE.md`'s merge
rules explains why the fallback template should be exempt from the same
patch mechanism reference-type templates already have. Proposed direction:
widen both fields from `Option<Template>` to `Option<TemplateVariant>`,
reusing the exact `Full`/`Diff` resolution already built and tested for
reference-type templates, so a child's `Diff` patches the parent's
already-resolved fallback template the same way it already patches a
parent's reference-type template. Tracked as bean `csl26-4xg9`.

### What would be sensible to add next

Of the six CSL citation systems (axis 1), full notes-and-bibliography is the
natural one for Citum to add next: it is the CSL template's own default, and
`chicago-notes-18th.yaml` (notes-only) plus
`chicago-shortened-notes-bibliography-core.yaml` (shortened
notes-and-bibliography) already establish the pattern of a full form and a
shortened sibling sharing a family — full notes-and-bibliography would slot
in as a third sibling with an unshortened bibliography, not a new citation
grammar built from nothing. The two in-text (Bluebook-style) forms are a
bigger, more specialized undertaking — a citation grammar unlike anything
Citum's Chicago family has today — and are a lower priority.

Of axes 2 through 9, the ones with no open gap are the sensible ones to
build first: axis 9 (a one-line option) and axes 4, 5, 6, and 8 (reference-type
patches only, no fallback-template or `links:` involvement). Axis 2
(subsequent-note form) is next — also structural, but already fully
supported and demonstrated in the worked example above. Axis 3 (URL/DOI
suppression) is the lowest priority of the seven until Gap A or Gap B lands;
building it today means either the per-reference-type workaround described
there, or a fallback-template copy that immediately drifts from its parent.

### Portfolio tier

If and when any of these axes are implemented as actual Citum styles,
`STYLE_INHERITANCE.md`'s portfolio policy already answers where they go:
community tier, meaning they live in the sibling `citum-styles` repository,
`extends:` an embedded Chicago style, and their parity against citeproc-js
is tracked for information, not enforced by CI — the same tier as every
other child style built on an embedded parent. The embedded set stays
exactly as it is today; nothing here proposes adding to it.

### Licensing

`style-variant-builder`'s `templates/` and `diffs/` directories are CC
BY-SA 3.0, the same license as the upstream
[`citation-style-language/styles`](https://github.com/citation-style-language/styles)
repository already vendored into this repo as the `styles-legacy` submodule.
Any future Citum style derived from reading these files would carry the
same `source:` attribution block Citum's existing Chicago styles already use
(`original-authors`, `license`, `csl-id` — see
`chicago-notes-18th.yaml:20-33` for the current pattern), extended to credit
`style-variant-builder`'s diff as the specific source of the variant's
delta, not just the shared template.

## Implementation Notes

- No engine or schema change is needed for axes 2, 4, 5, 6, 8, and 9; the
  mechanism exists and is tested. Axis 3 can be reproduced today with the
  per-reference-type workaround in Gap A, but would benefit from that gap's
  proposed fix. Implementing any of these axes is style-authoring work in
  `citum-styles`, tracked separately from this spec; the two schema
  proposals are tracked as their own beans (`csl26-cfgw`, `csl26-4xg9`), not
  implemented here.
- Citum's own Chicago styles are not yet at exact-text parity with
  citeproc-js (`CHICAGO_FAMILY_STRATEGY.md`'s defect-cluster plan,
  `csl26-h7oc`, is mid-flight). Building child styles on top of an unfinished
  parent multiplies unverified output, so implementing any axis from this
  map should either wait for its parent style to stabilize, or ship with its
  own citeproc-js baseline generated from `style-variant-builder`'s own
  output (`make final` in that repo materializes all 74 variants; each can
  be run through `node scripts/oracle.js` independently of this repo's own
  fixture corpus).
- This spec does not itself change `citation.ibid` on
  `chicago-notes-18th.yaml` — see the worked example above and bean
  `csl26-adka`, which tracks the fix.

## Acceptance Criteria

- [x] The correspondence between `style-variant-builder`'s template-plus-diff
      model and Citum's `extends:`-plus-template-patch model is stated with
      concrete code citations on the Citum side.
- [x] The axis map covers all 74 Chicago diffs in the corpus and correctly
      states which of the six CSL citation systems Citum currently covers.
- [x] Both gaps found while checking axis coverage are recorded with file
      and line evidence and a proposed fix direction, each tracked as its
      own bean (`csl26-cfgw`, `csl26-4xg9`) — not implemented in this spec.
- [x] The `citation.ibid` defect is recorded and tracked as a bean
      (`csl26-adka`), not silently fixed or silently dropped.
- [x] The spec states which citation system and which axis are sensible to
      build next.
- [x] Portfolio tier and licensing are stated by reference to existing
      specs, not re-decided here.

## Changelog

- 2026-08-07: Initial version.
- 2026-08-07: Corrected the citation-system count in the axis map — Citum
  covers three of the six CSL systems (notes-only, shortened
  notes-and-bibliography, author-date), not two; full notes-and-bibliography
  and the two in-text forms are missing. Replaced undefined internal
  shorthand ("head", "wrapper") with plain language throughout.
- 2026-08-07: Review pass — shortened the `citation.ibid` finding to a
  one-line pointer at bean `csl26-adka`; added the missing Citum-side YAML
  to the subsequent-note-form worked example; reassessed Gap A and Gap B as
  likely accidental gaps (not intentional constraints) with a proposed fix
  direction for each, tracked as beans `csl26-cfgw` and `csl26-4xg9`; added
  a recommendation on which citation system and axis are sensible to build
  next.
