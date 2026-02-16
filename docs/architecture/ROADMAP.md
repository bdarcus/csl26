# CSLN Project Roadmap

**Last updated:** 2026-02-15
**Purpose:** Strategic plan tracking project maturity, phases, and risks

## Current State Matrix

### Foundation (Complete ✅)

| Component | Status | Coverage | Notes |
|-----------|--------|----------|-------|
| CSL 1.0 Parser | ✅ Complete | 2,844 styles | Full XML parsing capability |
| CSLN Schema | ✅ Complete | - | Style, Template, Options, Locale types |
| Type System | ✅ Designed | - | Hybrid model with 4-factor test policy |
| EDTF Dates | ✅ Complete | - | Range/uncertainty handling |

### Migration Pipeline (Operational ✅)

| Component | Status | Accuracy | Notes |
|-----------|--------|----------|-------|
| XML Options Extraction | ✅ Operational | 87-100% citations | ~2,500 lines, DO NOT TOUCH |
| Output-Driven Templates | ✅ Validated | 95-97% confidence | Tested on 6 styles |
| LLM Hand-Authoring | 🔄 In Progress | 14/15 bibliography (APA) | 10/10 top styles converted |
| Oracle Verification | ✅ Complete | - | Structured diff, batch aggregator |

### Processor (Format-Specific Readiness)

| Format | Citations | Bibliography | Blockers |
|--------|-----------|--------------|----------|
| Author-Date | 9/10 at 15/15 (90%) | 6-14/15 (varies) | Quality refinement needed |
| Numeric | 15/15 (tested 3 styles) | 0/15 | Year positioning, numbering, superscript |
| Note | Not tested | Not tested | Position tracking (ibid, subsequent) |

**Output Formats:** Plain text ✅, HTML ✅, Djot ✅

### Tooling (Optimized ✅)

| Tool | Status | Impact | Notes |
|------|--------|--------|-------|
| Oracle Verification | ✅ Complete | 88% token reduction | Caching, structured diff |
| Workflow Scripts | ✅ Complete | 4 phases validated | prep-migration.sh, workflow-test.sh |
| /styleauthor Skill | ✅ Complete | 18min budget/style | 5-phase iterative workflow |
| Benchmarking | ✅ Available | Opt-in for hot paths | rendering, formats benchmarks |

## Phase Plan

### Phase 1: Author-Date Quality Refinement (Current)
**Target:** 4 styles at 12/15+ bibliography match (~40% dependent corpus)
**Duration:** 2-3 weeks
**Approach:** /styleauthor iteration loops

**Styles:**
1. APA (783 dep) - 14/15 → 15/15
2. Elsevier Harvard (665 dep) - 8/15 → 12/15+
3. Chicago Author-Date (234 dep) - 6/15 → 10/15+
4. Springer Basic Author-Date (460 dep) - Baseline + iterate to 10/15+

**Success Criteria:**
- 4/4 styles at 12/15+ bibliography match
- No citation regression (maintain 15/15)
- Common failure patterns documented
- Workflow optimization insights captured

**Risks:**
- LLM budget overruns if processor features missing
- Variation in style complexity (APA success may not predict others)

### Phase 2: Numeric Style Features (Next)
**Target:** Enable 6 numeric styles (~20% dependent corpus)
**Duration:** 3-4 weeks
**Approach:** Feature implementation + /styleauthor iteration

**Prerequisites:**
- Year positioning fix (affects all numeric styles at 0/15)
- Citation numbering system
- Superscript support
- Sorting templates

**Styles:**
1. Elsevier With-Titles (672 dep)
2. IEEE (176 dep)
3. Elsevier Vancouver (502 dep)
4. American Medical Association (293 dep)
5. Springer Vancouver Brackets (472 dep)
6. Springer Basic Brackets (352 dep)

**Success Criteria:**
- 6/6 styles at 8/15+ bibliography match
- Citation numbering works reliably
- Sorting matches citeproc-js output

**Estimated Effort:** 500-800 lines new code

**Risks:**
- Numeric features more complex than anticipated
- Sorting edge cases (same author, same year, etc.)
- Timeline delay could push 60% coverage goal

### Phase 3: Note Styles (Deferred)
**Target:** 542 note styles (~19% corpus)
**Duration:** TBD
**Approach:** Feature implementation after numeric validation

**Prerequisites:**
- Position tracking (ibid, subsequent, first)
- Note-specific formatting (no parentheses, different delimiters)
- Disambiguation in notes context

**Styles:**
1. Chicago Notes
2. OSCOLA (legal)
3. MHRA

**Decision Point:** Reassess after Phase 2 complete to validate approach repeatability

### Phase 4: Production Readiness (Future)
**Target:** Beta-ready for Zotero/Pandoc integration
**Duration:** TBD

**Features:**
- WASM build (browser/plugin integration)
- JSON server mode (minimize startup latency)
- API stability (versioned schema, migration guide)
- Performance optimization (benchmarks, profiling)

**Prerequisites:**
- 10+ parent styles at 80%+ match
- Comprehensive test coverage
- Documentation complete

## Key Metrics Dashboard

| Metric | Current | Phase 1 Target | Phase 2 Target | Notes |
|--------|---------|----------------|----------------|-------|
| Top-10 coverage | 10/10 (100%) | 10/10 | 10/10 | All converted |
| Top-10 citation quality | 9/10 at 15/15 (90%) | 10/10 at 15/15 | 10/10 at 15/15 | Fix Springer regression |
| Author-date bib quality | 6-14/15 (varies) | 12-15/15 (4 styles) | - | Quality refinement |
| Numeric bib quality | 0/15 (blocked) | - | 8-12/15 (6 styles) | After feature work |
| Dependent corpus coverage | 4,792/7,987 (60%) | ~3,200/7,987 (40%) | ~4,800/7,987 (60%) | Top-10 styles |
| Bean health | Stale statuses | All accurate | All accurate | Audit 2026-02-15 |

## Risk Register

### High Priority Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Numeric timeline delay | 60% coverage goal delayed | Medium | Focus Phase 1 on author-date only, prove repeatability first |
| LLM budget overruns | Extended iteration time | Medium | Document patterns, optimize workflow, use @builder for processor features |
| Perception gap | Marketing as "CSL replacement" but note styles deferred | Medium | Transparent roadmap, focus on 60% corpus coverage first |
| Springer regression | Citation quality drop | Low | Tracked, isolated to single style, fix in Phase 1 |

### Medium Priority Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Workflow optimization not transferable | APA success doesn't predict other styles | Medium | Test against diverse styles (Elsevier, Chicago, Springer) |
| Numeric features more complex | 500-800 line estimate too low | Low | Incremental implementation, cargo test guard |
| Note style complexity | Position tracking harder than anticipated | Low | Defer to Phase 3, reassess after numeric validation |

## Workflow Optimization Notes

### What Works (APA 14/15 Success)
- 5-phase /styleauthor workflow (research → author → test → evolve → verify)
- Structured oracle comparison (component-level diff)
- Iterative refinement with processor evolution
- Reusable pattern capture in common-patterns.yaml

### What Needs Improvement
- Numeric style support (0/15 bibliography across all numeric styles)
- Workflow budget optimization (actual time vs 18min target)
- Failure pattern documentation (systematic categorization)
- Cross-style consistency (delimiter variations, volume/issue formatting)

### Common Failure Modes
1. **Year positioning** - Numeric styles show year in wrong position
2. **Volume/issue spacing** - "2, (2)" vs "2(2)" inconsistency
3. **Editor labels** - Capitalization and punctuation variations
4. **Page delimiters** - Comma vs colon varies by style
5. **DOI suppression** - Some styles don't output DOI

## Decision Log

### 2026-02-15: Hybrid Migration Strategy Validated
**Decision:** Use XML options + output-driven templates + LLM authoring
**Rationale:** XML excels at options (87-100% cit), fails at templates (0% bib). LLM authoring achieves 14/15 bibliography for APA.
**Refs:** ./architecture/MIGRATION_STRATEGY_ANALYSIS.md, bean csl26-m3lb

### 2026-02-08: Defer Note Styles to Phase 3
**Decision:** Focus author-date (40% corpus) then numeric (20% corpus) before note (2% corpus)
**Rationale:** Maximize coverage with proven approach before tackling position tracking complexity.
**Refs:** ./../reference/STYLE_PRIORITY.md, bean csl26-5t6s

### 2026-02-08: Type System Architecture Finalized
**Decision:** Hybrid model (structural for academic, flat for legal/domain-specific)
**Rationale:** Balances data efficiency with style clarity using 4-factor test.
**Refs:** ./architecture/design/TYPE_SYSTEM_ARCHITECTURE.md, ./architecture/design/TYPE_ADDITION_POLICY.md

## References

- **Architecture Docs:** ./architecture/
- **Bean Tracker:** ../.beans/
- **Style Priority:** ./../reference/STYLE_PRIORITY.md
- **Migration Analysis:** ./architecture/MIGRATION_STRATEGY_ANALYSIS.md
- **Workflow Guide:** ./../guides/RENDERING_WORKFLOW.md
