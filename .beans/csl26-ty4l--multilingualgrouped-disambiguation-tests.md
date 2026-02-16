---
# csl26-ty4l
title: Multilingual/grouped disambiguation tests
status: complete
type: task
priority: high
created_at: 2026-02-16T14:06:53Z
updated_at: 2026-02-16T14:07:09Z
blocked_by:
    - csl26-t1kn
---

Phase 4: Comprehensive test coverage for multilingual + grouped disambiguation.

Test Files:
- crates/csln_processor/tests/disambiguation.rs (verify no regressions)
- crates/csln_processor/tests/multilingual_disambiguation.rs (new)
- crates/csln_processor/tests/grouped_disambiguation.rs (new)

Test Scenarios:
Multilingual (5 tests):
1. Vietnamese given-family order with disambiguation
2. Japanese romanization collision detection
3. Chinese transliteration matching
4. Mixed-script bibliography with year suffix
5. Multilingual given name expansion

Grouped (3 tests):
1. Legal citations with type-order sorting
2. Language-based grouping with per-group suffix restart
3. Combined grouping + multilingual + disambiguation

Acceptance:
- All existing 11 tests pass (no regressions)
- 5 new multilingual disambiguation tests pass
- 3 new grouped disambiguation tests pass

Dependencies: Phases 1-3

Refs: docs/architecture/DISAMBIGUATION_MULTILINGUAL_GROUPING.md Phase 4
