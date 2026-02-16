---
# csl26-rztf
title: Implement disambiguation test coverage from CSL suite
status: completed
type: feature
priority: high
created_at: 2026-02-16T08:22:16Z
updated_at: 2026-02-16T09:41:26Z
---

**Context**: Disambiguation is implemented but needs comprehensive test coverage from CSL suite.

**Scope**: Extract ~10-15 disambiguation edge cases. Dual-mode test generation.

**Current Branch**: feat/disambiguation-test-coverage

**Completed**:
1. ✅ CSL test suite submodule (tests/csl-test-suite)
2. ✅ Converter script scaffold (tests/fixtures/convert_csl_tests.sh)

**Architecture Decision - Dual Test Modes**:

**Integration Mode** (current): Parse CSL XML → migrate → validate
**Processor Mode** (new): Use native CSLN structs, skip migration

Add --mode flag to update_disambiguation_tests.py

**Implementation Tasks**:

1. Add --mode processor flag to update_disambiguation_tests.py
2. Extract native CSLN structures from compiled Style (skip XML in tests)
3. Serialize CSLN Reference/Style/Citation as inline Rust using serde_json::to_string_pretty
4. Create crates/csln_processor/tests/disambiguation_native.rs for processor-only tests
5. Extract ~10-15 edge cases: year suffix baseline, name expansion priority, given name expansion, combined strategies, missing author fallback
6. Mark with #[ignore] initially
7. Document in docs/DISAMBIGUATION.md

**Assumptions for @builder**:
- CSLN types implement Serialize (confirmed)
- Native tests use #[ignore] initially for baseline
- Integration tests (XML-based) unchanged, new mode additive
- Native mode skips compile_style_from_xml, expects pre-compiled Style
- Use serde_json::to_string_pretty for readable fixtures

**Files**:
- MODIFY: tests/fixtures/update_disambiguation_tests.py
- CREATE: crates/csln_processor/tests/disambiguation_native.rs
- CREATE: docs/DISAMBIGUATION.md
