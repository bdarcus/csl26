---
# csl26-7652
title: No locator-type-conditional template primitive
status: todo
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
created_at: 2026-09-06T15:56:25Z
updated_at: 2026-09-06T15:56:25Z
parent: csl26-ccdt
---

Both APA and MLA need their citation locator's LABEL and its JOIN
DELIMITER to a preceding component to vary by locator kind (page/line/
timestamp vs everything else), and neither is expressible today.

## APA (1 row, locator-section-with-suffix)
apa.csl's label-locator macro (styles-legacy/apa.csl:204-236): page and
paragraph locators get form="short" ("p. 33"); every other kind
(including section) gets text-case="capitalize-first" with NO
abbreviation ("Section 12"). Citum's `variable: locator` component
already reads a per-style `options.locators` LocatorConfig
(crates/citum-schema-style/src/options/locators.rs), which supports
`kinds.<kind>.label-form` -- but that key space is keyed by
LocatorType, and there's no way to express "everything except page/
paragraph gets long+capitalized" short of enumerating every other
LocatorType variant individually (fragile, and doesn't match the CSL's
actual is-numeric/else-branch logic in the general `<else>` arm).

Repro: apa-7th, citation "(Hawking, 1988, sec. 12, esp. discussion)" vs
oracle "(Hawking, 1988, Section 12, esp. discussion)".

## MLA (10 bibliography-adjacent citation rows)
mla.csl:1146-1157 conditions the delimiter joining author-short to the
locator on locator type: `group delimiter=" "` for locator="line page
timestamp" (bare value, no label -- "(Kuhn, Title 23)"), vs `group
delimiter=", "` + a labeled macro for every other kind ("(Hawking,
section. 12)"). Citum's modern-language-association.yaml citation
template renders `variable: locator` as its own top-level array item
after the [author+title] group, so it always gets the citation's
default ", " delimiter -- correct for the labeled-kind branch, wrong
for page/line/timestamp, which is the overwhelming majority of MLA's
citations (10/34 N-punctuation residual rows, all identical shape:
"(Author, Title, 23)" vs oracle "(Author, Title 23)").

## The gap
TemplateConditionField (crates/citum-schema-style/src/template.rs:1822)
has no locator-kind variant, and `render_when` only exists on `group`
components (crate::template.rs:1790-1816), not leaf components like
`variable: locator`. LocatorConfig governs the locator's own label/
range/period rendering, not the delimiter joining it to the rest of
the template.

## Scope
Needs either: (a) a `TemplateConditionField::LocatorKind` (or similar)
usable in `render_when` on a wrapping group, so the citation template
can pick delimiter/prefix by locator kind directly; or (b) folding the
join-delimiter into LocatorConfig itself (a `patterns`-like per-kind
join override). Engine (crates/citum-engine) + schema
(crates/citum-schema-style), not style YAML. A docs/specs/ proposal
should cover both call sites (APA's citation-side sole use, MLA's much
larger surface) since a schema addition should serve both from day one.
