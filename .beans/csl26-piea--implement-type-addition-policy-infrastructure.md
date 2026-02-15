---
# csl26-piea
title: Implement type addition policy infrastructure
status: todo
type: task
priority: normal
created_at: 2026-02-15T00:17:53Z
updated_at: 2026-02-15T00:17:53Z
---

Complete infrastructure for type addition policy enforcement.

**Policy documented:** TYPE_ADDITION_POLICY.md (active)
**Decision:** Hybrid model (Option A) with 4-factor test

**Remaining Tasks:**

1. **PR Template Update**
   - Add "New Reference Type" issue template
   - Require 4-factor test completion for type proposals
   - Include fields: semantic distinction, style discrimination evidence,
     unique fields list, parent relationship analysis

2. **Decision Matrix Examples**
   - Create 10 test cases evaluating the 4-factor test
   - Types to evaluate: Dataset, Software, Preprint, Standard, Map,
     Chart, Figure, Performance, Artwork, Podcast
   - Document in TYPE_ADDITION_POLICY.md appendix

3. **Current Type Audit**
   - Verify all SerialComponent subtypes pass policy test
   - Document rationale for structural vs flat decisions
   - Add to TYPE_ADDITION_POLICY.md

4. **biblatex Mapping Guide**
   - Document biblatex @types → CSLN type mapping
   - Explain structural (collection/serial) vs flat decisions
   - Add to migration documentation

5. **Style Guide Update**
   - Add type addition policy to CONTRIBUTING.md
   - Link from CLAUDE.md (already done)
   - Add policy summary to README.md

**Deliverables:**
- .github/ISSUE_TEMPLATE/new_reference_type.md
- Decision matrix examples (10 cases)
- Updated TYPE_ADDITION_POLICY.md with audit
- docs/migration/BIBLATEX_MAPPING.md
- Updated CONTRIBUTING.md

**Dependencies:**
- None (policy is active, infrastructure is enhancement)

**Refs:** csl26-wodz, TYPE_ADDITION_POLICY.md
