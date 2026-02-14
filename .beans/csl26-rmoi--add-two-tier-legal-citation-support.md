---
# csl26-rmoi
title: Add two-tier legal citation support
status: todo
type: feature
priority: normal
created_at: 2026-02-14T22:25:08Z
updated_at: 2026-02-14T22:25:08Z
---

Implement legal reference types as first-class CSLN types with two-tier support model:

**Tier 1 - Core Legal Types (Zero Burden):**
For academics citing legal materials in APA/Chicago/MLA
- Add legal-case, statute, treaty, hearing, regulation, brief, classic as core ReferenceType variants
- Basic fields: title, authority, volume, reporter, page, issued
- Works out-of-the-box in standard academic styles

**Tier 2 - Legal Specialist Features (Opt-In):**
For lawyers using Bluebook/ALWD
- Optional specialist fields: jurisdiction (hierarchies), court-class, parallel-first, hereinafter
- Position extensions: far-note, container-subsequent
- Legal-specific template components

**Key Insight:**
Legal citations are a spectrum, not binary (lawyer/non-lawyer):
1. Simple academic (APA): Brown v. Board of Education, 347 U.S. 483 (1954)
2. Complex legal (Bluebook short): Brown, 347 U.S. at 495
3. Specialist (Bluebook parallel): Full parallel citation with jurisdiction

Same reference type, different template complexity.

**References:**
- CSL-M legal extensions (PRIOR_ART.md)
- CLAUDE.md Feature Roadmap (Medium priority)
- Domain Expert persona legal checklist

**Deliverables:**
- Architecture doc: docs/architecture/design/LEGAL_CITATIONS.md
- Core legal types in csln_core/src/reference/types.rs
- Legal type overrides in styles/apa-7th.yaml (proof of concept)
- Bluebook reference style with specialist features
- Test fixtures for both tiers
- Update /styleauthor skill with legal type support
