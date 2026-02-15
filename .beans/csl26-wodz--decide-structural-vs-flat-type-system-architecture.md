---
# csl26-wodz
title: 'Decide: structural vs flat type system architecture'
status: completed
type: feature
priority: high
created_at: 2026-02-14T22:53:24Z
updated_at: 2026-02-15T00:18:09Z
---

Architectural decision triggered by legal citations PR #164.

**DECISION: Option A - Hybrid Model with Type Addition Policy**

**Question:** Should CSLN use structural types (Monograph, SerialComponent),
flat types (JournalArticle, MagazineArticle, LegalCase), or hybrid?

**Answer:** Hybrid approach with documented 4-factor test policy.

**Rationale:**
1. Data efficiency - Parent-child relationships reduce duplication for academic refs
2. Style clarity - Flat types enable explicit overrides for legal/domain refs
3. User alignment - Semantic type names match mental models
4. CSL 1.0 compatibility - Flat types map 1:1, structural map many:1

**4-Factor Test for New Types:**
1. Semantic distinction - users think of it differently
2. Style discrimination - 20%+ of major styles format differently
3. Field schema difference - 3+ unique required/expected fields
4. No meaningful parent - container is locator, not semantic

**Pass all 4 → Add flat type**
**Fail 3-4 → Use structural type** (efficiency > template complexity)

**Documentation:**
✅ TYPE_SYSTEM_ARCHITECTURE.md - full analysis, recommendation
✅ TYPE_ADDITION_POLICY.md - active policy with decision criteria
✅ CLAUDE.md - updated with policy links

**Impact:**
- No breaking changes (current types validated against policy)
- Legal citations PR #164 proceeds (LegalCase, Treaty, etc. pass 4-factor test)
- Future type additions use documented policy

**Follow-up:**
- Bean csl26-piea: Implement policy infrastructure (PR template, examples, audit)

**References:**
- TYPE_SYSTEM_ARCHITECTURE.md
- TYPE_ADDITION_POLICY.md
- biblatex (31 flat types) - better prior art than CSL 1.0 hierarchy
- Legal citations: csl26-rmoi, PR #164
