---
# csl26-cfgw
title: 'Proposal: style-wide switch to suppress URL/DOI text (not just hyperlinking)'
status: todo
type: feature
priority: normal
tags:
    - schema
    - style
    - chicago
created_at: 2026-08-07T23:53:02Z
updated_at: 2026-08-07T23:53:02Z
---

Surfaced in docs/specs/CHICAGO_VARIANT_AXES.md (Gap A) while checking whether Citum's inheritance covers the CSL Chicago family's -no-url variant. LinksConfig.url / LinksConfig.doi (crates/citum-schema-style/src/options/mod.rs:605-655) feed resolve_effective_url (crates/citum-engine/src/values/variable.rs:433-455), which controls whether a rendered URL/DOI becomes a clickable link, not whether it renders as text at all. A single component can already be hidden per reference type with suppress: true on a modify: operation (Rendering.suppress, crates/citum-schema-style/src/template.rs:143), so a -no-url child style is possible today, just authored once per reference type (chicago-author-date-18th.yaml alone has ~18 such sites).

This looks like an accidental gap, not an intentional one: subsequent-author-substitute (a comparably narrow, single-purpose style-wide choice) already has its own top-level option, and suppressing URLs entirely is at least as common across academic citation styles. Proposed direction: a new boolean, e.g. links.show-text (default true), checked in variable.rs alongside the existing link-resolution logic, so that when false, the URL/DOI variable resolves to no value at all -- the same way an absent reference field already does -- rather than only gating the hyperlink.

This is a schema and engine change. Per this project's policy, it needs its own docs-first spec PR (with rejected alternatives) before any implementation -- not done as part of the Chicago-axes mapping spec.

- [ ] Draft a docs/specs/*.md spec proposing the links.show-text (or equivalent) option, including rejected alternatives (e.g. reusing links.url/links.doi -- rejected because that would also break the legitimate 'plain-text URL, no hyperlink' case)
- [ ] Get the spec reviewed and merged as Draft
- [ ] Implement in a follow-up PR, status flips to Active in that same commit
