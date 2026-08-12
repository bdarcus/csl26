# Specifications

Feature and design specifications for non-trivial Citum capabilities.
A spec captures the *what* and *why* before implementation begins.

## Spec Template

Copy this template when creating a new spec:

```markdown
# [Feature Name] Specification

**Status:** Draft | Active | Completed | Superseded
**Date:** YYYY-MM-DD
**Supersedes:** (optional path, if any)
**Related:** (policy, bean, or issue)

## Purpose
One paragraph: what feature this specifies and why.

## Scope
In scope. Explicitly out of scope.

## Design
(Core content — decisions, data models, examples.)

## Implementation Notes
(Non-normative hints, known constraints.)

## Acceptance Criteria
- [ ] Verifiable condition 1
- [ ] Verifiable condition 2

## Changelog
- DATE: Initial version.
```

## Workflow

Before creating or converting a spec, consult
[`../guides/DOCUMENT_CLASSIFICATION.md`](../guides/DOCUMENT_CLASSIFICATION.md)
to make sure the document should be a spec rather than architecture or policy.

1. Create a spec file in `docs/specs/` (Status: `Draft`) **before** writing
   implementation code.
2. Commit the spec. Get it merged.
3. Set Status to `Active` in the same commit as the first implementation.
4. Reference the spec path in the bean description.

## Using This Index

The **Specs** section below is organized by capability area. Each row includes:

- **Spec** — links directly to the spec file.
- **Status** — `Draft` / `Active` / `Completed` / `Superseded`.
- **Tests** — the test file or `file::module` that exercises this capability. Use
  these anchors to direct an agent: *"review `citations.rs::disambiguation` against
  `DISAMBIGUATION.md`"*.

Specs covering multiple areas are listed under their primary area with a *see also*
note. The `—` marker in the Tests column means no targeted test exists yet.

---

## Specs

### Citations

| Spec | Status | Tests |
|------|--------|-------|
| [`DISAMBIGUATION.md`](./DISAMBIGUATION.md) — collision-key model, strategy cascade, multilingual/group disambiguation, APA §8.15 reprint keying | Active | `citations.rs::disambiguation` |
| [`DATE_SUBSTITUTE.md`](./DATE_SUBSTITUTE.md) — preset or type-scoped fallback policy for the identity date slot | Draft | — |
| [`CITE_SITE_COMPOUND_GROUPING.md`](./CITE_SITE_COMPOUND_GROUPING.md) — cite-site compound grouping and position-aware overrides | Active | `citations.rs::sorting_and_grouping` |
| [`INTEGRAL_NAME_MEMORY.md`](./INTEGRAL_NAME_MEMORY.md) — durable author-display state across citation clusters | Active | `citations.rs::integral_name_memory` |
| [`PERSONAL_COMMUNICATION_CITATION.md`](./PERSONAL_COMMUNICATION_CITATION.md) — rendering of personal communications across style families | Active | `citations.rs::contributor_scoping` |
| [`LEGAL_CITATIONS.md`](./LEGAL_CITATIONS.md) — legal-case type support and jurisdiction-aware rendering | Draft | — |
| [`LOCATOR_RENDERING.md`](./LOCATOR_RENDERING.md) — style-level LocatorConfig replacing per-template locator fields | Active | `citations.rs` |
| [`NON_STANDARD_NUMBERING_AND_LOCATOR_KINDS.md`](./NON_STANDARD_NUMBERING_AND_LOCATOR_KINDS.md) — domain-specific numbering and locator labels | Active | `citations.rs` |
| [`REFERENCE_MARKERS.md`](./REFERENCE_MARKERS.md) — processor-owned numeric and label markers; removes label components from templates | Active | `citations.rs` |

### Bibliography

| Spec | Status | Tests |
|------|--------|-------|
| [`ANNOTATED_BIBLIOGRAPHY.md`](./ANNOTATED_BIBLIOGRAPHY.md) — document-scoped annotation overlay for bibliography rendering | Active | `bibliography.rs::annotated_html_preview`, `document.rs` |
| [`BIBLIOGRAPHY_GROUPING.md`](./BIBLIOGRAPHY_GROUPING.md) — grouped bibliography architecture: group specs, heading generation, nested groups | Active | `document.rs::grouped_bibliography` |
| [`ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md`](./ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md) — external bibliography parameter for article-journal page fallback | Active | `bibliography.rs::article_journal_no_page_fallback` |
| [`CITATION_BIBLIOGRAPHY_OPTION_SPLIT.md`](./CITATION_BIBLIOGRAPHY_OPTION_SPLIT.md) — strict schema split for citation and bibliography option scopes | Active | `bibliography.rs`, `citations.rs` |
| [`INLINE_JOURNAL_DETAIL_GROUPING.md`](./INLINE_JOURNAL_DETAIL_GROUPING.md) — inline article-journal detail blocks with mixed delimiters | Active | `bibliography.rs` |
| [`CROSS_ROLE_CONTRIBUTOR_LISTS.md`](./CROSS_ROLE_CONTRIBUTOR_LISTS.md) — merged multi-role name lists with per-name/collective labels and same-person combination | Active | `cross_role_contributors.rs` |
| [`PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`](./PRIMARY_CONTRIBUTOR_SUBSTITUTION.md) — type-aware scalar and merged contributors promoted into the primary slot | Active | `cross_role_contributors.rs` |
| [`ROLE_SUBSTITUTE_FALLBACK.md`](./ROLE_SUBSTITUTE_FALLBACK.md) — normative behavior for role-aware contributor fallback chains | Active | `bibliography.rs::substitution` |
| [`SECONDARY_CONTRIBUTOR_ROLE_FORMATTING.md`](./SECONDARY_CONTRIBUTOR_ROLE_FORMATTING.md) — consistent rendering and verification for secondary contributor roles | Active | `bibliography.rs` |

### Note Styles & Repeated Citations

| Spec | Status | Tests |
|------|--------|-------|
| [`NOTE_SHORTENING_POLICY.md`](./NOTE_SHORTENING_POLICY.md) — normative contract for repeated-note and shortened-note behavior | Active | `citations.rs::note_style_positions` |
| [`NOTE_START_REPEATED_NOTE_POLICY.md`](./NOTE_START_REPEATED_NOTE_POLICY.md) — repeated-note behavior at note start vs. internal positions | Active | `citations.rs::note_style_positions` |
| [`NOTE_STYLE_DOCUMENT_NOTE_CONTEXT.md`](./NOTE_STYLE_DOCUMENT_NOTE_CONTEXT.md) — document-level note processing and note-in-note participating | Active | `document.rs::note_flow` |
| [`REPEATED_NOTE_CITATION_STATE_MODEL.md`](./REPEATED_NOTE_CITATION_STATE_MODEL.md) — style-driven repeated-citation model for note processing | Active | `citations.rs::note_style_positions` |
| [`NOTE_POSITION_AUDIT.md`](./NOTE_POSITION_AUDIT.md) — audit layer for note-style repeated-citation behavior | Active | `citations.rs::note_style_positions` |

### Sorting

| Spec | Status | Tests |
|------|--------|-------|
| [`SORTING.md`](./SORTING.md) — end-to-end sort semantics: citation vs bibliography, sort keys, presets, collation, tiebreaking | Active | `sort_oracle.rs`, `bibliography.rs::sorting` |
| [`EXPLICIT_DEFAULT_SORTING.md`](./EXPLICIT_DEFAULT_SORTING.md) — processing-family bibliography defaults; `citation.sort` explicit-only policy | Draft | `sort_oracle.rs` |
| [`UNICODE_BIBLIOGRAPHY_SORTING.md`](./UNICODE_BIBLIOGRAPHY_SORTING.md) — locale-aware UCA/CLDR collation for bibliography sort keys | Active | `sort_oracle.rs` |
| [`MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md) — sort and section multilingual bibliographies by script or language | Active | `sort_oracle.rs`, `multilingual.rs` |
| [`MULTILINGUAL_SORTING.md`](./MULTILINGUAL_SORTING.md) — `options.sorting` multilingual sort modes and hidden `sort-as` romanized sort keys | Draft | — |

### Localization & Multilingual

| Spec | Status | Tests |
|------|--------|-------|
| [`MULTILINGUAL.md`](./MULTILINGUAL.md) — multilingual data model, processor logic, sorting, disambiguation *(umbrella)* | Active | `multilingual.rs`, `i18n.rs::multilingual_rendering` |
| [`LOCALE_MESSAGES.md`](./LOCALE_MESSAGES.md) — ICU MF2 parameterized message system replacing flat YAML terms | Active | `i18n.rs::string_resolution`, `i18n.rs::config` |
| [`CONTRIBUTOR_PHRASE_MESSAGES.md`](./CONTRIBUTOR_PHRASE_MESSAGES.md) — locale-owned contributor phrase messages for role/name/title ordering | Draft | — |
| [`GENDERED_LOCALE_TERMS.md`](./GENDERED_LOCALE_TERMS.md) — multi-dimensional locale terms with grammatical gender support | Active | `i18n.rs::string_resolution` |
| [`MULTILINGUAL_NAMES.md`](./MULTILINGUAL_NAMES.md) — script-specific contributor name assembly | Active | `multilingual_names.rs`, `i18n.rs::name_resolution` |
| [`SENTENCE_INITIAL_LABELS.md`](./SENTENCE_INITIAL_LABELS.md) — sentence-initial capitalization for localized labels | Active | `i18n.rs` |
| [`EDTF_ERA_LABEL_PROFILES.md`](./EDTF_ERA_LABEL_PROFILES.md) — era label profiles and unspecified historical-year display | Active | `metadata.rs` |
| [`EDTF_HISTORICAL_ERA_RENDERING.md`](./EDTF_HISTORICAL_ERA_RENDERING.md) — locale-backed rendering of valid historical EDTF years | Active | `metadata.rs` |

### Data Model & Types

| Spec | Status | Tests |
|------|--------|-------|
| [`TYPE_REFACTOR_v3.md`](./TYPE_REFACTOR_v3.md) — unified type system refactor for high-fidelity work modeling | Active | `metadata.rs`, `domain_fixtures.rs` |
| [`TYPE_SYSTEM_ARCHITECTURE.md`](./TYPE_SYSTEM_ARCHITECTURE.md) — overall type system architecture and classification hierarchy | Draft | `crates/citum-schema/tests/` |
| [`TYPED_TITLE_MAPPING.md`](./TYPED_TITLE_MAPPING.md) — normalized reference-type keys and closed title-rendering categories | Active | `crates/citum-schema-style/src/tests.rs`, `citum-engine::render::component` |
| [`INPUT_REFERENCE_CLASS_DISCRIMINATOR.md`](./INPUT_REFERENCE_CLASS_DISCRIMINATOR.md) — discriminated union for reference-class dispatch | Active | `crates/citum-schema/tests/` |
| [`CSL_TYPE_CONVERSION_CONTRACT.md`](./CSL_TYPE_CONVERSION_CONTRACT.md) — CSL 1.0.2 type vocabulary, routing closure, and note-override validation | Active | `crates/citum-schema-data/src/reference/conversion/contract_tests.rs` |
| [`GENERALIZED_RELATIONAL_CONTAINER_MODEL.md`](./GENERALIZED_RELATIONAL_CONTAINER_MODEL.md) — recursive relational container model replacing flat variables | Active | `metadata.rs` |
| [`DATE_MODEL.md`](./DATE_MODEL.md) — refined date model for created vs. issued distinction | Active | `metadata.rs` |
| [`EDTF_DATE_RANGE_FORMATTING.md`](./EDTF_DATE_RANGE_FORMATTING.md) — style-configurable condensed EDTF year ranges | Active | `date.rs`, `metadata.rs` |
| [`CALENDAR_DATE_ANNOTATIONS.md`](./CALENDAR_DATE_ANNOTATIONS.md) — EDTF dates with an optional, style-controlled opaque note | Active | `crates/citum-schema-data/src/reference/date.rs`, `crates/citum-engine/tests/date_annotations.rs` |
| [`NUMBERING_SEMANTICS.md`](./NUMBERING_SEMANTICS.md) — canonical semantics for numbering, report, and part fields | Active | `metadata.rs` |
| [`ARCHIVAL_UNPUBLISHED_SUPPORT.md`](./ARCHIVAL_UNPUBLISHED_SUPPORT.md) — ArchiveInfo and EprintInfo first-class source support | Active | `metadata.rs` |
| [`ORIGINAL_PUBLICATION_RELATION_SUPPORT.md`](./ORIGINAL_PUBLICATION_RELATION_SUPPORT.md) — universal original publication metadata support across all types | Active | `metadata.rs` |
| [`STRONG_DOMAIN_TYPES_PHASE1.md`](./STRONG_DOMAIN_TYPES_PHASE1.md) — replacing primitive String aliases with dedicated domain types | Active | `crates/citum-schema/tests/` |

### Text & Rendering

| Spec | Status | Tests |
|------|--------|-------|
| [`TEMPLATE_V2.md`](./TEMPLATE_V2.md) — simplified Template Schema v2 with group-first composition | Active | `bibliography.rs`, `citations.rs` |
| [`TEMPLATE_V3.md`](./TEMPLATE_V3.md) — Template Schema v3 extensions | Active | `bibliography.rs`, `citations.rs` |
| [`TEMPLATE_RENDERING_SEMANTICS.md`](./TEMPLATE_RENDERING_SEMANTICS.md) — render-time variable-once and group consumption semantics | Active | `bibliography.rs`, `citations.rs` |
| [`RENDER_WHEN_CONTRACT.md`](./RENDER_WHEN_CONTRACT.md) — specified retention contract for template field-presence conditions | Active | `crates/citum-schema-style/src/tests.rs`, `bibliography.rs` |
| [`TITLE_TEXT_CASE.md`](./TITLE_TEXT_CASE.md) — modeling and applying title-like text-case transformations | Active | `bibliography.rs` |
| [`TITLE_NAME_INFLECTION.md`](./TITLE_NAME_INFLECTION.md) — grammatical inflection of title-adjacent names | Active | `bibliography.rs` |
| [`PUNCTUATION_NORMALIZATION.md`](./PUNCTUATION_NORMALIZATION.md) — normalization of punctuation at rendered output boundaries | Draft | `bibliography.rs`, `citations.rs` |
| [`DJOT_RICH_TEXT.md`](./DJOT_RICH_TEXT.md) — Djot as the rich-text markup substrate for note/abstract fields | Active | `document.rs::djot_adapter` |
| [`SHORT_NAME.md`](./SHORT_NAME.md) — short-name rendering for abbreviated contributor identifiers | Active | `bibliography.rs::title_short_resolution` |
| [`ABBREVIATION_MAP.md`](./ABBREVIATION_MAP.md) — abbreviation lookup map for journal and publisher names | Active | `bibliography.rs` |

### Document & Input

| Spec | Status | Tests |
|------|--------|-------|
| [`DOCUMENT_INPUT_PARSER_BOUNDARY.md`](./DOCUMENT_INPUT_PARSER_BOUNDARY.md) — boundary between format parsers and shared processing pipeline | Active | `document.rs`, `io.rs` |
| [`PANDOC_MARKDOWN_CITATIONS.md`](./PANDOC_MARKDOWN_CITATIONS.md) — Pandoc-style citation marker support for Markdown documents | Completed | `document.rs::markdown_documents` |
| [`FIDELITY_RICH_INPUTS.md`](./FIDELITY_RICH_INPUTS.md) — fidelity pipeline support for relational benchmark inputs | Active | `document.rs`, `io.rs` |
| [`FORWARD_COMPATIBILITY.md`](./FORWARD_COMPATIBILITY.md) — forward-compatibility guarantees across schema versions | Active | `forward_compatibility.rs` |
| [`SERVER_INTERACTIVE_API.md`](./SERVER_INTERACTIVE_API.md) — document-batch and session APIs for server and WASM | Draft | `crates/citum-server/tests/rpc.rs` |

### Migration (CSL → Citum)

| Spec | Status | Tests |
|------|--------|-------|
| [`MIGRATE_RESEARCH_RICH_INPUTS.md`](./MIGRATE_RESEARCH_RICH_INPUTS.md) — bounded rich-input workflow for migrate-research passes | Active | `crates/citum-migrate/tests/` |
| [`MIGRATION_TAXONOMY_AWARE_WRAPPERS.md`](./MIGRATION_TAXONOMY_AWARE_WRAPPERS.md) — taxonomy-aware wrapper derivation during style migration | Active | `crates/citum-migrate/tests/` |
| [`EMBEDDED_ROOT_WRAPPER_MIGRATION.md`](./EMBEDDED_ROOT_WRAPPER_MIGRATION.md) — proof-gated embedded roots plus thin public wrappers for style migration | Active | `crates/citum-migrate/tests/` |
| [`MIXED_CONDITION_NOTE_POSITION_TREES.md`](./MIXED_CONDITION_NOTE_POSITION_TREES.md) — migration of legacy choose trees with mixed position predicates | Active | `crates/citum-migrate/tests/` |
| [`EMBEDDED_JS_TEMPLATE_INFERENCE.md`](./EMBEDDED_JS_TEMPLATE_INFERENCE.md) — embedded JS runtime for live template inference in migrator | Active | `crates/citum-migrate/tests/` |
| [`OUTPUT_DRIVEN_TEMPLATE_SYNTHESIS.md`](./OUTPUT_DRIVEN_TEMPLATE_SYNTHESIS.md) — measured candidate selection for migrated templates | Active | `crates/citum-migrate/tests/` |
| [`ENGINE_MIGRATE_COEVOLUTION_WAVE.md`](./ENGINE_MIGRATE_COEVOLUTION_WAVE.md) — engine-first co-evolution wave for style-fidelity fixes | Active | — |

### Style System & Configuration

| Spec | Status | Tests |
|------|--------|-------|
| [`STYLE_PRESET_ARCHITECTURE.md`](./STYLE_PRESET_ARCHITECTURE.md) — two-level configuration reuse for bases and profiles | Active | `crates/citum-schema-style/tests/` |
| [`STYLE_REGISTRY.md`](./STYLE_REGISTRY.md) — serde-driven StyleRegistry replacing hardcoded slices | Active | `crates/citum-schema-style/tests/` |
| [`STYLE_TAXONOMY.md`](./STYLE_TAXONOMY.md) — Citum style taxonomy based on semantic class and implementation | Active | `crates/citum-schema-style/tests/` |
| [`STYLE_EDITIONS_AND_FAMILIES.md`](./STYLE_EDITIONS_AND_FAMILIES.md) — style edition and family versioning model | Active | `crates/citum-schema-style/tests/` |
| [`STYLE_ALIASING.md`](./STYLE_ALIASING.md) — alias/discovery layer for style identity resolution | Active | `crates/citum-schema-style/tests/` |
| [`STYLE_COMPATIBILITY_INHERITANCE_REPORT.md`](./STYLE_COMPATIBILITY_INHERITANCE_REPORT.md) — family-grouped compatibility and exact-text parity reporting | Active | `scripts/oracle.test.js`, `scripts/lib/report-inheritance.test.js`, `scripts/report-core.test.js` |
| [`APA_SQI_ALIGNMENT_AND_PRESET_REFACTOR.md`](./APA_SQI_ALIGNMENT_AND_PRESET_REFACTOR.md) — APA SQI alignment and preset-first cleanup | Active | `bibliography.rs`, `citations.rs` |
| [`CHICAGO_18_COVERAGE.md`](./CHICAGO_18_COVERAGE.md) — Chicago 18th and APA 8th high-fidelity coverage enhancement | Active | `bibliography.rs`, `citations.rs` |
| [`CHICAGO_VARIANT_AXES.md`](./CHICAGO_VARIANT_AXES.md) — mapping `style-variant-builder`'s template-diff model onto Citum `extends:`/type-variant inheritance | Draft | `crates/citum-schema-style/tests/bdd_inheritance.rs` |
| [`STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md`](./STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md) — bounded template reuse, inherited fallback diffs, and auditable field coverage | Active | `crates/citum-schema-style/tests/bdd_inheritance.rs` |
| [`UNIFIED_SCOPED_OPTIONS.md`](./UNIFIED_SCOPED_OPTIONS.md) — typed scoped options replacing flat author-facing contracts | Active | `crates/citum-schema-style/tests/` |
| [`PER_DOCUMENT_CONFIG_OVERRIDES.md`](./PER_DOCUMENT_CONFIG_OVERRIDES.md) — eligible options and syntax for per-document configuration overrides | Draft | `bibliography.rs::local_overrides` |
| [`SCHEMA_SPLIT_AND_CONVERT_NAMESPACE.md`](./SCHEMA_SPLIT_AND_CONVERT_NAMESPACE.md) — crate-level schema split and CLI conversion namespace | Active | `crates/citum-schema/tests/` |
| [`PROFILE_DOCUMENTARY_VERIFICATION.md`](./PROFILE_DOCUMENTARY_VERIFICATION.md) — verification model for profile styles using external authority | Draft | — |
| [`JOURNAL_PROFILE_TAXONOMY_AUDIT.md`](./JOURNAL_PROFILE_TAXONOMY_AUDIT.md) — audit of journal-profile taxonomy and authority rules | Completed | — |
| [`CONFIG_ONLY_PROFILE_OVERRIDES.md`](./CONFIG_ONLY_PROFILE_OVERRIDES.md) — alternative configuration-only profile override model | Superseded | — |

### Platform & Infra

| Spec | Status | Tests |
|------|--------|-------|
| [`LANGUAGE_BINDINGS.md`](./LANGUAGE_BINDINGS.md) — multi-language type bindings for canonical data shapes | Active | `crates/citum-bindings/tests/` |
| [`WASM_SUPPORT.md`](./WASM_SUPPORT.md) — WASM build targets and embedding model | Active | — |
| [`DISTRIBUTED_RESOLVER.md`](./DISTRIBUTED_RESOLVER.md) — federated registry and distributed resolver architecture (Phases 2–3) | Active | — |
| [`CLI_UX_REDESIGN.md`](./CLI_UX_REDESIGN.md) — clean command model for style discovery, registry management, and CLI validation UX | Active | — |
| [`AUTHORING_AGENT_SKILL.md`](./AUTHORING_AGENT_SKILL.md) — Citum Authoring Agent Skill specification for AI-assisted style authoring | Active | — |
| [`REPO_LOCAL_HARNESS.md`](./REPO_LOCAL_HARNESS.md) — repo-owned control surfaces for Claude/Codex workflow and skill boundaries | Active | — |
| [`EXPLICIT_RENDER_RUN_STATE.md`](./EXPLICIT_RENDER_RUN_STATE.md) — typed per-run state (`RunState`/`FinalizedRun`) replacing `Processor`'s `RefCell` fields | Active | `processor/tests.rs` |
