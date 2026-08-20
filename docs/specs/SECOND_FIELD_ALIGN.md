---
title: Second-Field-Align Bibliography Layout
status: Active
created: 2026-08-20
---

# Second-Field-Align Bibliography Layout

**Status:** Active
**Date:** 2026-08-20
**Related:** bean `csl26-qdff`; `REFERENCE_MARKERS.md`

## Purpose

CSL 1.0's `<bibliography second-field-align="flush|margin">` is a **layout**
attribute: citeproc-js renders each bibliography entry as two sibling boxes —
a left-margin box holding the reference marker (`[1]`, `Kuh62`) and a
right-inline box holding the entry body — so a consuming stylesheet can align
the second field (the body) into a column, independent of marker width. This
spec gives that layout a runtime representation and an HTML rendering, so an
entry's marker and body become separately addressable output, not just an
already-fused string.

`REFERENCE_MARKERS.md` gave the reference marker a first-class value on
`ProcEntry` and settled the *text* half of this problem: flush concatenation
(`[1]J. Smith`, no inserted space) is the shipped default because that is
exactly what citeproc-js's two-box output flattens to. That spec explicitly
deferred second-field-align's structural half to this one. This spec does not
reopen the text path.

## Scope

**In scope:**

- A `second-field-align` option on `BibliographyConfig` / `BibliographyOptions`,
  parsed from CSL 1.0's `<bibliography second-field-align="…">` attribute.
- A runtime `BibliographyLayout` value carrying that setting alongside
  `hanging-indent` (parsed today, consumed by nothing — see
  [Hanging indent](#hanging-indent)).
- An `OutputFormat` seam (`entry_slots`) at the point a marker meets its body,
  with a default that reproduces today's fused string exactly, so every
  non-HTML format is unaffected by construction.
- An HTML rendering that emits the marker and body as two sibling `<div>`s
  when alignment is declared.
- `citum-migrate` reading the real attribute instead of the placeholder
  comment in `options_extractor/processing.rs`.

**Explicitly out of scope:**

- `entry-spacing` and `line-spacing`. CSL 1.0 does not define these as
  `<bibliography>` attributes in the first place (they are citeproc-js
  rendering-engine options, not style-authored data), `csl-legacy` parses
  neither, and nothing in this spec models them.
- Hanging-indent realization in non-HTML formats. LaTeX/Typst have their own
  hanging-indent primitives (`\hangindent`, list environments); wiring those up
  is separate follow-on work, not part of this spec.
- Adoption across shipped styles. No embedded or exemplar style declares
  `second-field-align` as a result of this spec; landing the mechanism must be
  provably byte-identical to `main`, *for the `second-field-align` option
  specifically*, on every style. `hanging-indent` is a **narrower exception**:
  see [Hanging indent](#hanging-indent) — it is already declared on 11 shipped
  styles and closing its dead output is an intentional, visible (markup-only)
  change for exactly those.
  Turning `second-field-align` on for `ieee`, `american-medical-association`,
  `royal-society-of-chemistry`, and the other numeric styles whose CSL source
  carries the attribute is a follow-up bean — it changes shipped HTML and
  deserves its own parity review.
- **Compound-numeric entries.** `processor/bibliography/compound.rs` renders
  grouped entry bodies through `render_entry_body_components_with_format` and
  assembles its own markers directly; it never crosses the `entry_slots` seam
  this spec adds. A style declaring both `compound-numeric` and
  `second-field-align` renders compound entries fused, regardless of the
  alignment setting. Extending slot rendering to compound entries is future
  work if a real style needs it.
- Author-date collapse and other citation-side concerns. This spec is
  bibliography-layout only; citation rendering is untouched.
- Re-litigating the marker model itself (`REFERENCE_MARKERS.md` already
  settled that).

## Design

### Model

```rust
/// Runtime bibliography layout, resolved from `BibliographyConfig` and handed
/// to the output format at the point a marker meets its body.
pub struct BibliographyLayout {
    /// CSL `second-field-align`. `None` when the style declares neither value.
    pub second_field_align: Option<SecondFieldAlign>,
    /// CSL `hanging-indent`.
    pub hanging_indent: bool,
}

pub enum SecondFieldAlign {
    /// Marker and body render flush; a consuming stylesheet aligns the body
    /// into a second column regardless of marker width.
    Flush,
    /// Marker and body render with a margin/gutter between the two boxes.
    Margin,
}
```

`Flush` and `Margin` are kept as distinct values, not collapsed into a
boolean, even though **HTML emits identically for both** — alignment is a
CSS-layer concern in both cases; the two sibling `<div>`s are what let a
stylesheet express either. The distinction is preserved for round-trip
fidelity: CSL source can say `margin`, and a lossy `bool` would make
CSL → Citum → CSL conversion drop that information for no rendering benefit.

### Schema surface

New option on `BibliographyConfig` (`crates/citum-schema-style/src/options/bibliography.rs`)
and its style-authoring counterpart on `BibliographyOptions`
(`crates/citum-schema-style/src/options/mod.rs`):

```yaml
bibliography:
  options:
    second-field-align: flush   # or: margin
```

Kebab-case serde, `JsonSchema` derive, forward-compat `unknown_fields`
handling — matching every sibling option in that file (`anonymous-entries`,
`hanging-indent`). It participates in style inheritance the same way
`hanging-indent` does today: present on `BibliographyOptions::merge` via the
existing `merge_options!` macro, so a child style inherits a parent's
declaration under `extends` unless it overrides.

### Rendering seam

Today, `render_entry_body_with_format` fuses the marker and body
unconditionally: `format!("{marker}{body}")`
(`crates/citum-engine/src/render/bibliography.rs`). This spec inserts one seam
on `OutputFormat`:

```rust
fn entry_slots(
    &self,
    marker: Option<&str>,
    body: Self::Output,
    layout: &BibliographyLayout,
) -> Self::Output {
    // Default: today's fuse, expressed through `affix` (not `format!`,
    // which would only compile for `Output = String`) so every format that
    // doesn't override this is byte-identical to current output.
    self.affix(marker.unwrap_or(""), body, "")
}
```

Only `Html` overrides it, emitting `citum-entry-marker` / `citum-entry-body`
sibling `<div>`s when `layout.second_field_align` is set. `PlainText`,
`Markdown`, `Djot`, `Org`, `LaTeX`, `Typst`, and any third-party
`OutputFormat` implementation inherit the default and render exactly as they
do today, whether or not a style declares the option — because none of them
override the seam.

`bibliography()` also receives `layout`, so the HTML container can carry the
hanging-indent class once, rather than repeating it per entry.

### Hanging indent

`hanging-indent` is parsed by `csl-legacy` and carried on
`BibliographyConfig` today, but nothing reads it — confirmed by a full grep of
the engine and CLI crates. This spec's layout model gives it a consumer: HTML
renders a hanging-indent class on the bibliography container
(`citum-bibliography--hanging-indent`), letting a stylesheet apply the
standard `text-indent`/`padding-left` hanging-indent pattern. This closes the
dead-field gap named in the bean's own acceptance criteria without inventing
new schema — `hanging-indent` already exists, it simply had no output.

**Unlike `second-field-align`, this is not adoption-free.** 11 shipped styles
already declare `hanging-indent: true`, so wiring it to output is a real,
intentional HTML markup change (an added CSS class only — no visible text
changes) for exactly those styles, confirmed by grepping the real (non-symlink)
paths:

- Embedded-core (7): `apa-7th`, `chicago-author-date-18th`,
  `chicago-shortened-notes-bibliography-core`, `elsevier-harvard-core`,
  `gb-t-7714-2025-author-date`, `modern-language-association`,
  `springer-basic-author-date-core`.
- Exemplar/other (3): `chicago-notes-bibliography-17th-edition`,
  `international-journal-of-wildland-fire`, `mhra-notes`.
- Experimental (1): `jm-turabian-multilingual`.

`entomological-society-of-america`,
`taylor-and-francis-council-of-science-editors-author-date-core`, and
`american-medical-association` declare `hanging-indent: false`, which is a
no-op — `BibliographyLayout::from_config` treats `Some(false)` the same as
unset, so those three render unchanged. Any test asserting exact HTML
bibliography-container markup for one of the 11 `true` styles must be updated
to expect the modifier class in the same change that lands this spec's
implementation.

### HTML output

```html
<div class="citum-bibliography citum-bibliography--hanging-indent">
  <div class="citum-entry" id="ref-smith2000">
    <div class="citum-entry-marker">[1]</div><div class="citum-entry-body">J. Smith, <i>Book A</i>. 2000.</div>
  </div>
</div>
```

`citum-*` names, not citeproc-js's `csl-left-margin` / `csl-right-inline` —
consistent with the rest of Citum's HTML vocabulary
(`citum-bibliography`, `citum-entry`, `citum-title`, …), none of which matches
citeproc's class names either, so csl-* naming here would not make existing
citeproc stylesheets apply to Citum output regardless.

### Warnings interaction

`bibliography_label_missing_separator_warnings`
(`crates/citum-engine/src/api/warnings.rs`) currently fires whenever a numeric
or alphabetic `label-mode` has no `label-wrap` and no `label-separator`, with
its doc comment naming `royal-society-of-chemistry`'s in-file prose comment as
the justification for treating a bare flush marker as possibly-intentional.
Once `second-field-align` is declarable, a declared value is the *affirmative*
signal that flush is intentional — the warning should be suppressed when
`second_field_align.is_some()`, replacing the prose-comment justification with
a structural one.

## Rejected alternatives

- **Rendering the split in every output format.** Plain text has no
  representation of a two-column layout; forcing one would either do nothing
  (defeating the purpose) or break plain-text parity with citeproc-js's
  flattened output, which `scripts/oracle.test.js` pins.
- **Reusing `label-separator` to express alignment.** `label-separator` is
  spacing between the marker and body within a single fused string — exactly
  the kind of option `REFERENCE_MARKERS.md` (§Motivation, "spacing became
  schema") warns against overloading. Alignment is a structural layout
  decision, not a spacing value; conflating the two would resurrect that
  failure mode.
- **Emitting `csl-left-margin` / `csl-right-inline`.** Would partially align
  with citeproc-js CSS but split Citum's own HTML vocabulary in two, since no
  other Citum container or entry class follows citeproc naming.
- **Modelling `second-field-align` as a `bool`.** Loses the `flush`/`margin`
  distinction CSL source carries, for no rendering benefit (HTML emits the
  same shape either way) — but a real cost to round-trip fidelity.

## Implementation Notes

- Parity is the gate, continuously, per `REFERENCE_MARKERS.md`'s own
  implementation notes for this area: `node scripts/report-core.js
  --all-features` and `just check-core-quality` must hold at every step.
  Scope the byte-identical check correctly: since no shipped style declares
  `second-field-align`, every embedded/exemplar style's output must be
  **byte-identical** to `main` on that axis. `hanging-indent` output is the
  named exception — the 11 styles listed under
  [Hanging indent](#hanging-indent) gain a container CSS class, and
  `report-core`'s text-based comparison should show zero effect there (the
  matcher strips HTML markup before comparing), but confirm this empirically
  rather than assuming it — and any exact-HTML-markup test assertion for one
  of those 11 styles needs updating in the same change. Confirm the rest by
  direct diff against a `main` baseline worktree, not just by trusting the
  report's normalized percentages.
- `render_entry_body_with_format` stays `pub` with its current signature,
  delegating internally to the new seam, since it is used outside
  `render/bibliography.rs` (`processor/rendering/tests.rs`).
- The annotation append in `refs_to_string_slice_with_format` must attach to
  the *body* slot before the marker/body fuse happens, so an annotated entry's
  annotation lands inside `citum-entry-body`, not outside both slots.

## Acceptance Criteria

- [x] `second-field-align` is declarable on `BibliographyConfig` /
      `BibliographyOptions`, kebab-case, forward-compat, schema-generated.
- [x] `csl-legacy` parses `<bibliography second-field-align="…">`.
- [x] `citum-migrate` maps the parsed attribute onto the new option.
- [x] `OutputFormat::entry_slots` exists with a default that is byte-identical
      to today's fuse for every format that doesn't override it.
- [x] `Html` renders sibling `citum-entry-marker` / `citum-entry-body` divs
      when alignment is declared, and today's flush string when it isn't.
- [x] `hanging-indent` renders a container class in HTML.
- [x] `bibliography_label_missing_separator_warnings` is suppressed when
      `second_field_align` is declared.
- [x] Every embedded/exemplar style's rendered output is byte-identical to
      `main` on the `second-field-align` axis; the 11 `hanging-indent: true`
      styles named above gain a container CSS class and nothing else,
      confirmed by `report-core.js`'s text-based comparison plus updated exact
      HTML-markup test assertions.
- [x] `just pre-commit`, `just schema-gen`, `node scripts/report-core.js
      --all-features`, and `just check-core-quality` all green.

## Related specs

- [REFERENCE_MARKERS](REFERENCE_MARKERS.md) — the marker model and flush-text
  default this spec builds on; lists this spec's scope as its own non-goal.
- [BIBLIOGRAPHY_RENDERING_PIPELINE](BIBLIOGRAPHY_RENDERING_PIPELINE.md) —
  selection and layout precedence this spec's rendering sits downstream of.
- [UNIFIED_SCOPED_OPTIONS](UNIFIED_SCOPED_OPTIONS.md) — where the new option
  lives in the scoped-option model.

## Changelog

- 2026-08-20: Initial version.
