# Rendering Fidelity Workflow Guide

This guide describes the standard workflow for debugging and fixing rendering issues in CSLN. It assumes you have basic familiarity with the project structure and oracle comparison tools.

## Quick Reference

```bash
# Test a single style (default: structured diff)
node scripts/oracle.js styles/apa.csl

# Run full workflow test (structured diff + batch impact)
./scripts/workflow-test.sh styles/apa.csl

# Batch analysis across top 10 styles
node scripts/oracle-batch-aggregate.js styles/ --top 10

# Legacy simple string comparison
node scripts/oracle-simple.js styles/apa.csl
```

## The Standard Workflow

When fixing rendering issues, follow this process:

### Step 1: Identify the Problem

Start with the structured oracle to see component-level differences:

```bash
node scripts/oracle.js styles/apa.csl
```

This shows you **which specific components** differ between citeproc-js (oracle) and CSLN, not just that the strings are different.

**Example output:**
```
Bibliography Entry ITEM-1:
  ✓ author matches
  ✗ year: expected "(1962)" got "1962"
  ✓ title matches
  ✗ volume: expected "2(2)" got "Vol. 2, Issue 2"
```

This tells you:
- The year needs parentheses
- The volume/issue formatting is wrong

### Step 2: Understand the Scope

Run the workflow test to see if this is a style-specific issue or systemic:

```bash
./scripts/workflow-test.sh styles/apa.csl
```

This runs:
1. Structured oracle for the specific style (detailed diagnosis)
2. Batch analysis across top 10 styles (impact assessment)

**Interpreting batch results:**

```
Top 10 Priority Styles Analysis:
  APA (783 deps): 5/5 citations ✓, 3/5 bibliography (year, volume issues)
  Elsevier Harvard (665 deps): 5/5 citations ✓, 5/5 bibliography ✓
  IEEE (176 deps): 2/5 citations (year issue), 5/5 bibliography ✓
```

**Key insight:** If multiple styles show the same component failure (e.g., "year issue"), fix the root cause in the processor, not style-specific logic.

### Step 3: Locate the Fix

Based on scope, determine where to make changes:

#### Systemic Issues (affects multiple styles)
→ Fix in `crates/csln_processor/`
- Example: Year parentheses missing across all author-date styles
- Look in: `rendering.rs`, `bibliography.rs`, date formatting logic

#### Style-Specific Issues (affects one style or one style family)
→ Fix in migration logic or style YAML
- Example: APA uses "Vol." prefix, IEEE doesn't
- Check: `crates/csln_migrate/`, generated YAML overrides

#### Migration Issues (CSL → YAML conversion wrong)
→ Fix in `crates/csln_migrate/`
- Example: Variable ends up in wrong template section
- Future: Use `csln_migrate --debug-variable VAR` (Task #24)

### Step 4: Make the Fix

**Golden Rule:** Be explicit in style YAML, keep processor dumb.

**Bad (magic in processor):**
```rust
// Processor has hidden logic for journals
if ref_type == "article-journal" {
    volume_prefix = "Vol. ";
}
```

**Good (explicit in style):**
```yaml
# Style explicitly declares type-specific behavior
- variable: volume
  overrides:
    article-journal:
      prefix: "Vol. "
```

### Step 5: Verify the Fix

Re-run the workflow test:

```bash
./scripts/workflow-test.sh styles/apa.csl
```

Check that:
1. ✅ The specific issue is fixed (structured oracle shows match)
2. ✅ No regressions in batch analysis (other styles still pass)
3. ✅ Rust tests still pass (`cargo test`)

### Step 6: Track Progress

After significant fixes, update the baseline (Task #25, not yet implemented):

```bash
# Future: Save baseline after milestone
node scripts/oracle-batch-aggregate.js styles/ --top 20 --json > baselines/baseline-$(date +%F).json
```

## Oracle Scripts Reference

### `oracle.js` (Structured Diff - DEFAULT)

**When to use:** Always use this as your first diagnostic tool.

**What it shows:** Component-level differences (author, year, title, volume, etc.)

**Advantages:**
- Pinpoints **which component** is wrong
- Shows expected vs actual values
- Faster debugging than string comparison

**Output format:**
```
Citations:
  [ITEM-1] ✓ matches
  [ITEM-2] ✗ differs

Bibliography Entry ITEM-2:
  ✓ author: "Hawking, S." matches
  ✗ year: expected "(1988)" got "1988"
  ✓ title: "A Brief History of Time" matches
```

**Example usage:**
```bash
node scripts/oracle.js styles/apa.csl
node scripts/oracle.js styles/chicago-author-date.csl --verbose
```

### `oracle-simple.js` (String Comparison - LEGACY)

**When to use:** Rarely. Only for exact string output or when structured diff is insufficient.

**What it shows:** Raw string comparison (harder to parse)

**Example usage:**
```bash
node scripts/oracle-simple.js styles/apa.csl
```

### `oracle-batch-aggregate.js` (Multi-Style Impact)

**When to use:** After making changes to see broader impact.

**What it shows:** Pass/fail counts across multiple styles.

**Example usage:**
```bash
# Test top 10 styles
node scripts/oracle-batch-aggregate.js styles/ --top 10

# Test all author-date styles (may be slow)
node scripts/oracle-batch-aggregate.js styles/ --format author-date

# JSON output for scripting
node scripts/oracle-batch-aggregate.js styles/ --top 20 --json
```

**Output interpretation:**
```
Priority: 1 (783 dependents)
Style: apa.csl
Citations: 5/5 passing ✓
Bibliography: 3/5 passing
  Failing: ITEM-1, ITEM-3 (both have year formatting issue)
```

### `workflow-test.sh` (Recommended Wrapper)

**When to use:** Default workflow for any rendering fix.

**What it does:**
1. Runs structured oracle for detailed diagnosis
2. Runs batch analysis (top 10 styles) for impact assessment
3. Shows both in one command

**Example usage:**
```bash
./scripts/workflow-test.sh styles/apa.csl
./scripts/workflow-test.sh styles/ieee.csl --json
./scripts/workflow-test.sh styles/nature.csl --top 20
```

## Common Failure Patterns

### Pattern 1: Year Formatting

**Symptom:** Expected "(1988)" got "1988"

**Cause:** Missing `wrap: parentheses` in date rendering options

**Fix location:** `csln_migrate` date compilation or style YAML

**Example fix:**
```yaml
- date: issued
  form: year
  wrap: parentheses  # Add this
```

### Pattern 2: Volume/Issue Grouping

**Symptom:** Expected "2(2)" got "Vol. 2, Issue 2"

**Cause:** Missing delimiter override or incorrect template composition

**Fix location:** `csln_processor` bibliography rendering or migration logic

**Check:** Does CSL source use `<group delimiter="">` around volume/issue?

### Pattern 3: Author Name Order

**Symptom:** Expected "Kuhn, T. S." got "T. S. Kuhn"

**Cause:** Missing `name-order: family-first` or wrong disambiguation

**Fix location:** Style YAML contributor options

**Example fix:**
```yaml
- contributor: author
  form: long
  name-order: family-first
```

### Pattern 4: Missing Punctuation

**Symptom:** Expected "Nature, 521, 436-444." got "Nature 521 436-444"

**Cause:** Group delimiters not extracted from CSL during migration

**Status:** Known gap (see WORKFLOW_ANALYSIS.md bottleneck #1)

**Workaround:** Manually add delimiters to style YAML until migration improves

### Pattern 5: Initialization Inconsistency

**Symptom:** Expected "Kuhn, T. S." got "Kuhn, Thomas S."

**Cause:** `initialize-with` option not applied

**Fix location:** Style YAML contributor options or migration logic

**Example fix:**
```yaml
- contributor: author
  form: long
  initialize-with: "."
```

## Interpreting Structured Diff Output

The structured oracle breaks bibliography entries into semantic components. Here's how to read the output:

### Component Types

| Component | Description | Example |
|-----------|-------------|---------|
| `author` | Primary contributor(s) | "Kuhn, T. S." |
| `year` | Issued date | "(1962)" |
| `title` | Primary title | "The Structure of Scientific Revolutions" |
| `container-title` | Journal/book title | "Nature" |
| `volume` | Volume number | "2" or "Vol. 2" |
| `issue` | Issue number | "(2)" |
| `page` | Page range | "436-444" or "pp. 436-444" |
| `publisher` | Publisher name | "University of Chicago Press" |
| `DOI` | Digital object identifier | "https://doi.org/10.1234/example" |

### Match Symbols

- `✓` - Component matches oracle exactly
- `✗` - Component differs (shows expected vs actual)
- `(missing)` - Component in oracle but not in CSLN output
- `(extra)` - Component in CSLN but not in oracle

### Reading a Diff

```
Bibliography Entry ITEM-3:
  ✓ author: "LeCun, Y., Bengio, Y., & Hinton, G." matches
  ✗ year: expected "(2015)" got "2015"
  ✓ title: "Deep Learning" matches
  ✓ container-title: "Nature" matches
  ✓ volume: "521" matches
  ✗ page: expected "pp. 436-444" got "436-444"
```

**Diagnosis:**
1. Year needs parentheses wrapper
2. Page needs "pp." label prefix
3. Everything else is correct

**Action:** Fix year wrapping and page label extraction (likely in migration).

## Advanced Techniques

### Debugging Migration Issues

When a variable ends up in the wrong place or has wrong formatting, trace through the migration pipeline:

1. **Check CSL source:**
   ```bash
   grep -n "volume" styles/apa.csl
   ```

2. **Check generated YAML:**
   ```bash
   csln_migrate styles/apa.csl > /tmp/apa.yaml
   grep -A5 "volume" /tmp/apa.yaml
   ```

3. **Compare with oracle:**
   ```bash
   node scripts/oracle.js styles/apa.csl --verbose
   ```

**Future (Task #24):** Use migration debugger:
```bash
csln_migrate styles/apa.csl --debug-variable volume
```

### Testing Edge Cases

The current test data (`tests/fixtures/references-expanded.json`) has 15 items covering 8 reference types. When fixing issues:

1. **Check coverage:** Does the fix affect an untested reference type?
2. **Add test items:** Consider expanding test data (Task #11)
3. **Run batch:** See if fix helps untested styles

**Example edge cases to test:**
- No author (title-first sorting)
- No date ("n.d." handling)
- Very long titles (>200 chars)
- Corporate authors (literal names)

### Performance Optimization

When running many tests:

```bash
# Test only citations (faster)
node scripts/oracle.js styles/apa.csl --cite

# Test only bibliography
node scripts/oracle.js styles/apa.csl --bib

# Limit batch analysis
node scripts/oracle-batch-aggregate.js styles/ --top 5
```

## Troubleshooting

### "Oracle script not found"

Make sure you're running from project root or scripts directory:
```bash
cd /Users/brucedarcus/Code/csl26
node scripts/oracle.js styles/apa.csl
```

### "Style not found"

Check style path relative to current directory:
```bash
# From project root
node scripts/oracle.js styles/apa.csl

# From scripts/
node oracle.js ../styles/apa.csl
```

### "Locale not found"

Oracle scripts need locale files in scripts/ directory:
```bash
ls scripts/locales-*.xml
# Should show: locales-en-US.xml, etc.
```

### "citeproc module not found"

Install Node.js dependencies:
```bash
cd scripts
npm install citeproc
```

### Structured oracle shows all matches but strings differ

This means the component extraction is incomplete. The structured oracle only checks components it knows about. If strings differ but components match:

1. Check for punctuation/delimiter differences
2. Use `--verbose` flag for more detail
3. Fall back to `oracle-simple.js` for raw comparison
4. File an issue if it's a systematic gap

## Related Documentation

- **[WORKFLOW_ANALYSIS.md](./WORKFLOW_ANALYSIS.md)**: Detailed analysis of bottlenecks and improvement plan
- **[STYLE_PRIORITY.md](./STYLE_PRIORITY.md)**: Which styles to prioritize based on dependent counts
- **[TEST_STRATEGY.md](./architecture/design/TEST_STRATEGY.md)**: Oracle vs CSLN-native testing approach
- **[CLAUDE.md](../CLAUDE.md)**: Test commands and autonomous workflow whitelist

## Future Improvements

### Phase 2: Migration Debugger (Task #24)
```bash
csln_migrate styles/apa.csl --debug-variable volume
# Shows: CSL source → IR → YAML, with deduplication decisions
```

### Phase 3: Regression Detection (Task #25)
```bash
# Save baseline
node scripts/oracle-batch-aggregate.js styles/ --top 20 --json > baselines/baseline-2026-02-05.json

# Compare against baseline
node scripts/oracle-batch-aggregate.js styles/ --top 20 --compare baselines/baseline-2026-02-05.json
# Output: "Regression: APA 15/15 → 14/15 (ITEM-3 now failing)"
```

### Phase 4: Test Data Generator (Task #26)
```bash
node scripts/generate-test-item.js
# Interactive prompt to add new reference types to test fixtures
```

## Questions?

If this guide doesn't answer your question:

1. Check the [WORKFLOW_ANALYSIS.md](./WORKFLOW_ANALYSIS.md) for deeper technical details
2. Look at existing oracle script source code in `scripts/`
3. Run with `--verbose` flag for more diagnostic output
4. Check task list for known gaps (e.g., Task #11, #14, #24-26)
