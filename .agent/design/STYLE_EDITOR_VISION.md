# Citation Style Platform Vision

> A user story for a web-based citation style platform built on CSLN.

## Overview

This document captures the vision for a comprehensive **Citation Style Platform**. The primary goal is to **solve the user's citation formatting problem with the least amount of friction**.

This represents a shift from just an "editor" to a platform where the hierarchy of needs is:
1.  **Find** an existing style (Discovery).
2.  **Tweak** a close match (Modification).
3.  **Build** from scratch (Creation).

**Goal**: Validate that the CSLN model supports this vision and document API requirements.

## Core User Stories

### 1. Style Discovery & Usage (The "App Store" for Styles)
*Primary workflow for 90% of users.*

- **Search**: By name ("Nature"), field ("Neuroscience"), or generic type ("Author-Date").
- **Search by Example**: User pastes a formatted citation (e.g., "Doe, J. (2020)..."), system finds styles that match this output.
- **One-Click Use**: "Copy CSL JSON", "Download CSL", or "Get ID" for use in Zotero, Mendeley, Pandoc, etc.

### 2. Smart Modification ("Like X but...")
*Secondary workflow for minor adjustments.*

- User starts with a discovered style (e.g., "APA").
- "I want APA but with square brackets."
- "Use Harvard but force et-al after 3 authors."
- System creates a style using relevant presets or configuration override rather than a full independent style fork where possible.

### 3. Guided Creation (The Wizard)
*Fallback workflow for completely new requirements.*

- Wizard-driven flow:
    1. Select type (author-date, numeric, footnote)
    2. Enter metadata (title, discipline)
    3. **Progressive refinement**: system shows ~5 example citations → user picks closest → system refines → repeat
    4. Same process for bibliography

### 4. Field-Specific Examples
Example data sets per academic discipline (law, sciences, humanities) with diverse reference types and edge cases to validate styles during discovery and editing.

### 5. Style Persistence
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
POST /search/match          # Search styles by example citation input
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
- **Implement "Reverse Match" logic for example search**

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
- [Style Editor Repository](https://github.com/bdarcus/style-editor) - The web platform companion to this repository
- [PERSONAS.md](.agent/PERSONAS.md) - stakeholder alignment
- [options.rs](crates/csln_core/src/options.rs) - configuration model

---

> [!NOTE]
> This is a planning document, not a commitment to build the web app in this repo.
