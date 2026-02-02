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

## Tooling Improvements

### Tool 1: Structured Diff Oracle ✅ IMPLEMENTED

**Location:** `scripts/oracle-structured.js`

**Usage:**
```bash
node scripts/oracle-structured.js styles/apa.csl           # Human-readable
node scripts/oracle-structured.js styles/apa.csl --verbose # With detailed failures
node scripts/oracle-structured.js styles/apa.csl --json    # Machine-readable
```

**Features:**
- Parses bibliography entries into structural components (contributors, year, title, volume, etc.)
- Compares components independently between oracle and CSLN
- Detects ordering differences (e.g., year before title vs after)
- Aggregates component issues across entries
- Identifies missing/extra components

**Example output:**
```
--- COMPONENT ISSUES ---
  containerTitle:missing: 1 entries
  year:extra: 5 entries

--- ORDERING ISSUES: 10 entries ---

--- DETAILED FAILURES ---
Entry 1:
  Order Oracle: contributors → title → containerTitle → volume → year
  Order CSLN:   contributors → year → title → containerTitle
  Issue: ordering: Component order differs
```

### Tool 2: Batch Oracle Aggregator ✅ IMPLEMENTED

**Location:** `scripts/oracle-batch-aggregate.js`

**Usage:**
```bash
node scripts/oracle-batch-aggregate.js styles/ --top 10     # Top 10 priority styles
node scripts/oracle-batch-aggregate.js styles/ --top 20     # Top 20 priority styles
node scripts/oracle-batch-aggregate.js styles/ --styles apa,ieee,nature  # Specific styles
node scripts/oracle-batch-aggregate.js styles/ --json       # Machine-readable
```

**Features:**
- Tests multiple styles from STYLE_PRIORITY.md
- Aggregates component issues across all styles
- Shows which issues affect the most entries
- Ranks styles by bibliography success (worst first)

**Results from 10 priority styles:**
```
Styles tested: 10
Citations 100%: 9/10 (90%)
Bibliography 100%: 0/10 (0%)

--- TOP COMPONENT ISSUES ---
  year:extra: 19 occurrences
  issue:missing: 14 occurrences
  containerTitle:missing: 10 occurrences
  pages:missing: 5 occurrences
  volume:missing: 5 occurrences

--- ORDERING ISSUES: 59 total ---
```

### Tool 3: Migration Debugger (PENDING)

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

## Key Insight from Tooling: Year Position is Root Cause

### Sample Analysis (10 Priority Styles)

Running the batch oracle on 10 priority styles revealed a **systemic pattern**:

```
TOP COMPONENT ISSUES:
  year:extra: 19 occurrences      ← #1 issue
  issue:missing: 14 occurrences
  containerTitle:missing: 10 occurrences
  pages:missing: 5 occurrences
  volume:missing: 5 occurrences
```

### Full Corpus Analysis (2,844 Parent Styles)

Running the parallel batch oracle on the **entire CSL corpus** confirms and amplifies these findings:

```
Duration: ~18 minutes (4 workers)
Citations 100%: 348/2844 (12%)
Bibliography 100%: 26/2844 (1%)

TOP COMPONENT ISSUES:
  year:missing: 7,116 occurrences    ← #1 issue
  year:extra: 3,762 occurrences      ← #2 issue
  issue:missing: 2,556 occurrences
  doi:extra: 2,381 occurrences
  containerTitle:missing: 2,230 occurrences
  volume:missing: 2,203 occurrences
  editors:extra: 2,044 occurrences
  contributors:missing: 2,014 occurrences
  title:missing: 1,846 occurrences
  pages:extra: 1,799 occurrences

ORDERING ISSUES: 18,219 total
```

**Year positioning issues (missing + extra = 10,878) are the #1 problem across the entire corpus.**

### The Pattern

For numeric styles (IEEE, Nature, Elsevier Vancouver), CSLN produces:
```
contributors → year → title → containerTitle
```

But the oracle expects:
```
contributors → title → containerTitle → volume → year
```

**Year should be at the END for numeric styles, not after contributors.**

This is fundamentally different from author-date styles where year comes early (in parentheses after author).

### Root Cause Analysis

The migration currently:
1. Detects `issued` date in the bibliography template
2. Places it after contributors (author-date pattern)
3. Does NOT detect that numeric styles want year at end

### Proposed Fix: Style-Class-Aware Year Positioning

```rust
// In template compilation
if style_class == "numeric" {
    // Move year component to end of template
    move_component_to_end(&mut template, "date:issued");
}
```

This single fix should resolve:
- **10,878 year issues** → year moves to correct position
- **18,219 ordering issues** → most are caused by year being early
- **Cascade effect** → other components will naturally align

### Revised Priority Order

Based on tooling data, the **highest-impact fixes** are:

1. **Year positioning for numeric styles** (19 occurrences, 59 ordering issues)
2. **Issue suppression** (14 occurrences) - some styles don't show issue
3. **Container title extraction** (10 occurrences) - missing for some types
4. **Superscript citations** (Nature/Cell) - affects 2 styles

---

## Phase 1: Numeric Style Foundations

### Issue 1.0: Year Position for Numeric Styles (NEW - HIGHEST PRIORITY)

**Problem:** Year appears after contributors, but numeric styles want year at end.

**Current behavior:**
```
Oracle: [1]T. S. Kuhn, "The Structure...", International Encyclopedia..., vol. 2, no. 2, 1962
CSLN:   [1]T. S. Kuhn, 1962. "The Structure..."
         ↑ year in wrong position
```

**Evidence:** 19 `year:extra` occurrences, 59 ordering issues across 10 styles.

**Root cause:** Migration uses author-date year positioning for all styles.

**Fix:**
- [ ] Detect numeric style class from CSL `<citation>` element
- [ ] For numeric styles, move `date:issued` component to end of template
- [ ] Preserve year position for author-date styles

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

- [x] **Implement structured diff oracle** (Tool 1) ✅
- [x] **Implement batch oracle aggregation** (Tool 2) ✅
- [ ] **Fix year positioning for numeric styles** (Issue 1.0) ← NEW PRIORITY
- [ ] Fix Nature/Cell superscript citations (Issue 1.1)
- [ ] Fix Springer citation regression (Issue 2.1)

### Short-term (Tier 3.2)

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

- [x] Structured diff oracle identifies specific component failures ✅
- [x] Batch oracle can test 50+ styles and cluster failures ✅
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
