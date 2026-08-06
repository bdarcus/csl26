---
# csl26-huuz
title: Disambiguation collision grouping is variable-based, not render-text-based
status: todo
type: bug
priority: normal
tags:
    - engine
    - fidelity
created_at: 2026-08-06T12:43:05Z
updated_at: 2026-08-06T12:44:42Z
parent: csl26-ccdt
---

citeproc-js groups year-suffix disambiguation collisions by RENDERED TEXT
equality (the CSL <sort>/disambiguation macro's actual output), while citum's
Disambiguator groups by abstract variable equality (same author-slot key,
same date value) regardless of what text the style's template actually
renders for that reference. For most references these coincide, but they
diverge whenever a style's date/name macro is type-conditional.

## Evidence (csl26-m8la, 2026-08-06 session)

gb-t-7714-2025-author-date's upstream CSL date-intext macro is conditional:
for article-journal/article-magazine types without volume/issue it falls
through to a plain <date variable="issued"> branch (never reaching the outer
<else> that renders the locale's "no date" term), while all other types with
no issued date DO reach that outer <else> and render "无日期"/"n.d.".

Two article-journal references (gbt7714.7.2.1:7, gbt7714.7.2.3:7) with no
issued date therefore render as bare "Anon，b." / "Anon，c." in the oracle —
no "n.d." term at all — and citeproc-js treats them as a SEPARATE,
non-colliding disambiguation sequence from the "Anon，n.d.-X" sequence the
other undated anonymous references share (confirmed: both
gbt7714.7.2.1:7="Anon，b." and gbt7714.7.3:7="Anon，n.d.-b." independently
reach letter 'b' in the oracle output — only possible if they're in separate
groups).

citum's Disambiguator has no notion of the rendered text — it only sees
"author=None (ANONYMOUS_FALLBACK_KEY), date=None" for both, so it lumps all
12 English-language anonymous-undated references (plus a 13th,
gbt7714.8.11.2.2:2, a webpage the oracle excludes from this bucket entirely
for a similarly unreplicated reason) into ONE shared collision group,
producing a systematic letter-count mismatch (citum's group has 13 members;
oracle's equivalent groups have 10 + 2 = 12, with different membership) that
manifests as a consistent +2 letter offset for every entry after the first
divergence point.

Full before/after diagnostic data (entry IDs, letters, rendered text) is in
this bean's parent PR discussion.

## Why this is architectural, not a bounded fix

Fixing this properly means giving Disambiguator's collision-key computation
awareness of what the ACTIVE TEMPLATE would actually render for a reference
(type-conditional branches, available-date/accessed fallbacks, etc.) — or
else switching collision detection to compare rendered text directly, closer
to citeproc-js's own algorithm. Either is real design work spanning the
template-resolution and disambiguation modules, not a targeted patch.

## Scope note

csl26-m8la's shipped fix (registry-order year-suffix ties for a resolved
`group_sort`) is unaffected by this and already brings
gb-t-7714-2025-author-date's adjusted bibliography oracle failures from 42 to
30 (out of 203), with zero regressions in citum-engine's test suite. A
follow-up bean (`csl26-q67h`, "restore gb-t-7714-2025-author-date's own
bibliography.sort") covers the still-missing explicit sort — this bean covers
only the residual ~9-entry gap in the English anonymous-undated bucket that
traces to the grouping/rendering mismatch described above, which is
independent of whether that sort gets restored.
