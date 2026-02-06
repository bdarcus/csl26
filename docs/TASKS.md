# CSLN Task List

> **⚠️ DEPRECATED:** This document is now a historical snapshot. All active task tracking has migrated to [GitHub Issues](https://github.com/bdarcus/csl26/issues).
>
> **For Contributors:** Create new issues using the templates in `.github/ISSUE_TEMPLATE/`
>
> **Migration:** Use `./scripts/tasks-to-issues.sh` to export remaining tasks from this file to GitHub Issues.

**Status:** Historical snapshot (deprecated as primary tracker)
**Last Updated:** 2026-02-05 (pre-GitHub Issues migration)
**Migrated:** 2026-02-06

This document was the primary task tracker during early development. Tasks have been migrated to GitHub Issues for better community visibility and collaboration. This file is preserved for historical reference and task organization patterns.

## Task Status Legend

- ✅ **Completed** - Done and merged
- 🔄 **In Progress** - Currently being worked on
- 📋 **Pending** - Not started, ready to begin
- 🚫 **Blocked** - Waiting on another task

## High Priority: Rendering Fixes (Tier 1/2)

These tasks improve rendering fidelity for top parent styles and have the highest impact on the corpus.

### #14: Fix year positioning for numeric styles 📋

**Priority:** HIGHEST
**Impact:** ~10,000+ issues across entire corpus
**Effort:** 1-2 days
**Blocks:** #15, #16

**Problem:** Year appears after contributors in numeric styles, but should appear at end.

**Evidence:**
- 10,878 year position issues across 2,844 styles
- 18,219 ordering issues (mostly caused by year being early)

**Current:** `[1]T. S. Kuhn, 1962. "The Structure..."`
**Expected:** `[1]T. S. Kuhn, "The Structure...", vol. 2, no. 2, 1962`

**Fix:**
- Detect numeric style class from CSL citation element
- Move date:issued component to end of template for numeric styles
- Preserve year position for author-date styles
- Test against IEEE, Nature, Elsevier Vancouver

**Refs:** TIER3_PLAN.md Issue 1.0

---

### #15: Support superscript citation numbers 📋

**Priority:** HIGH
**Impact:** Nature, Cell, and other high-impact journals
**Effort:** 1-2 days
**Blocked by:** #14

**Problem:** Nature and Cell styles use superscript numbers ¹²³, not [1] or (Author Year).

**Current:** `(Kuhn 1962)`
**Expected:** `¹`

**Fix:**
- Detect `<text variable="citation-number"/>` in CSL citation layout
- Detect `vertical-align="sup"` on number text
- Set citation.template to number-only for numeric styles
- Handle superscript as rendering option in csln_core
- Test against Nature, Cell styles

**Refs:** TIER3_PLAN.md Issue 1.1

---

### #16: Fix volume/issue ordering for numeric styles 📋

**Priority:** HIGH
**Impact:** Numeric styles (57% of corpus)
**Effort:** 1-2 days
**Blocked by:** #14

**Problem:** Volume appears in wrong position with wrong punctuation for numeric styles.

**Current:** `Nature,. , 436–444 521:`
**Expected:** `Nature 521, 436–444`

**Issues:**
- Same ordering issues as author-date but different delimiters
- Colon for IEEE, comma for Nature
- Issue suppression for styles that don't show it

**Fix:**
- Apply reorder_serial_components() to numeric styles
- Extract delimiter from CSL group (comma for Nature, colon for IEEE)
- Suppress issue for styles that don't show it
- Test against IEEE, Nature, Elsevier Vancouver

**Refs:** TIER3_PLAN.md Issue 1.3

---

### #17: Debug Springer citation regression 📋

**Priority:** HIGH
**Impact:** 460 dependent styles (5.8% impact)
**Effort:** 1-2 days

**Problem:** Springer shows 0/15 citations (regression from earlier work).

**Current:** `(2018). Journal of...` ← wrong, showing bibliography entry
**Expected:** `(Smith 2020)`

**Investigation:**
- Debug Springer migration with --debug flag
- Compare citation template generation to APA
- Verify author-date format extraction
- Check for citation vs bibliography template confusion

**Refs:** TIER3_PLAN.md Issue 2.1

---

### #12: Fix conference paper template formatting 📋

**Priority:** MEDIUM
**Impact:** Conference paper citations
**Effort:** 2-3 days

**Problem:** Conference papers need special formatting with "in:", "Presented at", and "pp." for page ranges.

**Current issues:**
- Missing title after editors
- Wrong punctuation around "pp."
- "in:" without space

**Fix:**
- Extract container prefix ("in:", "In") from CSL conditionals
- Add page label extraction ("pp." from CSL Label nodes)
- Handle "Presented at the [event]" pattern
- Reorder chapter components: "In:" + editors + title + publisher + pages
- Test against Elsevier Harvard

**Expected:** `In: Ericsson KA, Charness N, ... (eds) The Cambridge Handbook of Expertise. Cambridge University Press, pp 683–703`

**Refs:** TIER2_PLAN.md Phase 4, TIER3_PLAN.md Phase 3

---

### #18: Support name formatting without periods in initials 📋

**Priority:** MEDIUM
**Impact:** Styles using no-period initials
**Effort:** 1 day

**Problem:** Some styles want initials without periods.

**Current:** `Kuhn, T. S.` (with periods)
**Expected:** `Kuhn TS` (no periods)

**Fix:**
- Handle `initialize-with=""` (no period after initials)
- Handle `initialize-with=" "` (space only)
- Distinguish from `initialize-with="."` (period)
- Update name rendering in csln_processor
- Test against styles using no-period format

**Refs:** TIER3_PLAN.md Issue 2.2

---

### #19: Support name delimiter without comma after family 📋

**Priority:** MEDIUM
**Impact:** Styles using space-only separator
**Effort:** 1 day

**Problem:** Some styles use "Family I" not "Family, I."

**Current:** `Smith, J, Anderson, M`
**Expected:** `Smith J, Anderson M`

**Fix:**
- Extract `sort-separator=" "` from CSL name element
- Apply to bibliography contributor config
- Test against styles using space-only separator
- Ensure family-given delimiter is configurable per style

**Refs:** TIER3_PLAN.md Issue 2.3

---

### #13: Refine bibliography sorting for anonymous works 📋

**Priority:** MEDIUM
**Impact:** Chicago Author-Date and similar styles
**Effort:** 1-2 days

**Problem:** Chicago Author-Date shows entries out of order for anonymous works.

**Issues:**
- Anonymous work sorting not respecting "The" article stripping
- Year fallback not working correctly for same-author entries

**Fix:**
- Review anonymous work sorting logic
- Ensure article stripping ("The", "A", "An") works for all styles
- Verify year-based secondary sort for same-name entries
- Test against Chicago Author-Date

**Target:** Chicago bibliography improves from 4/15 to 5/15+

**Refs:** TIER2_PLAN.md Phase 5

## Workflow Improvements

These tasks improve the development workflow and were identified in the workflow analysis (PR #103).

### #24: Implement migration variable debugger 📋

**Priority:** MEDIUM (Phase 2)
**Impact:** 50% reduction in migration debugging time
**Effort:** 3-5 days

**Goal:** Add `--debug-variable` flag to csln_migrate to trace variable provenance through the compilation pipeline.

**Features:**
- Track CSL source nodes → intermediate → final YAML
- Show deduplication decisions
- Display override propagation
- Output ordering transformations

**Example:**
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
  - overrides: {article-journal: {suppress: false}}
```

**Refs:** WORKFLOW_ANALYSIS.md Phase 2

---

### #25: Add baseline tracking for regression detection 📋

**Priority:** MEDIUM (Phase 3)
**Impact:** Catch regressions immediately
**Effort:** 2-3 days

**Goal:** Implement baseline tracking in oracle-batch-aggregate.js to detect regressions automatically.

**Features:**
- Save baseline results as JSON
- Compare current run against baseline
- Show regressions (previously passing now failing)
- Show improvements (newly passing)
- Net impact calculation

**Usage:**
```bash
node scripts/oracle-batch-aggregate.js --save baseline.json
node scripts/oracle-batch-aggregate.js --compare baseline.json
```

**Refs:** WORKFLOW_ANALYSIS.md Phase 3

---

### #26: Create test data generator tool 📋

**Priority:** LOW (Phase 3)
**Impact:** Makes test expansion 3-4x faster
**Effort:** 4 hours
**Blocks:** #11

**Goal:** Create `scripts/generate-test-item.js` interactive tool to make test data expansion easier.

**Features:**
- Interactive prompts for reference type and fields
- Validate required fields per type
- Auto-assign next ITEM-N number
- Add to references-expanded.json
- Run oracle comparison automatically
- Show results

**Refs:** WORKFLOW_ANALYSIS.md Phase 3

---

### #11: Expand test data coverage to 20+ items 📋

**Priority:** MEDIUM
**Impact:** Better coverage for tier 3 styles
**Effort:** 2-3 days (or 4 hours with #26 generator)
**Blocked by:** #26 (optional)

**Goal:** Current oracle tests use only 15 reference items. Expand to 25+ items covering more diverse reference types and edge cases.

**Phase 1 additions (10 items to reach 25 total):**
- 2 article-magazine
- 1 article-newspaper
- 2 software citations (increasingly important)
- 2 dataset citations (increasingly important)
- 1 legal_case
- 1 legislation
- 1 webpage with access date

**Edge cases to add:**
- No author (use title for sorting)
- No date ("n.d." handling)
- Very long title (>200 characters)
- Multilingual data (future: for csln#66)

**Refs:** WORKFLOW_ANALYSIS.md Phase 3, csln#64, csln#66

## Features & Enhancements

These tasks add new capabilities to CSLN beyond CSL 1.0 parity.

### #6: Support language-dependent title formatting 📋

**Priority:** MEDIUM
**Impact:** Multilingual bibliographies
**Effort:** 1-2 weeks

**Problem:** From GitHub Issue #97 [Expert]: In current CSL it's impossible to apply different rules for e.g. title-casing to the title of an article and the book title. However, it's quite common for edited volumes in German to contain an article in English.

**Requirements:**
- Entry-level language field support (biblatex/CSL-M pattern)
- Language-specific formatting rules per field
- Locale-specific template sections (CSL-M pattern)
- Support for multilingual documents with field-level language tagging

**Refs:** csln#66, GitHub #97

---

### #5: Implement full document processing 📋

**Priority:** LOW
**Impact:** Core functionality for end users
**Effort:** 2-4 weeks

**Goal:** From GitHub Issue #99: Once migration fidelity is a little better, implement formatting of full documents.

**Requirements:**
- Support for processing complete documents (not just individual citations/bibliography entries)
- Integration with document formats (likely Djot integration per #86)
- Proper handling of citation context within paragraphs
- Output formatting for different document types

**Refs:** csln#86, GitHub #99

---

### #9: Support automatic foot/endnoting of citations 📋

**Priority:** MEDIUM
**Impact:** Note styles (19% of corpus)
**Effort:** 2-3 weeks

**Goal:** From GitHub Issue #88: Ensure the processor supports automatic foot/endnoting of citations to enable seamless style switching between in-text and note styles.

**Requirements:**
- Citations in manually-placed foot/endnotes (position doesn't change with style)
- Automatically created footnotes (tool creates/removes notes based on style)
- Corner cases around surrounding punctuation (footnote mark outside period at end of sentence)
- Must work in both batch (Pandoc) and interactive (Word/Zotero) workflows

**Reference implementations:**
- Zotero/citeproc-js (details unclear)
- org-cite oc-csl.el (see GitHub link in issue)

**Refs:** csln#88, GitHub #88

---

### #10: Refactor delimiter handling with hybrid enum approach 📋

**Priority:** LOW
**Impact:** Code quality improvement
**Effort:** 1 week

**Goal:** Current delimiter handling mixes structural delimiters with decorative affixes. Implement hybrid approach with enum for common cases and Custom(String) for edge cases.

**Implementation:**
- Create Delimiter enum with variants: Comma, Period, Colon, Semicolon, Space, None, Custom(String)
- Analyze corpus to find common patterns and promote to enum variants
- Add validation in linter for non-standard Custom delimiters
- Migrate existing string-based delimiters incrementally

**Refs:** csln#89, csln#64

## Documentation & Evaluation

### #7: Create style authors guide 📋

**Priority:** MEDIUM
**Impact:** Onboarding for style authors
**Effort:** 1 week

**Goal:** From GitHub Issue #96: Create a document for the style author persona that explains how to use the new style model.

**Focus:**
- YAML option and syntax
- Setting up IDE for autocomplete and validation
- Highlight options and presets
- Make sure it 100% accurately represents the code
- Link to style-hub repo for style wizard integration

**Target audience:** Style authors from PERSONAS.md
**Output:** docs/guides/style-authoring.md

**Refs:** GitHub #96

---

### #8: Evaluate ICU library for date/time internationalization 📋

**Priority:** LOW
**Impact:** Architecture decision
**Effort:** 1 week (research + doc)

**Goal:** From GitHub Issue #93: Add proper internationalization of dates and times. Evaluate pros and cons of using ICU library vs alternatives.

**Consider:**
- EDTF native date handling (already prioritized)
- Locale-specific date formatting
- Integration complexity and dependencies
- Performance implications
- Compatibility with existing date handling

**Deliver:** Architecture decision document with recommendation

**Refs:** GitHub #93

## Completed Tasks ✅

### #1: Migrate project to Claude Code native tasks ✅
**Completed:** 2026-02-04
Converted TODO items, GitHub issues, and TIER plans to native Claude Code tasks with proper dependency chains.

### #2: Restructure documentation directories ✅
**Completed:** 2026-02-04
Moved `.agent/` to `docs/architecture/` for better organization.

### #3: Convert CLAUDE.md from symlink to standalone file ✅
**Completed:** 2026-02-04
Made CLAUDE.md a standalone project file with project-specific instructions.

### #4: Align project agents with global agents ✅
**Completed:** 2026-02-04
Documented integration between project and global `~/.claude/` agents.

### #20: Evaluate and improve rendering fidelity workflow ✅
**Completed:** 2026-02-05 (PR #103)
Comprehensive analysis identifying bottlenecks and improvement plan.

### #21: Make structured oracle the default comparison tool ✅
**Completed:** 2026-02-05 (PR #104)
Renamed oracle scripts to make component-level diff the default.

### #22: Create workflow test script with batch analysis ✅
**Completed:** 2026-02-05 (PR #104)
Created `scripts/workflow-test.sh` wrapper combining structured oracle + batch analysis.

### #23: Write rendering workflow documentation ✅
**Completed:** 2026-02-05 (PR #104)
Created `docs/RENDERING_WORKFLOW.md` with comprehensive workflow guide.

---

## How to Update This Document

When task status changes:

1. **For Claude Code users:** Tasks are automatically tracked. Update this file periodically:
   ```bash
   # Get current task list
   claude code /tasks

   # Update TASKS.md manually or via script
   ```

2. **For contributors without Claude Code:** Edit this file directly and open a PR.

3. **When creating new tasks:**
   - Add to appropriate section
   - Include: Priority, Impact, Effort, Dependencies, Description, Refs
   - Follow existing format

4. **When completing tasks:**
   - Move to "Completed Tasks" section
   - Add completion date and PR number
   - Keep brief summary

---

## Related Documentation

- **[WORKFLOW_ANALYSIS.md](./WORKFLOW_ANALYSIS.md)**: Detailed workflow bottleneck analysis
- **[RENDERING_WORKFLOW.md](./RENDERING_WORKFLOW.md)**: Step-by-step rendering fix guide
- **[STYLE_PRIORITY.md](./STYLE_PRIORITY.md)**: Which styles to prioritize by impact
- **[TIER2_PLAN.md](./architecture/TIER2_PLAN.md)**: Tier 2 implementation plan
- **[TIER3_PLAN.md](./architecture/TIER3_PLAN.md)**: Tier 3 implementation plan
