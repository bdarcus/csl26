---
# csl26-o333
title: Fix GB/T author-date date-parsing edge cases (approximate year, dropped body date)
status: todo
type: bug
priority: low
tags:
    - style
    - fidelity
created_at: 2026-07-23T17:06:18Z
updated_at: 2026-08-02T14:19:29Z
parent: csl26-ccdt
---

Small cluster of unresolved GB/T 7714-2025 author-date rendering gaps against the upstream corpus (tests/fixtures/test-items-library/gb-t-7714-2025.json, --scope bibliography), triaged in csl26-6eak's 2026-07-23 session but not attempted (low item count, distinct root causes from the main disambiguation work):

1. **Approximate/bracketed year falls to no-date term instead of the approximate value** — `gbt7714.8.11.2.2:1` (oracle: `佚名，[2025a]. 中国国家博物馆...`, citum: `佚名，无日期-p. ...`), `gbt7714.8.11.3.2:1` (oracle: `高等教育文献保障系统，[2025]. ...`, citum: `...无日期`), `gbt7714.8.11.3.2:5` (oracle: `Zotero，[2024]. ...`, citum: `Zotero，n.d. ...`). These items have no `issued` but do have `accessed`; the front `date: issued, form: year` component's `fallback:` chain apparently doesn't reach `accessed` the way the oracle's rendering implies it should for these carrier types (GB/T §7.5.4.3 uncertain-date convention — bracketed year).

2. **Body date silently dropped** — `gbt7714.8.5.1.1:7`: oracle shows `佚名，2024b. [J]. 2024-05-09` (a full-precision body date after the bracket), citum shows `佚名，2024b. [J]` with the body date missing entirely.

3. **Unexplained bare disambiguation letter** — `gbt7714.7.2.1:4`: oracle shows `佚名，b. [J]. ...` (a bare letter `b` with no `无日期-`/`n.d.-` prefix at all, unlike every other item in the same no-date collision group). Not understood; may be an oracle/citeproc-js quirk rather than a citum gap — verify against citeproc-js behavior directly before assuming citum is wrong.

Part of csl26-6eak (Tune gb-t-7714-2025-author-date to full fidelity). Low priority — 5 entries total, none block the higher-leverage csl26-m8la (suffix ordering) or csl26-yyrs (org-as-author) follow-ups.
