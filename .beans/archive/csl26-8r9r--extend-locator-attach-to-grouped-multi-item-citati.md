---
# csl26-8r9r
title: Extend locator attach to grouped multi-item citations
status: completed
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
    - engine
created_at: 2026-09-06T19:51:38Z
updated_at: 2026-09-06T20:51:23Z
parent: csl26-ccdt
---

docs/specs/LOCATOR_RENDERING.md's "Label Case and Attachment" v1.1 addition
(bean csl26-7652, implemented) added a per-kind `attach` delimiter for
locators, threaded through ProcTemplateComponent::locator_attach and
RenderedComponent::join_delimiter_override, consumed only in
citation_to_string_with_format's join loop
(crates/citum-engine/src/render/citation.rs). That loop is the join
mechanism for a single citation item's (or an integral citation's)
top-level template list.

A GROUPED citation with more than one item (e.g. MLA's
"multi-item-with-locators": "(Kuhn, Title 10; see also LeCun et al 440)")
renders each item's remaining template through a different path --
filter_author_from_template / render_item_from_template_with_format
(crates/citum-engine/src/processor/rendering/grouped/core.rs) -- which
computes its own "leading affix" between the externally-rendered author
heading and the item's locator. This path never consults
join_delimiter_override, so a locator inside a multi-item grouped citation
still gets the pre-v1.1 comma join instead of its configured `attach`.

Repro: modern-language-association.yaml's `multi-item-with-locators`
citation fixture. Confirmed via
`node scripts/report-core.js --all-features --styles modern-language-association`:
citum renders "(Kuhn, "The Structure of Scientific Revolutions" 10; see
also LeCun et al, 440)" vs oracle "(... see also LeCun et al 440)" --
note the stray comma before "440".

Needs: extend the leading-affix computation in
filter_author_from_template/leading_group_affix/author_group_delimiter_affix
to also consult a locator's effective `attach` (values::locator::effective_attach)
when the item's remaining template's first item is `variable: locator`.
Read the full leading-affix-scavenging history first (see the csl26-475u
comment in grouped/core.rs) before touching this path -- it already has a
subtle "external join" contract that a prior fix (csl26-475u) had to
correct.

## Summary of Changes

Fixed. Root cause: a grouped multi-item citation renders each item's
remaining (author-stripped) template through
filter_author_from_template/render_item_from_template_with_format, which
splices the externally-rendered author heading onto the item's own
rendered content using a delimiter GUESSED from the item's structurally
first remaining TemplateComponent -- wrong whenever that component (e.g.
a disambiguate-only title) renders empty at runtime and a later component
(e.g. a locator with `attach`) becomes the item's true first visible
content.

Fix: new `leading_join_delimiter_override()` in
crates/citum-engine/src/render/citation.rs -- given an item's own
ProcTemplate, renders components in order and returns the
`join_delimiter_override` of the first one that renders non-empty text
(i.e. citation_to_string_with_format's own answer to "what actually
renders first", not a structural guess). Threaded through
render_item_from_template_with_format and
render_group_item_from_template_with_format (both now return
`Option<(String, Option<String>)>`), and preferred over the static
`leading_affix` guess in render_group_item_parts_with_format when
establishing `group_delimiter`.

Verified: full-corpus `report-core.js --all-features` (all 35 embedded
styles, not just locators:-using ones, since this touches the shared
grouped-citation join path) shows 8 exact-parity improvements (all in
modern-language-association, including the target
multi-item-with-locators row) and ZERO regressions; fidelityScore
unchanged for every style. Full `cargo nextest run` 2767/2767 green.

Found via an external adversarial (Codex) review of the parent PR that
correctly flagged this as user-visible output corruption before merge.
