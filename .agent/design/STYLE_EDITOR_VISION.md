# Style Editor Vision

> A user story for a web-based citation style editor built on CSLN.

## Overview

This document captures the vision for a **MakeCSL**-style web editor that allows users to create citation styles through guided, progressive refinement—without needing to understand YAML schemas or citation formatting internals.

**Goal**: Validate that the CSLN model supports this vision and document API requirements.

## Core User Stories

### 1. Style Discovery
Search existing styles by name, field, or citation pattern with autocomplete across ~10,000 migrated styles.

### 2. Guided Creation
Wizard-driven flow:
1. Select type (author-date, numeric, footnote)
2. Enter metadata (title, discipline)
3. **Progressive refinement**: system shows ~5 example citations → user picks closest → system refines → repeat
4. Same process for bibliography

### 3. Field-Specific Examples
Example data sets per academic discipline (law, sciences, humanities) with diverse reference types and edge cases.

### 4. Style Persistence
Export to YAML or JSON; share links. *(User account persistence feasibility TBD)*

## Architecture Decision

| Option | Recommendation |
|--------|----------------|
| Server in this repo | ❌ Scope creep |
| WASM only | 🤔 Viable but limited |
| **Separate repo + published crates** | ✅ Clean separation |

**Core crates** (`csln_core`, `csln_processor`) stay here.  
**Web server** lives in separate deployment-focused repo.

## API Surface Required

```
POST /preview/citation      # Render with style + refs
POST /preview/bibliography
POST /validate/style        # Validate YAML
GET  /schema/options        # Enum values for dropdowns
GET  /examples/:field       # Field-specific references
```

## Current Capability Audit

### ✅ Already Supported
- Declarative options model with JSON Schema
- All processing modes (`author-date`, `numeric`, `note`)
- Full contributor/date/title configuration
- YAML ↔ struct roundtrip with serde

### 🔍 Needs Enhancement
- Add `discipline` and/or `category` fields to `StyleInfo`
- Create example reference datasets per field
- Validate streaming/incremental preview

### ⏳ Future Work
- Note-bibliography mode (processor support)
- Legal citation features
- Per-item multilingual locale

## Open Questions

1. WASM build as prerequisite, or start server-side?
2. PDF extraction worth the complexity?
3. User auth model (OAuth, local storage, etc.)?

## Relevant Links

- [Issue #28: MakeCSL Vision](https://github.com/bdarcus/csln/issues/28)
- [PERSONAS.md](.agent/PERSONAS.md) - stakeholder alignment
- [options.rs](crates/csln_core/src/options.rs) - configuration model

---

> [!NOTE]
> This is a planning document, not a commitment to build the web app in this repo.
