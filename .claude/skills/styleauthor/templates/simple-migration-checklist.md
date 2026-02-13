# Simple Migration Checklist

Use this checklist for straightforward style migrations that don't require deep research or processor changes.

## When to Use Simple Path

- Converting standard author-date, numeric, or note styles
- Style uses only common formatting (no exotic features)
- Similar to existing styles (APA, MLA, Chicago, Elsevier)
- No obvious schema gaps based on CSL inspection

## Pre-Migration Checklist

Before deciding to use the simple path, verify:

- [ ] Style category is standard (author-date, numeric, or note)
- [ ] Only standard formatting: italics, bold, quotes, small-caps
- [ ] No complex conditionals or macros
- [ ] No custom sorting/disambiguation rules
- [ ] Similar to reference APA, Elsevier, or Chicago styles

## Simple Migration Workflow

**Standard path:** Phase 1 (research) → Phase 2 (plan) → Phase 3 (build) → Phase 4 (iterate) → Phase 5 (verify)

**Simple path:** Skip Phase 1, go directly to Phase 2

### Steps

1. **Prep**: Run `scripts/prep-migration.sh <csl-path>`
2. **Quick Analysis** (Coordinator):
   - Review target output patterns from citeproc-js
   - Confirm no exotic features (custom macros, complex conditionals)
   - If confident, proceed directly to Phase 2
3. **Phase 2** (styleplan): Creates build plan from prep output
4. **Phase 3** (styleauthor): Implements style YAML
5. **Phase 4** (styleauthor + styleplan): Iterate if needed (max 10 cycles)
6. **Phase 5** (styleplan): Rendering audit and verification

## Token Savings & Cost

**Token savings from skipping Phase 1:**
- Skip @dstyleplan sequential thinking: ~35K tokens
- Skip deep gap analysis: ~15K tokens
- **Tokens saved: ~50K tokens**

**Model change impact:**
- @styleauthor now uses Sonnet (was Haiku) for better template design
- Cost: 3x per token, but ~70% fewer iterations needed
- Net result: Similar total cost, but faster completion and fewer errors

**Total workflow:**
- Full workflow: ~140K tokens (Phase 1: 50K, Phase 2: 15K, Phase 3-4: 75K)
- Simple path: ~90K tokens (Phase 2: 15K, Phase 3-4: 75K)
- **Savings: ~50K tokens (36% reduction)**

## Risk Mitigation

**Focused Validation (oracle-migration.js):**
- Uses same 7-item subset as prep-migration.sh for consistency
- Fast execution (~10 seconds vs 30+ for full oracle.js)
- Clear success threshold: 5/7 items = acceptable, 7/7 = perfect

**Validation checkpoint at iteration 1:**
- Run oracle-migration.js immediately after first implementation
- If match rate <5/7 (71%), escalate immediately to @styleplan
- Don't waste iterations on fundamentally wrong template
- Saves ~80K tokens on failures

**Time Budget Enforcement:**
- Simple migration: 5 minutes max (1-2 iterations)
- Complex migration: 10 minutes max (4-5 iterations)
- Exceeded budget → escalate with status report

**If @styleauthor hits blockers during Phase 3-4:**
- Escalate to @dstyleplan for deep analysis
- Switches to full workflow with research phase
- No loss of work (escalation captures all context)

## Fast-Path Detection (Auto-Skip Phase 1)

For maximum efficiency, automatically detect simple migrations:

**Auto-qualify if ALL criteria met:**
- [ ] Processing type is `numeric` OR `author-date`
- [ ] Baseline CSLN from prep shows standard options (no exotic features)
- [ ] Prep output shows <5 type overrides needed
- [ ] Similar to existing style (check for `elsevier-*`, `apa-*`, `chicago-*` patterns)

**If auto-qualified:**
- Skip Phase 1 research entirely
- Use focused 7-item validation (oracle-migration.js)
- Time budget: 5 minutes max
- Success threshold: 5/7 items (71%) on first iteration

## Coordinator Decision Tree

```
Analyzing new style request...

┌─ Is it a new style or simple migration?
│
├─ NEW STYLE → Use @dstyleplan (Phase 1: research)
│             Then @styleplan + @styleauthor (Phases 2-5)
│
└─ SIMPLE MIGRATION → Check prep output
   │
   ├─ Auto-qualifies (numeric/author-date)? YES → FAST PATH
   │ └─ Skip Phase 1 → @styleplan (Phase 2) → @styleauthor (Phase 3-4)
   │    Use oracle-migration.js (7 items) for validation
   │    Time budget: 5 minutes
   │
   └─ Complex features? YES → Use full workflow
     └─ Use @dstyleplan (Phase 1)
```

## Example: Elsevier Harvard Migration

Elsevier Harvard was a good candidate for simple path:
- Standard author-date format
- Similar to Elsevier Vancouver (numeric) and APA 7th (author-date)
- No exotic features identified in prep output

Result: 50K token savings, 2-hour implementation time.

## When Simple Path Fails

If you hit any of these during Phase 3-4, **escalate immediately**:

1. **Schema mismatch** - New component type needed (e.g., legal case handling)
2. **Rendering gap** - Processor can't express required formatting
3. **Conditional logic** - Style needs if/else behavior not in current schema
4. **Collation rules** - Sorting differs from standard implementations
5. **Substitution rules** - Missing data fallback logic

**Escalation protocol:**
- Run `beans update TASK_ID --status blocked --note "Escalation: [reason]"`
- Contact coordinator for Phase 1 deep research
- @dstyleplan takes over to identify schema changes
- Resume Phase 2 with updated requirements

## Documentation Requirements

When a simple migration completes, document in style YAML comments:

```yaml
# Elsevier Harvard (hand-authored via simple migration path)
#
# Source: https://www.elsevier.com/authors/policies-and-guidelines/harvard
# Refs: elsevier-harvard.csl (CSL 1.0 parent style)
#
# Migration notes:
# - Standard author-date format with year in parentheses
# - No exotic features; schema alignment verified
# - Tested against prep output and oracle.js
```

This aids future maintenance and provides clear attribution.
