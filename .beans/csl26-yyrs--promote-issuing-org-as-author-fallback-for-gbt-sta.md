---
# csl26-yyrs
title: Promote issuing-org as author fallback for GB/T standard type
status: todo
type: task
priority: normal
tags:
    - fidelity
    - style
created_at: 2026-07-23T17:06:02Z
updated_at: 2026-08-02T14:19:29Z
parent: csl26-ccdt
---

GB/T 7714-2025 author-date's `standard` reference type: oracle promotes the issuing committee/organization into the author slot (e.g. `全国信息与文献标准化技术委员会，2021. GB/T 3792—2021 ...`) when no personal author exists. Citum's `standard` type-variant has no path to promote an org/publisher-shaped field into the author position, so it falls through to the `佚名` anonymous fallback instead, dropping the org name entirely.

## Evidence

3 corpus entries fail on this (tests/fixtures/test-items-library/gb-t-7714-2025.json, gbt7714.8.9.2:1-3), unchanged since the original 2026-07-22 triage in csl26-6eak.

## Likely approach

Needs a `substitute.template` extension — either a new `SubstituteField` variant for publisher/issuing-org, or a `TemplateContributor`-shaped publisher-as-contributor path (crates/citum-schema-style/src/options/substitute.rs, crates/citum-schema-style/src/template.rs). Scope carefully: if this needs to generalize beyond GB/T's one type-variant, consider whether it should be a broader `substitute` schema feature rather than a GB/T-specific hack.

Part of csl26-6eak (Tune gb-t-7714-2025-author-date to full fidelity).
