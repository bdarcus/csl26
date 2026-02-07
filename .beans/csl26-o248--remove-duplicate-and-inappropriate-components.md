---
# csl26-o248
title: Remove duplicate and inappropriate components
status: completed
type: bug
priority: high
created_at: 2026-02-07T18:20:06Z
updated_at: 2026-02-07T19:52:42Z
parent: csl26-ifiw
---

## csl26-o248 Investigation Findings

**Current State (Batch Oracle - Top 10):**
- Citations: 60% perfect (6/10 styles)
- Bibliography: 0% perfect (0/10 styles)
- Top issues: year:extra (21), year:missing (19), issue:missing (10), editors:missing (8), volume:extra (3)
- Ordering issues: 46 total

**Root Causes Identified:**

1. **Component Ordering Problem:**
   - CSL 1.0 APA bibliography macro order: author → date → title → source
   - CSLN migrated template order: title → contributor → date → ...
   - The template_compiler doesn't preserve macro call order from CSL 1.0

2. **Duplicate Components Problem:**
   - CSL 1.0 has ONE source-serial macro with conditional logic
   - CSLN generates THREE separate "volume + parent-serial" components
   - Example in APA: Lines ~68, ~74, ~81 all try to render volume/containerTitle

**Architecture Analysis:**
- Template compiler has occurrence-based merging system (collect_occurrences → merge_occurrences)
- Designed to handle conditional branches with suppress overrides
- Groups components by variable key, merges with correct suppress semantics
- **BUT**: List components return None for get_variable_key(), so Lists containing "volume + parent-serial" don't get grouped/merged

**Next Steps:**
- Fix List component grouping in merge_occurrences (use list signature consistently)
- Preserve macro call order from CSL 1.0 bibliography macro
- This is a template_compiler refactor - belongs in parent epic csl26-ifiw

**Files to Modify:**
- crates/csln_migrate/src/template_compiler/mod.rs (merge_occurrences, collect_occurrences)
