---
# csl26-m3lb
title: Implement hybrid migration strategy
status: todo
type: milestone
priority: high
created_at: 2026-02-08T00:19:16Z
updated_at: 2026-02-15T15:10:04Z
---

Strategic pivot from pure XML semantic compiler to hybrid approach combining
XML options extraction, output-driven template inference, and hand-authored 
templates.

**Context:** Current migration achieves 87-100% citation match but 0%
bibliography match across all top parent styles. The XML compiler excels at
extracting global options but fails at template structure due to type-
specific branch flattening (not node ordering). See
docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md for full analysis.

**Three-Tier Architecture:**

1. **Keep XML pipeline for OPTIONS** - Options extractor, preset detector,
locale handling (~2,500 lines working code). Do not touch.
2. **LLM-author templates for top 5-10 parent styles** - Using /styleauthor
skill or @styleauthor agent. Validated with APA 7th (5/5 citation +
bibliography). Covers 60% of dependent styles with highest confidence. See
bean csl26-o3ek.
3. **Build output-driven template inference for next tier** - Use citeproc-
js output + input data cross-referencing. Requires hardened oracle.js parser
and expanded test fixtures.
4. **Retain XML compiler as fallback** - For remaining 290 parent styles.
5. **Oracle cross-validation for all approaches** - Where approaches agree,
confidence is high.

**Success criteria:**

• APA bibliography: 0% -> 80%+ match (APA 7th now at 100% via LLM authoring)
• Top 10 styles: bibliography match comparable to citation match
• XML options pipeline remains intact
• Citation match does not regress (currently 87-100%)

**Estimated effort:** ~1,500 lines new code. LLM-authored templates replace
manual domain-expert time.

**Latest Progress (2026-02-15):**

✅ **Locale Term Infrastructure Complete**
* Implemented RoleLabel system for locale-specific role labels
* Added term, form, placement configuration to TemplateContributor
* Integrated with existing locale.role_term() infrastructure
* All pre-commit checks passing (fmt, clippy, test)
* Commits: 48001bb, 8e261be

✅ **AMA Style Updated**
* Applied locale term labels to editor component
* Fixed duplicate editor rendering for edited books (suppress override)
* Oracle validation: 7/7 citations, bibliography formatting gaps remain

🔄 **Next Steps:**
1. Test label system with Vancouver and IEEE numeric styles
2. Create documentation for label feature usage
3. Show APA example demonstrating integral/non-integral citation handling
4. Address AMA bibliography formatting issues:
   - Volume/issue spacing: "2, (2)" -> "2(2)"
   - Editor label punctuation: "(eds.)" -> ", eds."
   - Page delimiter consistency
5. Continue LLM authoring for top 10 parent styles
