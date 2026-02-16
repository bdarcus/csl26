---
# csl26-2wk4
title: 'Phase 1: Wire Disambiguator into Processor'
status: completed
type: task
priority: high
created_at: 2026-02-16T12:18:42Z
updated_at: 2026-02-16T12:18:42Z
blocking:
    - csl26-h2b0
---

Integrate Disambiguator::calculate_hints() into Processor::process_citation().

**Implementation**: See DISAMBIGUATION_IMPLEMENTATION_PLAN.md Phase 1

**Tasks**:
1. Add disambiguator field to ProcessorState
2. Call calculate_hints() after initial rendering pass
3. Pass hints to second rendering pass via ProcHints
4. Update render_citation_item() signature

**Acceptance Criteria**:
- Disambiguator logic executes during citation processing
- ProcHints populated with year suffixes and name expansion hints
- No change to output yet (renderers don't consume hints)

**Effort**: 2-3 hours

**@builder candidate**: Yes - well-defined integration task
