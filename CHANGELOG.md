# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features
- Automated versioning infrastructure with release-plz
- Two-track versioning strategy (code + schema)

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
