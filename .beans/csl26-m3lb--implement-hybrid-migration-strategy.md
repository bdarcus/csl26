---
# csl26-m3lb
title: Implement hybrid migration strategy
status: completed
type: milestone
priority: high
created_at: 2026-02-08T00:19:16Z
updated_at: 2026-02-19T20:49:49Z
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

**Success criteria (re-baselined 2026-02-19):**

• APA bibliography: 0% -> high-fidelity match (✅ ACHIEVED: 27/27 in current oracle set)
• Top 10 styles: bibliography match comparable to citation match (🔄 IN PROGRESS: 0-27/34 depending on style)
• XML options pipeline remains intact (✅ MAINTAINED: ~2,500 lines preserved)
• Citation match does not regress (🔄 strict oracle now uses 8 citation scenarios per style)

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


Latest Progress (2026-02-19):

✅ Output-driven automation promoted to primary migration path in csln-migrate.

- Branch: codex/output-driven-migrate-automation
- PR: https://github.com/bdarcus/csl26/pull/193
- Commit: f829b60

Implemented:
- Section-aware template resolution for both bibliography and citation.
- Explicit source modes: auto|hand|inferred|xml.
- Confidence-gated inferred template intake with XML fallback only when unresolved.
- Section-keyed cache artifacts for offline Rust migration:
  * templates/inferred/<style>.bibliography.json
  * templates/inferred/<style>.citation.json
- infer-template fragment output updated to emit section key + citation wrap metadata.
- batch-infer script now precompiles both sections per style.

Validation completed:
- cargo fmt
- cargo clippy --all-targets --all-features -- -D warnings
- cargo nextest run


Correction:
- Cache artifact pattern is templates/inferred/STYLE_NAME.bibliography.json and templates/inferred/STYLE_NAME.citation.json.


PR Conclusion (2026-02-19):

- PR #193 establishes a scalable output-driven migration baseline.
- Template migration is now section-aware (citation + bibliography) with inferred artifacts as primary input.
- Inferred mode is cache-only, enabling precompute-once then Rust-only migration runs.
- XML template compilation remains fallback-only for unresolved sections.
- Added migration docs at crates/csln_migrate/README.md.


Benchmark Update (2026-02-19, stratified sample n=100):

Status:
- ✅ Completed wider random benchmark across 100 styles:
  * 40 author-date
  * 40 numeric
  * 20 note
- ✅ Dynamic confidence scoring shipped in inference engine (commit b265280).
- ✅ No benchmark execution errors in final paired rerun.

Results (xml vs inferred):
- Overall citation: 79.0% -> 89.9% (+10.9pp)
- Overall bibliography: 84.8% -> 88.8% (+4.0pp)
- Author-date: citation +4.1pp, bibliography +9.0pp
- Numeric: citation +0.0pp, bibliography -1.2pp
- Note: citation +46.3pp, bibliography +4.3pp

Caveats:
- Numeric bibliography remains a mild aggregate regression (-1.2pp) with several
  strong negative outliers.
- Inference cache generation currently fails for a small minority (3/100 in this
  sample): international-review-of-the-red-cross, journal-of-the-indian-law-institute,
  art-history.
- Confidence scoring is heuristic and needs calibration data from larger random
  samples before freezing thresholds.

Next steps:
1. Triage numeric bibliography regression cluster and add targeted guardrails for
   inferred numeric bibliography templates.
2. Improve inference precompile reliability for the three cache-fail styles.
3. Add a benchmark gate in docs/PR workflow:
   - require non-regression for citations overall
   - require positive bibliography delta overall
   - report numeric bibliography separately
4. Run another wider random benchmark (>=200) after numeric fixes to confirm
   generalization.

## Closure Note (2026-02-19)

Strategy validated and deployed. Core objectives achieved:
- PR #193 merged: output-driven template inference promoted as primary migration path
- APA 7th: 27/27 bibliography (from 0%)
- Benchmark across 100 styles: citations +10.9pp, bibliography +4.0pp vs XML baseline
- Three-tier fallback (inferred -> hand-authored -> XML) operational

Follow-on work tracked in:
- csl26-l2hg: numeric style regression triage
- csl26-heqm: top 10 parent styles to 100% fidelity
- csl26-gidg: 90% oracle match corpus goal
