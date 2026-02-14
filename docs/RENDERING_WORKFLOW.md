# Rendering Fidelity Workflow Guide

This guide describes the standard workflow for debugging and fixing rendering issues in CSLN. It assumes you have basic familiarity with the project structure and oracle comparison tools.

## Quick Reference

```bash
# Process a bibliography with a style (default plain text)
csln process references.json styles/apa-7th.yaml

# Process with reference keys shown for debugging ([ITEM-1] ...)
csln process references.json styles/apa-7th.yaml --show-keys

# Convert a YAML style to binary CBOR for performance
csln convert styles/apa-7th.yaml --output styles/apa-7th.cbor

# Generate semantic HTML
csln process references.json styles/apa-7th.yaml --format html

# Test a single style (default: structured diff)
node scripts/oracle.js styles-legacy/apa.csl
```

## Hybrid Migration Strategy

CSLN uses a three-tier architecture to balance high fidelity for popular styles with broad coverage for the long tail of CSL 1.0 styles (see `csl26-m3lb`).

| Tier | Target | Method | Goal |
|------|--------|--------|------|
| **Tier 1: Core Options** | All Styles | XML Semantic Compiler | 100% fidelity for global options (names, dates, et-al) |
| **Tier 2: Top Styles** | Top 10 Parents | Agent-Assisted LLM Authoring | 100% citation + bibliography fidelity via `@styleauthor` |
| **Tier 3: The Long Tail** | 300+ Parents | Output-Driven Inference | 80%+ fidelity via automated template generation |
| **Tier 4: Fallback** | Remaining | XML Template Compiler | Baseline rendering for obscure styles |

**Priority**: We prioritize **Tier 2** for the top 10 parent styles (covering 60% of all dependent styles) and **Tier 3** for the next 40 styles.

## Fidelity Targets

Know when you've reached "good enough" for a style:

| Target | Criterion | Action |
|--------|-----------|--------|
| **Minimum Viable** | 5/5 citations, 8/15 bibliography | Move to next priority style |
| **High Quality** | 5/5 citations, 12/15 bibliography | Style ready for production use |
| **Perfect** | 15/15 citations AND bibliography | Diminishing returns - revisit later |

**When to Move On**: Once a style reaches 12/15+ bibliography matches, move to the next priority style. Perfecting one style has lower ROI than improving many styles to "good enough."

## Component-First Strategy (Tier 3 & 4)

**Key Principle**: Fix common failures across the "Long Tail" of styles, not individual styles in isolation.

**Why**: For Tier 3 and 4 styles (which use the automated compiler/inference), each component fix in the processor or migration logic can improve 10-20 styles simultaneously.

**Recommended Iteration Loop**:
1. Run batch analysis across top 20 styles
2. Identify most common component failure (e.g., "year formatting" in 15 styles)
3. Fix that ONE component issue in processor or migration code
4. Re-run batch and measure improvement
5. Repeat with next most common failure

See [WORKFLOW_ANALYSIS.md](./WORKFLOW_ANALYSIS.md) for detailed strategy.

## The Standard Workflow

When fixing rendering issues, follow this process:

### Step 1: Identify the Problem

Start with the structured oracle to see component-level differences:

```bash
node scripts/oracle.js styles-legacy/apa.csl
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
./scripts/workflow-test.sh styles-legacy/apa.csl
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

**CRITICAL INSIGHT - Component-First Approach:**

If multiple styles show the same component failure (e.g., "year issue" in both APA and IEEE), **this is your highest priority fix**. Fixing one common component improves 10-20 styles simultaneously.

**Action**: Focus on the most frequent component failures across the batch, not individual style debugging. Use the `componentSummary` from batch output to identify common issues.

### Step 3: Locate the Fix

Based on the style's priority and the nature of the issue, choose the appropriate fix strategy:

#### High Priority / Tier 2 Styles (Top 10)
→ Use **Specialized Agent Workflow**

This project uses a tri-agent specialist model to achieve high-fidelity rendering for top parent styles:

1.  **@dstyleplan**: Conducts deep research on the style guide and designs the component tree architecture (nesting and delimiters). Identifies missing processor features.
2.  **@styleplan**: Converts the architectural design into a technical build plan with actionable tasks and exact code snippets for the builder.
3.  **@styleauthor**: Executes the implementation (Haiku) using the hand-authoring loop.

**Workflow**: Run `./scripts/prep-migration.sh <style>` and use the specialized agents to hand-author the CSLN template.

#### Systemic Issues (affects Tier 3/4 styles)
→ Fix in `crates/csln_processor/`
- Example: Year parentheses missing across all author-date styles.
- Look in: `rendering.rs`, `bibliography.rs`, date formatting logic.

#### Style-Specific Issues (Tier 3/4)
→ Fix in migration logic or style YAML
- Example: APA uses "Vol." prefix, IEEE doesn't.
- Check: `crates/csln_migrate/`, generated YAML overrides.

#### Migration Issues (CSL → YAML conversion wrong)
→ Fix in `crates/csln_migrate/`
- Example: Variable ends up in wrong template section.
- **Migration Debugger** (planned): `csln_migrate --debug-variable VAR` will show provenance tracking.

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

#### Handling Missing Dates
If a reference is missing a date, use the `fallback` field in the `TemplateDate` component to provide alternative content, such as the "n.d." (no date) term.

```yaml
- date: issued
  form: year
  fallback:
    - term: no-date
```

#### Category Mapping
To handle formatting that applies to broad categories (like all periodicals), use the `type-mapping` configuration in `options.titles`. This eliminates the need for the processor to know which types are "journals."

```yaml
options:
  titles:
    type-mapping:
      article-journal: periodical
      thesis: monograph
    periodical:
      emph: true
```

#### Inner vs Outer Affixes
Components distinguish between affixes outside the wrap (spacing) and inside the wrap (labels).

```yaml
- number: pages
  wrap: parentheses
  inner-prefix: "pp. "  # Inside: (pp. 45)
  suffix: " "           # Outside: (pp. 45) next
```

**Context-Sensitive Examples:**
Use the `!mode-dependent` tag to handle differences between narrative and parenthetical citations (as in APA 7th):

```yaml
options:
  contributors:
    and: !mode-dependent
      integral: text        # Narrative: "Smith and Jones (2020)"
      non-integral: symbol  # Parenthetical: "(Smith & Jones, 2020)"
```

### Options Resolution

The processor merges options using **three-tier precedence**:

1. **Global options** (`options:`) - base defaults for all contexts
2. **Context-specific** (`citation.options:` or `bibliography.options:`) - override global for that context
3. **Template overrides** (component `overrides:`) - type-specific rendering

**Example: APA uses different author shortening per context**
```yaml
options:
  contributors:
    shorten:
      min: 21          # Bibliography: show up to 20 authors
      use-first: 19
citation:
  options:
    contributors:
      shorten:
        min: 3         # Citations: 3+ becomes "First et al."
        use-first: 1
```

When debugging unexpectedly different output between citations and bibliography, check for conflicting context-specific options.

### Step 5: Verify the Fix

Re-run the workflow test:

```bash
./scripts/workflow-test.sh styles-legacy/apa.csl
```

Check that:
1. ✅ The specific issue is fixed (structured oracle shows match)
2. ✅ No regressions in batch analysis (other styles still pass)
3. ✅ Rust tests still pass (`cargo test`)

### Step 6: Track Progress

After significant fixes, update the baseline (regression detection planned):

```bash
# Planned: Save baseline after milestone
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --save baselines/baseline-$(date +%F).json

# Planned: Compare against baseline to detect regressions
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --compare baselines/baseline-2026-02-06.json
```

This will catch regressions immediately (e.g., "APA: 15/15 → 14/15 - ITEM-3 now failing").

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
node scripts/oracle.js styles-legacy/apa.csl
node scripts/oracle.js styles-legacy/chicago-author-date.csl --verbose
```

### `oracle-simple.js` (String Comparison - LEGACY)

**When to use:** Rarely. Only for exact string output or when structured diff is insufficient.

**What it shows:** Raw string comparison (harder to parse)

**Example usage:**
```bash
node scripts/oracle-simple.js styles-legacy/apa.csl
```

### `oracle-batch-aggregate.js` (Multi-Style Impact)

**When to use:** After making changes to see broader impact.

**What it shows:** Pass/fail counts across multiple styles.

**Example usage:**
```bash
# Test top 10 styles
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10

# Test all author-date styles (may be slow)
node scripts/oracle-batch-aggregate.js styles-legacy/ --format author-date

# JSON output for scripting
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --json
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
./scripts/workflow-test.sh styles-legacy/apa.csl
./scripts/workflow-test.sh styles-legacy/ieee.csl --json
./scripts/workflow-test.sh styles-legacy/nature.csl --top 20
```

### `prep-migration.sh` (Agent Context Prep)

**When to use**: Mandatory first step when hand-authoring a high-priority (Tier 2) style using the `@styleauthor` agent.

**What it does**: 
1. Generates "Target Rendering" using `citeproc-js`.
2. Generates "Baseline CSLN" (Tier 1 options + Tier 4 templates).
3. Packages both into a high-fidelity context packet for the agent.

**Example usage**:
```bash
./scripts/prep-migration.sh styles-legacy/apa.csl
```
Then, copy the output and provide it to the `@styleauthor` agent to begin the iterative authoring process.

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

## Known Acceptable Differences

Some differences between citeproc-js and CSLN are intentional or acceptable. **Do not spend time investigating these**:

### HTML Entity Encoding
**Example**: `&#38;` vs `&`, `&lt;` vs `<`
**Why**: Different HTML encoding strategies are both valid
**Action**: Ignore these differences

### Whitespace Normalization
**Example**: `"Nature  521"` vs `"Nature 521"` (extra space)
**Why**: Whitespace collapsing is cosmetic
**Action**: Ignore unless it affects readability

### Unicode vs ASCII
**Example**: Em-dash `—` vs double-hyphen `--` in page ranges
**Why**: Both are acceptable representations
**Action**: Prefer Unicode in CSLN, but ASCII is not wrong

### Quote Normalization
**Example**: Smart quotes `"` vs straight quotes `"`
**Why**: Depends on style specification and output format
**Action**: Match style requirements, otherwise prefer smart quotes

If the structured oracle flags one of these differences, note it but continue working on substantive component mismatches (author, year, title, etc.).

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
   grep -n "volume" styles-legacy/apa.csl
   ```

2. **Check generated YAML:**
   ```bash
   csln_migrate styles-legacy/apa.csl > /tmp/apa.yaml
   grep -A5 "volume" /tmp/apa.yaml
   ```

3. **Compare with oracle:**
   ```bash
   node scripts/oracle.js styles-legacy/apa.csl --verbose
   ```

**Future (Task #24):** Use migration debugger:
```bash
csln_migrate styles-legacy/apa.csl --debug-variable volume
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
node scripts/oracle.js styles-legacy/apa.csl --cite

# Test only bibliography
node scripts/oracle.js styles-legacy/apa.csl --bib

# Limit batch analysis
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 5
```

## Troubleshooting

### "Oracle script not found"

Make sure you're running from project root or scripts directory:
```bash
cd /Users/brucedarcus/Code/csl26
node scripts/oracle.js styles-legacy/apa.csl
```

### "Style not found"

Check style path relative to current directory:
```bash
# From project root
node scripts/oracle.js styles-legacy/apa.csl

# From scripts/
node oracle.js ../styles-legacy/apa.csl
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
csln_migrate styles-legacy/apa.csl --debug-variable volume
# Shows: CSL source → IR → YAML, with deduplication decisions
```

### Phase 3: Regression Detection (Task #25)
```bash
# Save baseline
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --json > baselines/baseline-2026-02-05.json

# Compare against baseline
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 20 --compare baselines/baseline-2026-02-05.json
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
