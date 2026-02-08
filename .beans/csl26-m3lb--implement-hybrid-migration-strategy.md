---
# csl26-m3lb
title: Implement hybrid migration strategy
status: todo
type: milestone
priority: high
created_at: 2026-02-08T00:19:16Z
updated_at: 2026-02-08T00:19:16Z
---

Strategic pivot from pure XML semantic compiler to hybrid approach combining XML options extraction with output-driven template generation.

**Context:** Current migration achieves 87-100% citation match but 0% bibliography match across all top parent styles. Analysis shows XML compiler excels at extracting global options (name formatting, et-al rules, dates) but fails at template structure due to fundamental model mismatch.

**Architecture:** (See docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md)

1. Keep XML pipeline for OPTIONS - Options extractor, preset detector, locale handling (~2,500 lines working code)
2. Build output-driven template generator - Use citeproc-js output + input data cross-referencing for template structure
3. Retain XML compiler as fallback - For rare types and validation
4. Cross-validation - Where both agree, confidence is high

**Subtasks:**
- Expand test fixtures from 15 to 25-30 references covering all major types
- Build output-driven template inferrer (~500-800 lines, extend oracle.js component parser)
- Integrate with existing options pipeline (~200 lines)
- APA proof-of-concept (most complex: 99 macros, 126 choose blocks)
- Generalize to top 10 parent styles (60% of dependents)

**Success criteria:**
- APA bibliography: 0% → 80%+ match
- Top 10 styles: bibliography match comparable to citation match
- XML options pipeline remains intact

**Estimated effort:** ~1,000 lines new code + integration work