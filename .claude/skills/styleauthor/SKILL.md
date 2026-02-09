# Style Author

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Create CSLN citation styles from reference materials using an iterative author-test-fix loop

## Overview

The `/styleauthor` skill guides creation of CSLN styles from scratch. It follows a 5-phase iterative workflow: research reference materials, author the style YAML, test with the processor, evolve processor code if needed, and verify against oracle output.

This skill can modify both style YAML and processor/core code when features are missing. Guard rails ensure regressions are caught.

## Invocation

```
/styleauthor <style-name> [--urls URL1 URL2] [--format author-date|numeric|note]
/styleauthor update <style-path> [--mode language|output|full]
```

**Parameters:**
- `style-name` (required for new styles): Name for the style file (e.g., `chicago-author-date`)
- `style-path` (required for updates): Path to an existing style file (CSLN or CSL)
- `--urls` (optional): Reference URLs for the style guide
- `--format` (optional): Citation format class (default: `author-date`)
- `--mode` (optional for updates): Update focus (`language`, `output`, or `full`)

**Examples:**
- `/styleauthor chicago-author-date --urls https://www.chicagomanualofstyle.org/`
- `/styleauthor ieee --format numeric`
- `/styleauthor oscola --format note`
- `/styleauthor apa --migrate styles-legacy/apa.csl`
- `/styleauthor update styles/apa-7th.yaml --mode language`

## Workflow Phases

### Migration Workflow (Optional)

Use this workflow when converting an existing CSL 1.0 style. It identifies the target output and baseline configuration to accelerate Phase 1 & 2.

1.  **Prep**: Run `scripts/prep-migration.sh <path-to-csl>`
2.  **Analyze**:
    -   **Target Output** (citeproc-js): This is your visual goal.
    -   **Baseline CSLN**: Use the `options` block as your starting point (it extracts name rules, date forms, etc.).
3.  **Author**: Proceed to Phase 2, but focus on mapping the visual components in "Target Output" to CSLN template components.

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

### Phase 1: RESEARCH

Gather and understand the style's formatting rules.

1. Read any provided reference URLs (style guides, university LibGuides, example PDFs)
2. Study `styles/apa-7th.yaml` as the gold-standard template for CSLN style structure
3. Read `crates/csln_core/src/template.rs` for available `TemplateComponent` types and rendering options
4. Read `crates/csln_core/src/style.rs` for top-level `Style`, `Options`, `Citation`, `Bibliography` structs
5. Identify the citation format class: `author-date`, `numeric`, or `note`
6. Extract key formatting rules:
   - Author name format (inverted? initials? conjunction?)
   - Title formatting (italics, quotes, capitalization)
   - Source block structure (container, volume/issue, pages, publisher, DOI)
   - Citation format (parenthetical vs narrative, numbering scheme)

**Output:** Mental model of the style's rules, ready for authoring.

### Phase 2: AUTHOR

Create the style YAML file.

1. Create `styles/<style-name>.yaml`
2. Follow CSLN design principles:
   - **Explicit over magic**: All behavior in the YAML, not hidden in processor
   - **Declarative templates**: Flat components with type overrides, not procedural logic
   - **Structured blocks**: Use `items` with `delimiter` for grouped components (not flat lists). Use nested `items` to handle varying delimiters (e.g., space after a citation number, commas between author/title).
   - **Prefer wrap for semantic punctuation**: Always use `wrap: parentheses|brackets|quotes` instead of manual `prefix`/`suffix` pairs for balanced characters. Use `prefix`/`suffix` only for unbalanced text or unique spacing. Avoid `suffix: " "` for spacing; use delimiters instead.
   - **Minimize overrides**: Only add type-specific overrides where rendering genuinely differs
3. Include the `info` block with title, id, link, and source URLs as comments
4. Add comments explaining non-obvious formatting decisions
5. Refer to `.claude/skills/styleauthor/templates/common-patterns.yaml` for reusable snippets

**Structure guide:**
```yaml
---
info:
  title: Style Name (CSLN)
  id: https://www.zotero.org/styles/<style-name>-csln
  link: <official-guide-url>
  # Sources: ...
options:
  processing: author-date  # or numeric, note
  contributors: { ... }    # global defaults
  titles: { ... }
citation:
  options:                 # citation-specific overrides
    contributors:
      shorten:
        use-first: 1       # fewer authors in citations
  non-integral: { ... }
  integral: { ... }        # author-date only
bibliography:
  options:                 # bibliography-specific overrides
    contributors:
      shorten:
        min: 99            # show all authors in bibliography
  template: [ ... ]
```

### Phase 3: TEST

Run the processor and compare output to expectations.

```bash
cargo run --bin csln-processor -- styles/<style-name>.yaml
```

Compare each output line against the reference material:
- Check author formatting (name order, initials, conjunction, et al)
- Check title formatting (italics, quotes, case)
- Check source block (container, volume/issue, pages, publisher)
- Check punctuation and spacing between components
- Check citation format (parenthetical wrapping, narrative structure)

If output matches expectations, proceed to Phase 5.
If not, iterate: fix the style YAML and re-run. If the issue is a missing processor feature, go to Phase 4.

### Phase 4: EVOLVE (if needed)

Add missing features to the processor or core types.

**Allowed modifications:**
- `crates/csln_processor/` - Rendering engine
- `crates/csln_core/` - Type definitions and schema
- `styles/` - Style files

**Protected files (do NOT modify):**
- `crates/csln_migrate/` - Migration pipeline
- `scripts/oracle*.js` - Oracle comparison
- `tests/fixtures/` - Test fixtures

**After every processor change:**
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

All three must pass before continuing. If tests fail, fix the issue before proceeding.

**Iteration cap:** Maximum 10 test-fix cycles. If blocked after 10 iterations, report:
- What works correctly
- What's blocked and why
- Suggested processor changes needed

### Phase 5: VERIFY

Final verification before declaring done.

1. If a CSL 1.0 equivalent exists in `styles-legacy/`:
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

## Guard Rails

- **Always** run `cargo fmt && cargo clippy && cargo test` before declaring done
- **Always** test with at least 4 reference types (article, book, chapter, webpage)
- **Never** modify migration code, oracle scripts, or test fixtures
- **Never** commit - leave that to the user or lead agent
- **Maximum** 10 test-fix iterations before escalating
