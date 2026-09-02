---
# csl26-475u
title: MLA drops locator delimiter entirely in same-author collapse
status: in-progress
type: bug
priority: normal
tags:
    - citation
    - engine
    - rendering
created_at: 2026-08-18T12:49:10Z
updated_at: 2026-09-02T17:30:43Z
parent: csl26-h7oc
---

Found while probing csl26-uctc (locator-aware collapse delimiter). MLA's
same-author collapse concatenates locators onto titles with no delimiter at
all, because group_delimiter derivation (render_group_item_parts_with_format,
crates/citum-engine/src/processor/rendering/grouped/core.rs) scavenges a
leading affix off the first non-author template component, and MLA's
title-based template has none to find.

modern-language-association, [@ITEM-31, p. 100; @ITEM-32]:

    (Garcia, "Methods for Robust Climate Attribution"100, "Methods for
    Probabilistic Climate Attribution")

Wanted something like:

    (Garcia, "Methods for Robust Climate Attribution", 100, "Methods for
    Probabilistic Climate Attribution")

Likely needs a delimiter fallback in group_delimiter derivation (mirroring
the intra_delimiter default used elsewhere) rather than relying solely on a
scavenged leading affix. Separate root cause from csl26-uctc, which is about
choosing between two *existing* delimiters, not synthesizing a missing one.

## Investigation notes (2026-09-02, branch fix/475u-mla-group-delimiter-fallback, uncommitted diagnostic work)

Root cause confirmed and narrower than the original description: the bean's
"group_delimiter derivation... has none to find" is only half right --
`leading_group_affix` DOES find an affix here (title's own `prefix: ", "`,
scavenged to join author->title externally). The actual defect is in the
SAME loop's `item_delimiter` (render_group_item_parts_with_format,
crates/citum-engine/src/processor/rendering/grouped/core.rs ~line 688):
`strip_item_delimiter` zeroes `item_delimiter` to avoid double-delimiting
that externalized author->title join, but `citation_to_string_with_format`
threads that ONE delimiter value uniformly across EVERY join in the item's
template -- so zeroing it also silently drops the join before any LATER
sibling with no leading affix of its own (MLA's bare `variable: locator`
after the title group). Reproduces even for a single, non-collapsed
citation item -- NOT specific to same-author collapse.

**Two engine fixes attempted, both reverted after regressing other embedded
styles:**

1. Fall back to `params.intra_delimiter` whenever more than one top-level
   component survives author-stripping. Fixed MLA. Regressed
   `processor::rendering::tests::test_type_specific_rendering`: a Date
   component with its OWN declared `prefix: ", "` got double-delimited
   (`"Title B, , 2021"`).
2. Narrowed the fallback to fire only when the immediate next component has
   no *static* `prefix`/`wrap` of its own (checked via `leading_group_affix`
   for prefix, added a `wrap.is_some()` check), injecting the delimiter as
   that component's own `Rendering.prefix`
   (`DelimiterPunctuation::Custom(...)`) instead of touching the shared
   `item_delimiter` value. Fixed MLA AND the type_specific_rendering case
   (a Date with `wrap: parentheses` -- no textual prefix, but self-delimits
   via the paren -- correctly left alone). Still regressed the REAL embedded
   MLA style via `document.rs::example_documents::
   mla_plain_text_shows_integral_name_memory`: produced orphan leading
   commas ("(Kuhn, , sec. 5)" instead of "(Kuhn, sec. 5)", "(, 10)" instead
   of "(10)") whenever the component BEFORE the one I injected into renders
   EMPTY (MLA's title is `disambiguate-only`, often suppressed; author can
   also be suppressed) -- `citation_to_string_with_format` already drops
   empty components from its `parts` array before joining, so its delimiter
   application is inherently "only between components that actually
   rendered." A statically-injected `prefix` has no way to know its
   predecessor vanished and fires unconditionally.

**Conclusion:** `citation_to_string_with_format`/`push_delimiter`
(crates/citum-engine/src/render/citation.rs) support exactly one uniform
delimiter value for a whole item template. The set of "this component
already supplies its own separator" shapes is open-ended (own `prefix`, own
`wrap`, a *previous* sibling's `suffix`, and likely more) and interacts with
dynamic empty-component dropping in a way no static per-component check can
safely account for. A real fix needs one of:

- Per-join delimiter support in `citation_to_string_with_format` (aware of
  which components actually rendered) -- the general, higher-risk engine fix.
- A narrower, non-engine fix: add `prefix: ", "` to MLA's own `variable:
  locator` template component in
  crates/citum-schema-style/embedded/styles/modern-language-association.yaml
  (and check the 4 other MLA-family styles --annotated-bibliography,
  no-url, notes, notes-no-url -- for the same gap). Sidesteps the shared
  code path entirely; zero regression risk to other styles, but doesn't fix
  the underlying engine gap for any future style with the same template
  shape.

Reproduction test added and kept, marked `#[ignore]`:
`crates/citum-engine/tests/citations.rs::
mla_shaped_template_keeps_title_locator_delimiter_in_same_author_collapse`
(bare fn + wrapper in the `note_style_positions` mod). Run with
`cargo test -p citum-engine --test citations -- mla_shaped_template
--include-ignored` to see it fail against the current (reverted) engine.

`core.rs` has been fully reverted to the fix/4if2 baseline on this branch;
only the ignored test and this bean update remain as diagnostic residue.
