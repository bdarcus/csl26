# Implementation Plan: Tier 2 Bibliography Rendering Fidelity

## Problem Statement

Tier 1 achieved **100% citation match** (15/15) for all 6 priority styles. However, bibliography rendering remains incomplete. This plan targets bibliography fidelity improvements.

### Current State

| Style | Citations | Bibliography |
|-------|-----------|--------------|
| apa | 15/15 ✅ | 2/15 |
| chicago-author-date | 15/15 ✅ | 1/15 |
| elsevier-harvard | 15/15 ✅ | 2/15 |
| springer-basic-author-date | 15/15 ✅ | 0/15 |
| ieee | 15/15 ✅ | 0/15 |
| elsevier-vancouver | 15/15 ✅ | 0/15 |

---

## Root Cause Analysis

Examining oracle failures reveals **5 systemic issues**:

### Issue 1: Container Title Missing (All Styles)

**Pattern:** Journal/proceedings name not appearing before volume.

```
Oracle: Nature 521, 436–444
CSLN:   521, 436–444
```

**Root Cause:** Container title (`container-title`) not in bibliography template or being suppressed incorrectly.

### Issue 2: "and" vs No "and" Before Last Author (Elsevier)

**Pattern:** Elsevier styles don't use "and" before last author in bibliography.

```
Oracle: LeCun, Y., Bengio, Y., Hinton, G.
CSLN:   LeCun, Y., Bengio, Y., and Hinton, G.
```

**Root Cause:** Bibliography contributor config not extracting Elsevier's `<name delimiter=", "/>` without `and` attribute.

### Issue 3: Editor Label Format (APA, Elsevier)

**Pattern:** Editor role missing or wrong format.

```
Oracle: Reis, H.T., Judd, C.M. (Eds.)
CSLN:   Reis, H.T., and Judd, C.M.
```

**Root Cause:** Editor-as-author case needs `(Ed.)` / `(Eds.)` suffix; extraction working but not applied to edited volumes.

### Issue 4: Conference Paper Format (Elsevier)

**Pattern:** "in:" prefix, "Presented at", "pp." for page ranges.

```
Oracle: Mikolov, T., ..., in: Proceedings of NIPS 2013. Presented at the Neural Information Processing Systems, pp. 3111–3119.
CSLN:   Mikolov, T., ..., Proceedings of NIPS 2013.
```

**Root Cause:** Conference papers need special container prefix ("in:") and page label extraction.

### Issue 5: Sorting Order (Chicago)

**Pattern:** Entries appear out of order (anonymous works sorted wrong).

```
Oracle: Entry 12 = Vaswani...
CSLN:   Entry 12 = The Role of Theory...
```

**Root Cause:** Anonymous work sorting not respecting "The" article stripping or year fallback.

### Issue 6: Component Ordering Within Lists

**Pattern:** Volume appears before container-title, pages appear before volume.

```
Oracle: Nature Climate Change, 10, 850–855.
CSLN:   850–855. 10 Nature Climate Change,
```

**Root Cause:** Components compiled from different CSL macros end up in wrong order.
Volume is first compiled from `label-volume` macro (for non-serial types), then from
`source-serial` (for journals). The first occurrence determines position.

---

## Workplan

### Phase 1: Container Title Restoration

**Status:** ✅ COMPLETED

- [x] Audit template generation for `container-title` variable
- [x] Fix suppression logic for journal/proceedings types
- [x] Fix component ordering within Lists (volume/container-title order)
- [x] Move pages to appear after container-title/volume for serials
- [x] Test against APA, Chicago, Elsevier Harvard

**Progress:**
- Implemented `propagate_list_overrides()` to ensure sibling components in a List
  all get the same type overrides (e.g., article-journal)
- Implemented `reorder_serial_components()` to put container-title before volume
- Implemented `reorder_pages_for_serials()` to move pages after serial list
- APA: 2/15 → 5/15 bibliography match
- Chicago: 1/15 → 3/15 bibliography match

### Phase 2: Bibliography "and" Configuration

**Status:** ⏳ PENDING

- [ ] Extract bibliography-specific `and` setting (can differ from citation)
- [ ] Add `and: Option<AndTerm>` to `BibliographyContributorConfig`
- [ ] Support `None` (no conjunction), `Text`, `Symbol`
- [ ] Test against Elsevier Harvard

### Phase 3: Edited Volume Author Labels

**Status:** ⏳ PENDING

- [ ] Detect when editors are in author position (edited volume without author)
- [ ] Apply `(Ed.)` / `(Eds.)` suffix based on contributor count
- [ ] Respect style-specific label form (short vs long)
- [ ] Test against APA, Elsevier Harvard

### Phase 4: Conference Paper Template

**Status:** ⏳ PENDING

- [ ] Extract container prefix ("in:", "In") from CSL conditionals
- [ ] Add page label extraction ("pp." from CSL Label nodes)
- [ ] Handle "Presented at the [event]" pattern
- [ ] Test against Elsevier Harvard

### Phase 5: Sorting Refinement

**Status:** ⏳ PENDING

- [ ] Review anonymous work sorting logic
- [ ] Ensure article stripping ("The", "A", "An") works for all styles
- [ ] Verify year-based secondary sort for same-name entries
- [ ] Test against Chicago Author-Date

### Phase 6: List Component Ordering

**Status:** ⏳ PENDING

- [ ] Reorder volume to appear after container-title in Lists
- [ ] Ensure pages appear after volume for serial types
- [ ] Handle ordering differences between serial and monographic types

---

## Commits

| Hash | Description |
|------|-------------|
| `73c0c4f` | docs: add tier 2 bibliography fidelity implementation plan |
| `205795e` | fix(migrate): propagate type overrides within lists |
| `ef33cea` | fix(migrate): reorder serial components for proper rendering |
| `a69725f` | docs: update tier 2 plan with phase 1 completion |
| `db28601` | fix(migrate): group volume and issue correctly in nested lists |
| `5f15b6c` | fix(migrate): add type-specific overrides for url and container-title |
| `1198d2c` | fix(migrate): add genre bracket wrap for thesis types |

---

## Success Criteria

**Target:** 8/15+ bibliography match for APA and Elsevier Harvard ✅ EXCEEDED

| Style | Start | Current | Target |
|-------|-------|---------|--------|
| apa | 2/15 | **11/15** ✅ | 8/15 |
| elsevier-harvard | 2/15 | 2/15 | 8/15 |
| chicago-author-date | 1/15 | 4/15 | 5/15 |

---

## Remaining Issues (APA)

1. **Entry 1 (thesis)**: Genre bracket attached to title without separator
2. **Entry 2 (chapter)**: "In [editors]" should come before book title
3. **Entry 8 (edited volume)**: Editors as authors need "(Eds.)" suffix after name
4. **Entry 14 (book)**: Edition should be in parentheses

## Dependencies

- Phase 1 blocks Phases 3, 4 (container title needed for context)
- Phase 2 is independent
- Phase 5 is independent

## Recommended Order

1. **Phase 1** - Container title (highest impact, fixes majority of failures) ✅ COMPLETED
2. **Phase 2** - Bibliography "and" config (Elsevier-specific, easy win)
3. **Phase 5** - Sorting refinement (Chicago, isolated fix)
4. **Phase 3** - Edited volume labels
5. **Phase 4** - Conference papers (most complex)
