# Style Author

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Create CSLN citation styles from reference materials using an iterative author-test-fix loop

## Overview

The style authoring workflow uses a tri-agent model adapted from the `minmax` patterns:

1.  **`@dstyleplan` (Specialist)**: Deep research and architectural design.
2.  **`@styleplan` (Specialist)**: Maintenance, bug fixes, and build planning.
3.  **`@styleauthor` (Builder)**: Implementation specialist (Sonnet).

## Invocation

The `/styleauthor` slash command is the universal entry point. Whichever agent receives the command acts as the **Coordinator** and follows the delegation logic below.

### Delegation Logic
- **If New Style / Complex Research**: Coordinator **must** delegate Phase 1 to `@dstyleplan`.
- **If Simple Migration**: Coordinator may skip Phase 1 and delegate directly to `@styleplan` (see Simple Migration Checklist).
- **If Maintenance / Simple Gaps**: Coordinator delegates to `@styleplan` for the build plan.
- **If Build Complete**: Coordinator **must** hand samples back to `@styleplan` or `@reviewer` for a final **Rendering Audit** before completion.
- **If Plan Approved**: Coordinator delegates Phase 3-4 to `@styleauthor`.

**Parameters:**
- `style-name` (required for new styles): Name for the style file (e.g., `chicago-author-date`)
- `style-path` (required for updates): Path to an existing style file (CSLN or CSL)
- `--urls` (optional): Reference URLs for the style guide
- `--format` (optional): Citation format class (default: `author-date`)
- `--mode` (optional for updates): Update focus (`language`, `output`, or `full`)

**Examples:**

- **Standard Entry**:
  `/styleauthor chicago-author-date --urls https://www.chicagomanualofstyle.org/`
- **Targeted Research**:
  `@dstyleplan /styleauthor nature --urls ...`
- **Targeted Modernization**:
  `@styleplan /styleauthor update styles/apa-7th.yaml --mode language`

## Workflow Phases

### Progress Tracking

The workflow automatically creates task tracking for visibility:

**At workflow start:**
```bash
/beans create "Style: {style-name}" --type feature --priority high
/beans update TASK_ID --status in-progress
```

**During execution:**
- Mark current phase as `in-progress` when starting: `/beans update TASK_ID --note "Phase N: [phase-name]"`
- Mark phase as `completed` when finished
- Update main task with current status
- On escalation: `/beans update TASK_ID --status blocked --note "Escalated: [reason]"`

**Benefits:**
- Real-time progress visibility for user
- Resume capability for interrupted workflows
- Clear audit trail of workflow execution
- Automatic task filtering by `in-progress` for next work

---

### Migration Workflow (Optional)

Use this workflow when converting an existing CSL 1.0 style. It identifies the target output and baseline configuration to accelerate Phase 1 & 2.

**For standard migrations**, see **[Simple Migration Checklist](./templates/simple-migration-checklist.md)** to potentially skip Phase 1 (research) and save ~50K tokens.

1.  **Prep**: Run `scripts/prep-migration.sh <path-to-csl>`
2.  **Analyze**:
    -   **Target Output** (citeproc-js): This is your visual goal.
    -   **Baseline CSLN**: Use the `options` block as your starting point (it extracts name rules, date forms, etc.).
3.  **Author**: Proceed to Phase 2, but focus on mapping the visual components in "Target Output" to CSLN template components.

**Simple Path** (if criteria met):
- Skip Phase 1 research
- Proceed directly to Phase 2 (build plan)
- See checklist for when this is appropriate

### Update Workflow (Optional)

Use this workflow to improve an existing style based on new language features or expanded output coverage.

1.  **Analyze**:
    -   Compare the style against Phase 2 "Standard Workflow Phases" for modern best practices.
    -   Identify missing reference types or edge cases by checking oracle output or reference materials.
2.  **Plan**:
    -   Fill out `.claude/skills/styleauthor/templates/update-checklist.md`.
    -   Prioritize modernization (e.g., replacing manual prefix/suffix with `wrap`).
3.  **Update**:
    -   Apply changes to the YAML file.
    -   If improving output, add new overrides or components to handle specific reference types.
4.  **Test & Verify**:
    -   Run `cargo run --bin csln-processor -- <style-path>`.
    -   Verify output against reference materials (e.g., style guide examples) or oracle output (if a legacy CSL exists).
    -   Ensure no regressions in existing supported types.

---

## Standard Workflow Phases

### Phase 1: RESEARCH (@dstyleplan)

Gather and understand the style's formatting rules using deep research and sequential thinking.

1. Read reference URLs, guide documents, and example PDFs.
2. Design the component tree architecture (nesting and delimiters).
3. **Identify Gaps**: Check if `csln_core` or the processor needs updates to support the requested formatting. If so, `@dstyleplan` must ask the user for approval to create a new task for core changes.

**Output:** Mental model of the style's architecture and identified gaps.

### Phase 2: PLAN (@styleplan)

Convert the architecture into actionable tasks.

1. Draft specific code changes for identified gaps (e.g., new components in `template.rs`). For significant core changes, present drafted code to the user for review.
2. Create a step-by-step implementation list for the builder.
3. Define assumptions and success criteria.

### Phase 3: BUILD (@styleauthor)

Implementation Specialist (Sonnet) takes over for the execution and test loop.

1. Implement core fixes and schema changes first.
2. Run `~/.claude/scripts/verify.sh` to ensure base correctness.
3. Author the style YAML using `/styleauthor`.
4. Verify rendering output against oracle or guides.

### Phase 4: EVOLVE & ITERATE

**Validation Checkpoint:** After iteration 1, run validation check:
- **Simple migrations**: Use `oracle-migration.js` (7-item focused test, ~10 seconds)
- **Complex styles**: Use full `oracle.js` (31-item comprehensive test)
- **Success threshold**: ≥5/7 items (71%) for simple, ≥50% for complex
- **If below threshold**: Escalate immediately to `@styleplan` for template redesign
- **Don't waste iterations** on fundamentally wrong structure

**Agent Transparency Requirement:**
After each iteration, the builder MUST report to user:
- Iteration number and validation results (X/7 or X/31 matches)
- What was fixed in this iteration
- What issues remain (if any)
- Next step (continue iterating or escalate)

If output doesn't match after 2 implementation retries (excluding checkpoint escalation), the builder escalates back to `@styleplan` to refine the strategy. When escalating, the agent must report the problem details to the user.

**Allowed modifications:**
- `crates/csln_processor/` - Rendering engine
- `crates/csln_core/` - Type definitions and schema
- `styles/` - Style files

**Protected files (do NOT modify):**
- `crates/csln_migrate/` - Migration pipeline
- `scripts/oracle*.js` - Oracle comparison
- `tests/fixtures/` - Test fixtures

**After every processor change:**
The agent must always run these checks and report their outcome to the user:
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

All three must pass before continuing. If tests fail, fix the issue before proceeding.

**Iteration cap:** Maximum 6 test-fix cycles (reduced from 10 due to validation checkpoint at iteration 2). If blocked after 6 iterations, the agent must clearly output the following to the user:
- What works correctly
- What's blocked and why
- Suggested processor changes needed

### Phase 5: VERIFY (@styleplan)

Final verification before declaring done. The builder MUST surface output samples for this phase.

1. **Rendering Audit**: @styleplan checks for spacing issues (double spaces, space before punctuation).
2. If a CSL 1.0 equivalent exists in `styles-legacy/`:
   ```bash
   node scripts/oracle.js styles-legacy/<style-name>.csl
   ```
   Compare CSLN output to citeproc-js output.

2. Run full test suite to check for regressions:
   ```bash
   cargo test
   ```

3. Document any known gaps between the CSLN style and reference material as comments in the YAML.

4. Verify the style covers at minimum:
   - Journal article
   - Book
   - Chapter/edited book
   - Webpage
   - Report (if applicable to the style)

## Time Budgets & Success Criteria

### Time Budget Enforcement

**Simple Migration (numeric/author-date):**
- Phase 2 (plan): 2 minutes max
- Phase 3-4 (build + iterate): 5 minutes max
- **Total: 7 minutes** (down from 15 minutes)

**Complex Migration (note styles, legal citations):**
- Phase 1 (research): 5 minutes max
- Phase 2 (plan): 3 minutes max
- Phase 3-4 (build + iterate): 10 minutes max
- **Total: 18 minutes**

**If time budget exceeded:**
- Agent must stop and report current status
- Surface what works, what's blocked, why
- User decides: continue iterating, accept partial success, or abandon

### Success Criteria Matrix

**Simple Migration (7-item focused test):**
```
oracle-migration.js results:
  7/7 citations + 7/7 bibliography = PERFECT ✅
  5-6/7 = ACCEPTABLE ✅ (document gaps in YAML comments)
  <5/7 = ESCALATE ⚠️ (template redesign needed)
```

**Complex Style (31-item comprehensive test):**
```
oracle.js results:
  >80% match = EXCELLENT ✅
  60-80% match = ACCEPTABLE ✅ (document known gaps)
  <60% match = ESCALATE ⚠️ (schema or processor changes needed)
```

**Validation cadence:**
- Iteration 1: Quick check (oracle-migration.js for simple, oracle.js for complex)
- Iteration 2: Full validation if iteration 1 passed
- Iteration 3+: Only if making targeted fixes to specific issues

### Validation Scripts

**oracle-migration.js** (to be created):
- Uses same 7-item subset as prep-migration.sh
- Fast execution (~10 seconds)
- Outputs: X/7 citations, X/7 bibliography
- Exit code 0 if ≥5/7, exit code 1 if <5/7

**oracle.js** (existing):
- Full 31-item comprehensive test
- Slower execution (~30 seconds)
- Outputs: X/31 citations, X/31 bibliography + component diff
- Use for final verification or complex styles

## Schema Reference

### Top-Level Structure
- `info` - Style metadata (title, id, link)
- `options` - Global formatting options
- `citation` - Citation specification (template + options)
- `bibliography` - Bibliography specification (template + options)

### Key Component Types
From `TemplateComponent` in `csln_core/src/template.rs`:
- `contributor` - Author, editor, translator (form: short/long/verb)
- `date` - Issued, accessed (form: year/full)
- `title` - Primary, parent-monograph, parent-serial
- `number` - Volume, issue, pages, edition
- `variable` - Publisher, doi, url, container-title, any string variable
- `items` - Group of components rendered together with shared delimiter

### Rendering Options
From `Rendering` in `csln_core/src/template.rs`:
- `emph` - Italics
- `strong` - Bold
- `quote` - Wrap in quotes
- `small-caps` - Small caps
- `prefix` / `suffix` - Text before/after
- `wrap` - Parentheses, brackets, quotes
- `suppress` - Hide this component (for type overrides)

### Three-Tier Options Architecture

Options are resolved in precedence order (inspired by biblatex):

| Tier | Location | Purpose |
|------|----------|---------|  
| 1. Global | `options:` | Style-wide defaults |
| 2. Context | `citation.options:` / `bibliography.options:` | Context-specific overrides |
| 3. Template | Component `overrides:` | Type-specific rendering |

**Tier 1 - Global options** (at style root):
```yaml
options:
  processing: author-date
  contributors:
    and: symbol
    shorten:
      min: 21
      use-first: 19
```

**Tier 2 - Context-specific options** (within citation/bibliography):
```yaml
citation:
  options:
    contributors:
      shorten:
        min: 3
        use-first: 1  # Fewer names in citations
bibliography:
  options:
    contributors:
      shorten:
        min: 99  # Show all names in bibliography
```

**Tier 3 - Template overrides** (on individual components):
```yaml
- number: pages
  overrides:
    chapter:
      wrap: parentheses
      prefix: "pp. "
    article-journal:
      suppress: true  # Hide pages for journals
```

### Options Reference
From `Config` in `csln_core/src/options/mod.rs`:
- `processing` - author-date, numeric, note
- `contributors` - Name formatting (initialize-with, and, display-as-sort, shorten)
- `titles` - Title formatting by category (monograph, periodical, component)
- `dates` - Date formatting defaults
- `substitute` - Substitution rules for missing data
- `localize` - Locale settings

## Design Principles

These come from the project's CLAUDE.md:

1. **Explicit over magic** - All behavior in the style YAML, not hardcoded in processor
2. **Declarative templates** - Flat components with type overrides, not `if/else` logic
3. **Structured blocks** - Use `items` with `delimiter` to group related components
4. **Minimal overrides** - Only where rendering genuinely differs by reference type
5. **Comments for clarity** - Explain non-obvious formatting decisions
6. **Source attribution** - Include reference URLs in info.link and as comments
7. **Semantic wrapping** - Use `wrap` for balanced punctuation (brackets, parentheses, quotes) to allow the processor to handle punctuation logic intelligently. Avoid manual `prefix: "["` pairs.
8. **Semantic joining** - Use nested `items` groups with `delimiter` to manage spacing between component blocks. Avoid trailing spaces in `suffix: " "` or leading spaces in `prefix: " "` when they serve as implicit delimiters between optional components. Static leading elements like citation numbers may use a space suffix for simplicity.

## Autonomous Command Whitelist

The styleauthor workflow has pre-approved safe operations that execute without confirmation:

### Always Safe (Style Development)
- Creating/editing `styles/*.yaml` - New or updated style files
- Running `node scripts/oracle*.js` - Oracle comparison tests
- `cargo fmt`, `cargo clippy`, `cargo check` - Code quality checks
- `cargo test` - Test suite execution
- `cargo run --bin csln-*` - Project binaries
- `git add`, `git commit` (feature branches only) - Commits to feature branches
- `git status`, `git diff`, `git log`, `git branch` - Inspection commands
- `mkdir -p styles/`, `mkdir -p tests/` - Safe directory creation

### Safe Cleanup
- Removing generated files: `target/`, `*.log`, `*.tmp`

### Require Confirmation
- `git push --force` or `git push --force-with-lease` - Destructive pushes
- `git push origin main` - Pushing to main branch
- `rm -rf` outside style/test temp directories - Destructive deletions
- Modifying `Cargo.toml`, `Cargo.lock` - Dependency changes
- Modifying `crates/csln_migrate/` - Migration pipeline (protected)
- Modifying `scripts/oracle*.js` - Oracle scripts (protected)
- Modifying `tests/fixtures/` - Test fixtures (protected)

## Guard Rails

- **Always** run `cargo fmt && cargo clippy && cargo test` before declaring done
- **Always** test with at least 4 reference types (article, book, chapter, webpage)
- **Never** modify migration code, oracle scripts, or test fixtures
- **Never** commit to main directly - only to feature branches
- **Validation checkpoint** at iteration 2: If match rate <50%, escalate immediately
- **Maximum** 6 test-fix iterations before escalating (reduced from 10)
