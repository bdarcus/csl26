# Implementation Plan: Tier 3 - Numeric Styles & Tooling Improvements

## Overview

Tiers 1 and 2 achieved **100% citation match** for author-date styles and significantly improved bibliography rendering. This tier focuses on:

1. **Numeric style bibliography rendering** (IEEE, Nature, Elsevier Vancouver)
2. **Superscript numeric citations** (Nature, Cell)
3. **Tooling improvements** to accelerate future development

### Current State

| Style | Citations | Bibliography | Impact |
|-------|-----------|--------------|--------|
| **APA** | 15/15 ✅ | 14/15 ✅ | 783 deps |
| **Elsevier Harvard** | 15/15 ✅ | 8/15 ✅ | 665 deps |
| **Chicago Author-Date** | 15/15 ✅ | 6/15 ✅ | ~50 deps |
| ieee | 15/15 ✅ | 0/15 | 176 deps |
| nature | 15/15 ❌ | 0/15 | 72 deps |
| elsevier-vancouver | 15/15 ✅ | 0/15 | 672 deps |
| springer-basic-author-date | 0/15 ❌ | 0/15 | 460 deps |

**Target:** 8/15+ bibliography for IEEE, Nature, Elsevier Vancouver

---

## Lessons Learned from Tiers 1-2

### What Worked Well

1. **Incremental fixes with oracle verification** - Each change was validated against citeproc-js
2. **Type-specific overrides** - Propagating overrides within Lists fixed many ordering issues
3. **Post-processing functions** - `reorder_serial_components()`, `deduplicate_nested_lists()` are composable

### What Was Inefficient

1. **Manual failure inspection** - Comparing strings by eye is slow and error-prone
2. **No structured diff** - Hard to see which *component* is wrong vs just different punctuation
3. **No batch progress tracking** - Can't see if a fix helps 10 styles or just 1
4. **Limited root cause visibility** - CSL → YAML migration is a black box

---

## Proposed Tooling Improvements

### Tool 1: Structured Diff Oracle

**Goal:** Show component-level diffs, not just string diffs.

**Design:**
```bash
# Current output (hard to parse)
Oracle: Smith, J., & Anderson, M. (2020). Nature Climate Change, 10, 850–855.
CSLN:   Smith, J., and Anderson, M. (2020). Nature Climate Change,. , 850–855 10:

# Proposed output (structured)
Entry 1/15: MISMATCH
  contributors: ✓ (order correct, but "and" vs "&")
  date: ✓
  title: ✓
  container-title: ✓
  volume: ✗ (expected "10" after container, got before pages)
  pages: ✗ (missing comma separator)
  issue: EXTRA (should be suppressed for this type)
```

**Implementation:**
- Parse both outputs into structural components (contributor, date, title, etc.)
- Compare each component independently
- Report which component types are failing across the test corpus

### Tool 2: Batch Oracle with Aggregation

**Goal:** Run oracle against many styles, aggregate failures by pattern.

**Design:**
```bash
csln_analyze --oracle-batch styles/ --top 50

# Output:
Tested: 50 styles
Citations: 47/50 at 15/15 (94%)
Bibliography: 12/50 at 8/15+ (24%)

Top failure patterns:
  1. volume/issue ordering (38 styles) - volume appears before container-title
  2. superscript citations (12 styles) - rendering as (Author Year) not superscript
  3. "pp." label missing (28 styles) - pages render without label
  4. "and" conjunction (15 styles) - wrong conjunction for style
```

**Implementation:**
- Run `oracle-e2e.js` for each style, capture JSON output
- Aggregate failures, cluster by pattern using string similarity
- Report top N failure patterns across corpus

### Tool 3: Migration Debugger

**Goal:** Trace how CSL nodes become YAML template components.

**Design:**
```bash
csln_migrate styles/apa.csl --debug-variable volume

# Output:
Variable: volume
Source CSL nodes:
  1. <text variable="volume"/> in macro "label-volume" (line 142)
  2. <text variable="volume"/> in macro "source-serial" (line 187)

Compiled to:
  Template component at index 4 in bibliography.template
  - rendering.prefix: " "
  - rendering.suffix: None
  - overrides: {article-journal: {suppress: false}}

Deduplication: Node 1 merged into Node 2 (same variable)
Ordering: Placed after container-title by reorder_serial_components()
```

**Implementation:**
- Add `--debug-variable VAR` flag to csln_migrate
- Track provenance of each component through compilation
- Output trace showing CSL → intermediate → final YAML

---

## Phase 1: Numeric Style Foundations

### Issue 1.1: Superscript/Bracket Citation Format

**Problem:** Nature uses superscript numbers `¹²³`, not `[1]` or `(Author Year)`.

**Current behavior:**
```
Oracle: 1
CSLN:   (Kuhn 1962)
```

**Root cause:** Citation template not detecting numeric superscript styles correctly.

**Fix:**
- [ ] Detect `<text variable="citation-number"/>` in CSL citation layout
- [ ] Detect `vertical-align="sup"` on number text
- [ ] Set `citation.template` to number-only for numeric styles
- [ ] Handle superscript as rendering option

### Issue 1.2: Bibliography Number Prefix

**Problem:** IEEE/Nature bibliography entries need `[1]` prefix.

**Current behavior:**
```
Oracle: [1]T. S. Kuhn...
CSLN:   [1]T. S. Kuhn...  ✓ (already working)
```

**Status:** Already implemented in Tier 1 for IEEE.

### Issue 1.3: Volume/Issue Ordering for Numeric Styles

**Problem:** Volume appears in wrong position, with wrong punctuation.

**Current behavior:**
```
Oracle: Nature 521, 436–444
CSLN:   Nature,. , 436–444 521:
```

**Root cause:** Same ordering issues as author-date, but with different delimiters (colon vs comma).

**Fix:**
- [ ] Apply `reorder_serial_components()` to numeric styles
- [ ] Extract delimiter from CSL group (comma for Nature, colon for IEEE)
- [ ] Suppress issue for styles that don't show it (Nature)

---

## Phase 2: Style-Specific Fixes

### Issue 2.1: Springer Citation Regression

**Problem:** Springer shows 0/15 citations (regression from earlier work).

**Current behavior:**
```
Oracle: (Smith 2020)
CSLN:   (2018). Journal of...  ← wrong, showing bibliography entry
```

**Root cause:** Unclear - needs debugging. May be citation vs bibliography template confusion.

**Fix:**
- [ ] Debug Springer migration with `--debug` flag
- [ ] Compare citation template generation to APA
- [ ] Verify author-date format extraction

### Issue 2.2: Name Formatting (Initials Without Period)

**Problem:** Some styles want initials without periods.

**Current behavior:**
```
Oracle: Kuhn TS (no periods)
CSLN:   Kuhn, T. S. (with periods)
```

**Root cause:** `initialize-with` extraction not handling empty/space-only values.

**Fix:**
- [ ] Handle `initialize-with=""` (no period after initials)
- [ ] Handle `initialize-with=" "` (space only)
- [ ] Distinguish from `initialize-with="."` (period)

### Issue 2.3: Name Delimiter (No Comma After Family)

**Problem:** Some styles use `Family I` not `Family, I.`

**Current behavior:**
```
Oracle: Smith J, Anderson M
CSLN:   Smith, J, Anderson, M
```

**Root cause:** `sort-separator` attribute not extracted correctly.

**Fix:**
- [ ] Extract `sort-separator=" "` from CSL name element
- [ ] Apply to bibliography contributor config

---

## Phase 3: Conference Paper Template

**Problem:** Conference papers have special format with "In:", "Presented at", "pp."

**Oracle example:**
```
In: Ericsson KA, Charness N, Feltovich PJ, Hoffman RR (eds) 
The Cambridge Handbook of Expertise and Expert Performance. 
Cambridge University Press, pp 683–703
```

**Current CSLN:**
```
in:Ericsson, KA, ... (Eds.). Cambridge University Press, (pp. 683–703).
```

**Issues:**
1. Missing title after editors
2. Wrong punctuation around "pp."
3. "in:" without space

**Fix:**
- [ ] Reorder chapter components: "In:" + editors + title + publisher + pages
- [ ] Extract page label ("pp" vs "pp." vs "p.")
- [ ] Add space after "in:" prefix

---

## Workplan

### Immediate (Tier 3.1)

- [ ] **Implement structured diff oracle** (Tool 1)
- [ ] Fix Nature/Cell superscript citations (Issue 1.1)
- [ ] Fix Springer citation regression (Issue 2.1)
- [ ] Test IEEE bibliography improvements

### Short-term (Tier 3.2)

- [ ] **Implement batch oracle aggregation** (Tool 2)
- [ ] Fix volume/issue ordering for numeric styles (Issue 1.3)
- [ ] Fix name formatting (initials, delimiters) (Issues 2.2, 2.3)
- [ ] Test Nature bibliography

### Medium-term (Tier 3.3)

- [ ] **Implement migration debugger** (Tool 3)
- [ ] Fix conference paper template (Phase 3)
- [ ] Fix APA chapter ordering (remaining 1/15 failure)
- [ ] Test Elsevier Vancouver bibliography

---

## Success Criteria

### Tooling

- [ ] Structured diff oracle identifies specific component failures
- [ ] Batch oracle can test 50+ styles and cluster failures
- [ ] Migration debugger traces variable provenance

### Rendering

| Style | Citation Target | Bib Target |
|-------|----------------|------------|
| IEEE | 15/15 ✅ | 8/15 |
| Nature | 15/15 | 8/15 |
| Elsevier Vancouver | 15/15 ✅ | 8/15 |
| Springer | 15/15 | 5/15 |
| APA | 15/15 ✅ | 15/15 |

**Total Impact:** With these styles, we cover ~70% of dependent styles in the CSL corpus.

---

## Appendix: Failure Pattern Analysis

### IEEE Bibliography Failures (0/15)

| Issue | Count | Example |
|-------|-------|---------|
| Extra quotes around title | 15 | `""Machine Learning""` |
| Volume as "volume15" | 15 | Should be `vol. 15` |
| Issue as "issue11" | 15 | Should be `no. 11` |
| Missing comma before pages | 15 | `volume15. 114042` |
| Date in wrong position | 15 | Should be at end |

### Nature Bibliography Failures (0/15)

| Issue | Count | Example |
|-------|-------|---------|
| Citation format wrong | 15 | Shows `(Author Year)` not `1` |
| Volume/pages wrong order | 15 | Should be `521, 436-444` |
| Italics on journal | 15 | Nature should be italic |
| DOI format | 15 | Missing `doi:` prefix |

### Springer Citation Failures (0/15)

| Issue | Count | Example |
|-------|-------|---------|
| Wrong template used | 15 | Shows bibliography not citation |
| Possible migration bug | ? | Needs debugging |

---

## Architecture Notes

### Why Tooling First?

The remaining rendering issues are increasingly style-specific. Rather than fixing one style at a time, better tooling will:

1. **Identify patterns** - See which issues affect many styles
2. **Prioritize fixes** - Focus on high-impact issues first
3. **Prevent regressions** - Batch testing catches breakage early
4. **Debug faster** - Tracing shows exactly where things go wrong

### Proposed Oracle Output Format

```json
{
  "style": "ieee",
  "citations": {
    "total": 15,
    "passed": 15,
    "failed": 0
  },
  "bibliography": {
    "total": 15,
    "passed": 0,
    "failed": 15,
    "failures": [
      {
        "entry": 1,
        "oracle": "[1]T. S. Kuhn...",
        "csln": "[1]T. S. Kuhn...",
        "components": {
          "number": {"status": "pass"},
          "contributors": {"status": "pass"},
          "title": {"status": "fail", "issue": "extra_quotes"},
          "container": {"status": "pass"},
          "volume": {"status": "fail", "issue": "wrong_format"},
          "pages": {"status": "fail", "issue": "missing_separator"}
        }
      }
    ]
  }
}
```

This structured output enables:
- Automated clustering of failure types
- Tracking improvement over time
- Targeted debugging of specific components
