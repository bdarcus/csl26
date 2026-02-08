---
# csl26-m3lb
title: Implement hybrid migration strategy
status: todo
type: milestone
priority: high
created_at: 2026-02-08T00:19:16Z
updated_at: 2026-02-08T00:38:40Z
---

Strategic pivot from pure XML semantic compiler to hybrid approach combining XML options extraction, output-driven template inference, and hand-authored templates.

**Context:** Current migration achieves 87-100% citation match but 0% bibliography match across all top parent styles. The XML compiler excels at extracting global options but fails at template structure due to type-specific branch flattening (not node ordering). See docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md for full analysis.

**Three-Tier Architecture:**

1. **Keep XML pipeline for OPTIONS** - Options extractor, preset detector, locale handling (~2,500 lines working code). Do not touch.
2. **Hand-author templates for top 5-10 parent styles** - Starting from examples/apa-style.yaml as a model. Covers 60% of dependent styles with highest confidence.
3. **Build output-driven template inference for next tier** - Use citeproc-js output + input data cross-referencing. Requires hardened oracle.js parser and expanded test fixtures.
4. **Retain XML compiler as fallback** - For remaining 290 parent styles.
5. **Oracle cross-validation for all approaches** - Where approaches agree, confidence is high.

**Success criteria:**
- APA bibliography: 0% -> 80%+ match
- Top 10 styles: bibliography match comparable to citation match
- XML options pipeline remains intact
- Citation match does not regress (currently 87-100%)

**Estimated effort:** ~1,500 lines new code + 5-10 hours domain-expert time for hand-authored templates.