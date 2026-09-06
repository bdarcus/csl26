---
# csl26-t1hh
title: MLA-style locator-type-conditional citation delimiter
status: completed
type: bug
priority: normal
tags:
    - citation
    - engine
    - rendering
    - schema
created_at: 2026-09-02T23:01:27Z
updated_at: 2026-09-06T20:51:32Z
parent: csl26-h7oc
blocked_by:
    - csl26-7652
    - csl26-8r9r
---

MLA's citation layout (`mla.csl`) picks the delimiter between the author-title
group and the locator based on locator *type*:

    <if locator="line page timestamp" match="any">
      <group delimiter=" "><text macro="author-short"/><text variable="locator"/></group>
    </if>
    <else>
      <group delimiter=", "><text macro="author-short"/><text macro="label-locator"/></group>
    </else>

Citum's schema has no way to express this. `TemplateGroupCondition`
(`render-when`, `crates/citum-schema-style/src/template.rs:1807`) is
field-presence only over reference fields (`TemplateConditionField`) — it
can't test a *citation item's* locator type, and there's no delimiter-choice
mechanism at all (a group has one static `delimiter:`).

Confirmed via oracle (`node scripts/oracle.js styles-legacy/modern-language-association.csl`
with an ad-hoc page/section-locator fixture — see csl26-475u's investigation
notes for the exact command): shipped MLA citation exact parity is 11/20;
these are the non-exact cases from that gap specifically:

    with-locator (page):     O "(Kuhn, "…Revolutions" 23)"       C "(Kuhn, "…Revolutions," 23)"
    multi-item-with-locators: O "…10; see also … 440)"            C "…10; see also …, 440)"
    (section-labeled locators, e.g. "sec. 5", already take the comma correctly)

A static `prefix: ", "` on MLA's `variable: locator` (the bean's original
suggestion) makes section-labeled locators correct but breaks these
page/line/timestamp ones — not a fix, just trades which five cases fail.

## Doctrine note

csl26-qyub audited `render-when` for removal and landed on *specified
retention*, explicitly declining to grow template-conditional mechanisms
further — the stated direction is options/presets/type-variants, not more
`render-when` surface. A locator-type-conditional delimiter is exactly the
kind of new conditional growth that doctrine pushes against.

**Recommendation: default to an options/preset-level mechanism** (e.g. a
locator-aware delimiter override alongside `citation.options.locators`),
not a `TemplateGroupCondition`/`render-when` extension. Extending
render-when is functionally reintroducing a CSL-style `<choose
locator="...">` conditional into the template layer — the exact pattern
Citum's declarative model has deliberately moved away from. Only reconsider
render-when if a docs/specs review demonstrates the options/preset approach
genuinely cannot express this case; it is the fallback, not a co-equal
option to weigh against the default.

Per repo policy (CLAUDE.md "Schema changes need a docs-only PR first"), spec
in docs/specs/ first, status Draft -> Active in the implementation commit,
before any schema/engine change.

## Cross-reference (2026-09-06)

Superset bean csl26-7652 covers this MLA gap plus the analogous APA
label-case gap under one schema addition. Spec drafted at
docs/specs/LOCATOR_RENDERING.md, "Label Case and Attachment (v1.1)"
section (currently Draft, pending review) — implements exactly the
options/preset-level mechanism this bean's investigation notes
recommended: a per-kind `attach` field on `LocatorConfig`/
`LocatorKindConfig`, resolved through the existing
`supplies_own_leading_separator` prefix-suppression path rather than any
`render-when`/`TemplateConditionField` extension.

This bean stays open as the MLA-specific tracking/verification bean;
closes when csl26-7652's implementation PR lands and the two MLA exact-
parity rows quoted above (with-locator page, multi-item-with-locators)
flip.

## Partial resolution (2026-09-06)

csl26-7652's implementation lands the options/preset-level `attach`
mechanism this bean's investigation recommended. The single-item repro
quoted above ("with-locator", page locator) now flips to exact parity:
`(Kuhn, "The Structure of Scientific Revolutions" 23)`.

The second repro ("multi-item-with-locators") does NOT yet flip --
implementation revealed that a *grouped multi-item* citation renders each
item through a different code path
(filter_author_from_template/render_item_from_template_with_format) that
the v1.1 `attach` mechanism doesn't reach. Split out to csl26-8r9r
("Extend locator attach to grouped multi-item citations"). This bean
stays open, now blocked by csl26-8r9r, until that lands.

## Fully resolved (2026-09-06)

csl26-8r9r fixed the multi-item grouped-citation join gap. Both repro
strings quoted in this bean's original body now match exactly:
- with-locator (page): "(Kuhn, "The Structure of Scientific
  Revolutions" 23)"
- multi-item-with-locators: "(Kuhn, "The Structure of Scientific
  Revolutions" 10; see also LeCun et al 440)"

Verified via full-corpus report-core.js sweep (zero regressions,
fidelityScore unchanged everywhere).
