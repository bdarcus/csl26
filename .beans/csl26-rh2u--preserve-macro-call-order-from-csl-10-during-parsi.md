---
# csl26-rh2u
title: Preserve macro call order from CSL 1.0 during parsing
status: todo
type: bug
priority: high
created_at: 2026-02-07T19:52:56Z
updated_at: 2026-02-08T00:19:20Z
blocking:
    - csl26-ifiw
    - csl26-m3lb
---

Problem: CSLN renders components in wrong order compared to CSL 1.0. Oracle shows contributors → year → title but CSLN renders title → contributors → year.

Investigation revealed this is a symptom of a broader architectural issue with the XML semantic compiler approach. The template compiler has hit a wall: 0% bibliography match across ALL top parent styles despite 87-100% citation match.

Failed approach: Built source_order infrastructure that tracked depth-first traversal order, but assigned wrong orders (title=0 when it should be last). Reverted in commit 1c9ad45.

Root cause: Fundamental model mismatch between CSL 1.0 (procedural: macros, choose/if/else, groups with implicit suppression) and CSLN (declarative: flat templates with typed overrides). The XML compiler excels at options extraction but fails at template structure.

**Resolution: See docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md for full analysis and recommended hybrid approach.**

This bean is blocked by implementation of the hybrid migration strategy (csl26-hybrid milestone).