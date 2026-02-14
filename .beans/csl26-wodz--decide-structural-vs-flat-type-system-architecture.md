---
# csl26-wodz
title: 'Decide: structural vs flat type system architecture'
status: todo
type: feature
priority: high
created_at: 2026-02-14T22:53:24Z
updated_at: 2026-02-14T22:53:24Z
---

Architectural decision triggered by legal citations PR #164.

**Question:** Should CSLN use structural types (Monograph, SerialComponent),
flat types (JournalArticle, MagazineArticle, LegalCase), or hybrid?

**Current State:** Hybrid (structural for academic, flat for legal)

**Options:**
A. Hybrid - keep current structural + new flat legal types
B. Structural - consolidate all into structural categories
C. Flat - expand all to explicit semantic types (~25-30 types)

**Impact Areas:**
- Style template complexity
- Code maintenance burden
- User experience (type selection)
- CSL 1.0 migration path
- Alignment with CSLN principles

**Analysis:** docs/architecture/design/TYPE_SYSTEM_ARCHITECTURE.md

**Decision Criteria:**
1. Is parent-child relationship valuable for academic materials?
2. How important is style template simplicity?
3. Code efficiency vs style authoring clarity priority?
4. Should CSLN match CSL 1.0 type vocabulary?
5. Can we tolerate architectural inconsistency?

**Next Steps:**
1. Review impact analysis
2. Consult with domain experts and style authors
3. Choose option (A, B, or C)
4. If Option C: create migration plan bean
5. Document decision criteria for future types

**Blocking:** Legal citations merge decision (csl26-rmoi)
May impact future type additions and style authoring patterns.
