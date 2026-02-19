# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Documentation
- **beans**: track external grouping and heading localization

## [0.5.0] - 2026-02-16

### Features
- **grouping**: implement group disambiguation and bibliography grouping
- **core**: support legal reference conversion (cases, statutes, treaties)
- **processor**: implement advanced selector predicate logic (type, field, and negation)
- **processor**: add HTML output for Djot document processing

### Documentation
- add bibliography grouping design and implementation guide
- document primary/secondary source selection
- update JSON schemas for style and reference models
- add project origins and vision to the landing page

## [0.4.0] - 2026-02-16

### Features
- **test**: add CSL test suite for disambiguation
- **beans**: add deno evaluation and interactive html tasks
- **beans**: add typst output format support
- **infra**: automated versioning infrastructure with release-plz
- **infra**: two-track versioning strategy (code + schema)
- **infra**: initial infrastructure for type-addition policy

### Bug Fixes
- **ci**: remove nextest to avoid yanked dependencies
- **nextest**: correct config field types and remove duplicate profile
- update dependency versions across the workspace

## [0.3.0] - 2026-02-15

### Features
- APA 7th Edition fully validated (5/5 citations + bibliography)
- 11 priority styles implemented (APA, Chicago, Elsevier, Springer, IEEE, etc.)
- EDTF date parser with Level 1 support (uncertainty, approximation, ranges)
- Batch testing framework for corpus analysis across 2,844 legacy styles
- CSL 1.0 to CSLN migration tooling with hybrid strategy
- Structured oracle testing with component-level validation
- Name formatting with initialize-with, name-as-sort-order, et-al rules
- Date formatting with long/short/numeric forms
- Page range formatting (expanded, minimal, chicago)
- Disambiguation support (add-names, add-givenname)
- Type-specific template overrides
- Contributor role substitution
- Small caps font variant support

### Architecture
- Workspace-based crate organization (7 crates)
- Core library (csln_core) with type-safe schema
- Citation processor (csln_processor) with rendering engine
- CLI tools (csln, csln-migrate, csln-analyze)
- Legacy CSL 1.0 parser (csl_legacy)
- CI/CD with fmt/clippy/test validation
- Comprehensive test fixtures and reference data

### Documentation
- Architecture decision records for migration strategy
- Design documents for legal citations, type system, style aliasing
- Rendering workflow guide with oracle testing
- CLAUDE.md project instructions for AI-assisted development
- Persona-driven feature design framework

[Unreleased]: https://github.com/bdarcus/csl26/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/bdarcus/csl26/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/bdarcus/csl26/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/bdarcus/csl26/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bdarcus/csl26/releases/tag/v0.3.0
