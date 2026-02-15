---
# csl26-rmoi
title: Add two-tier legal citation support
status: completed
type: feature
priority: normal
created_at: 2026-02-14T22:25:08Z
updated_at: 2026-02-15T00:40:20Z
---

Implement legal reference types as first-class CSLN types with two-tier support model:

**Tier 1 - Core Legal Types (Zero Burden): ✅ COMPLETED**
For academics citing legal materials in APA/Chicago/MLA

* ✅ Add legal-case, statute, treaty, hearing, regulation, brief, classic as core ReferenceType variants
* ✅ Basic fields: title, authority, volume, reporter, page, issued
* ✅ Works out-of-the-box in standard academic styles
* ✅ Test fixtures created (Brown v. Board, Civil Rights Act, Treaty of Versailles)
* ✅ APA 7th override added

**PR #164:** https://github.com/bdarcus/csl26/pull/164
Status: Awaiting review (architectural decision needed)

**Expanded Scope - biblatex Compatibility Types: ✅ COMPLETED**
Based on TYPE_ADDITION_POLICY.md and CURRENT_TYPE_AUDIT.md

* ✅ Patent (high-priority) - inventor, number, authority, jurisdiction
* ✅ Dataset (high-priority) - DOI, publisher, version, format
* ✅ Standard (medium-priority) - authority, number, status
* ✅ Software (medium-priority) - version, repository, license, platform
* ✅ Test fixtures created (U.S. Patent 7,347,809, ODP sediment data, IEEE 754, R 4.1.0)
* ✅ APA 7th overrides added for all types

**Commits:**
1. Core legal types (LegalCase, Statute, Treaty, Hearing, Regulation, Brief, Classic)
2. High-priority scientific types (Patent, Dataset)
3. Medium-priority scientific types (Standard, Software)
4. Test fixtures and APA overrides

**Architectural Decision Made:** csl26-wodz (type system architecture)
Hybrid model (Option A) validated by 4-factor test policy. All current types conform to policy. No revisions needed.

**Tier 2 - Legal Specialist Features (Opt-In): TODO**
For lawyers using Bluebook/ALWD

* Optional specialist fields: jurisdiction (hierarchies), court-class, parallel-first, hereinafter
* Position extensions: far-note, container-subsequent
* Legal-specific template components

**Key Insight:** Legal citations are a spectrum, not binary (lawyer/non-lawyer):

1. Simple academic (APA): Brown v. Board of Education, 347 U.S. 483 (1954)
2. Complex legal (Bluebook short): Brown, 347 U.S. at 495
3. Specialist (Bluebook parallel): Full parallel citation with jurisdiction

Same reference type, different template complexity.

**References:**
* CSL-M legal extensions (PRIOR_ART.md)
* CLAUDE.md Feature Roadmap (Medium priority)
* Domain Expert persona legal checklist

**Deliverables:**
* ✅ Architecture doc: docs/architecture/design/LEGAL_CITATIONS.md
* ✅ Architecture doc: docs/architecture/design/TYPE_SYSTEM_ARCHITECTURE.md
* ✅ Architecture doc: docs/architecture/design/TYPE_ADDITION_POLICY.md
* ✅ Current types audit: docs/architecture/CURRENT_TYPE_AUDIT.md
* ✅ Core legal types in csln_core/src/reference/types.rs (7 types)
* ✅ Scientific types in csln_core/src/reference/types.rs (4 types)
* ✅ Legal type overrides in styles/apa-7th.yaml (proof of concept)
* ✅ Scientific type overrides in styles/apa-7th.yaml
* ✅ Test fixtures for Tier 1 legal types (references-legal.json)
* ✅ Test fixtures for scientific types (references-scientific.json)
* ⏳ Bluebook reference style with specialist features (Tier 2)
* ⏳ Update /styleauthor skill with legal type support
