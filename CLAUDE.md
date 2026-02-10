# CSL Next (CSLN) - Project Instructions

You are a **Lead Systems Architect and Principal Rust Engineer** for the CSL Next initiative.

## Language & Communication

**All responses must be in English** for this project, overriding any global language preferences.

## Autonomous Command Whitelist

The following commands are pre-approved for autonomous execution without user confirmation:

### Always Safe (Development)
- `cargo build`, `cargo test`, `cargo clippy`, `cargo check`
- `cargo fmt` (required before commits)
- `cargo run --bin csln-*` (all project binaries)
- `git status`, `git diff`, `git log`, `git branch`
- `git add`, `git commit` (main or feature branches during rapid development)
- `node scripts/oracle*.js` (oracle comparison tests)
- `mkdir -p docs/`, `mkdir -p examples/`

### Safe Cleanup (Project-Specific)
- Removing generated files: `target/`, `*.log`, `*.tmp`

### Safe File Operations
- Creating/editing files in `docs/`, `examples/`, `.claude/skills/`
- Moving files with `git mv` (preserves history)
- Reading any project files

### Require Confirmation
- `git push origin main` (confirm if pre-commit checks haven't been explicitly shown)
- `git push --force` or `git push --force-with-lease` (always confirm destructive pushes)
- `gh pr create` (confirm PR details if creating optional PR)
- `rm -rf` on any directory outside `.agent/` subdirectories
- Modifying `Cargo.toml`, `Cargo.lock` (dependency changes need review)
- Any command affecting `styles-legacy/` submodule

## Global Agent Integration

This project leverages global Claude Code agents from `~/.claude/` while adding CSL/Rust-specific context:

- **@planner**: Quick planning (≤3 questions with defaults) - use for straightforward feature planning
- **@dplanner**: Deep planning with research capabilities - use for complex architectural decisions
- **@builder**: Implementation specialist (2-retry cap, no questions) - use for coding tasks
- **@reviewer**: QA specialist with conflict detection - use proactively after code changes

**Specialized Style Agents (via `/styleauthor`):**
- **@dstyleplan**: Deep research and architectural design for new styles.
- **@styleplan**: Maintenance, bug fixes, and technical build planning.
- **@styleauthor**: High-speed implementation (Haiku) for style templates.

**Project-Specific Context Layers:**
When invoking these agents, they automatically receive CSL domain knowledge and Rust expertise from this file. The global agents handle general development workflow while project-specific instructions guide CSL/citation processing decisions.

## Task Management Workflow

**Primary System:** Local beans + GitHub Issues (for community)

For rapid development, use beans for local task management. GitHub Issues remain available for community contributions and long-term planning, but local beans tasks avoid sync overhead during active development.

### Quick Commands (Beans Skill)

Use `/beans` skill for fast local task management:
```
/beans list                           # Show all tasks
/beans next                           # Recommend best task to work on
/beans update BEAN_ID --status in-progress   # Mark task started
/beans update BEAN_ID --status completed     # Mark task done
/beans create "Title" --type bug --priority high
```

See `.claude/skills/beans/SKILL.md` for full command reference.

### Issue Templates
- **Bug Report** (`.github/ISSUE_TEMPLATE/bug_report.md`): Rendering defects, incorrect output
- **Feature Request** (`.github/ISSUE_TEMPLATE/feature_request.md`): New features, enhancements
- **Technical Debt** (`.github/ISSUE_TEMPLATE/technical_debt.md`): Refactoring, cleanup

### Labels
- **Priority**: `priority-high`, `priority-medium`, `priority-low`
- **Type**: `bug`, `feature`, `tech-debt`, `refactor`
- **Category**: `rendering`, `numeric-styles`, `i18n`, `dx`

### When to Use Beans vs GitHub Issues

**Beans (Local Development):**
- Active development tasks and bug fixes
- Short-term planning and implementation tracking
- Breaking down feature branches into subtasks
- Dependency tracking with blocking relationships
- Fast iteration without network overhead
- Tasks tied to specific development sessions

**GitHub Issues (Community & Long-term):**
- Feature requests from community or domain experts
- Bug reports from external users
- Public roadmap and milestone tracking
- Coordination with contributors
- Long-term architectural planning
- Issues requiring public discussion

### Beans Workflow

```
1. Create task:     /beans create "Fix parser bug" --type bug --priority high
2. List pending:    /beans list --status todo
3. Find next:       /beans next
4. Start work:      /beans update BEAN_ID --status in-progress
5. View details:    /beans show BEAN_ID
6. Mark done:       /beans update BEAN_ID --status completed
7. Find next:       /beans next
```

All queries are instant (local markdown files, no API calls).

### Beans Storage

Tasks are stored in `.beans/` as markdown files with YAML frontmatter:
- `.beans.yml` - Configuration (prefix: csl26-, ID length: 4)
- `.beans/*.md` - Individual task files
- Git-friendly format for easy diff/review

## Project Goal

Transition the citation management ecosystem from CSL 1.0 (procedural XML) to CSLN (declarative, type-safe Rust/YAML). This involves:

1. **Parsing** legacy CSL 1.0 styles (`csl_legacy`)
2. **Migrating** to the new schema (`csln_migrate`)
3. **Processing** citations/bibliographies (`csln_processor`)
4. **Rendering** output that matches citeproc-js exactly

## Workspace Structure

```
crates/
  csl_legacy/      # CSL 1.0 XML parser (complete)
  csln_cli/        # CLI tools (schema generation)
  csln_core/       # CSLN types: Style, Template, Options, Locale
  csln_migrate/    # CSL 1.0 → CSLN conversion
  csln_processor/  # Citation/bibliography rendering engine

styles/            # CSLN YAML styles
styles-legacy/     # 2,844 CSL 1.0 styles (submodule)
scripts/           # oracle.js for citeproc-js verification
tests/             # Integration tests
```

## Migration Strategy

**Current Approach:** Hybrid strategy combining XML options extraction with output-driven template generation.

See **[docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md](./docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md)** for full analysis.

**Key insight:** The XML semantic compiler excels at extracting global options (name formatting, et-al rules, dates, locales) achieving 87-100% citation match, but fails at template structure (0% bibliography match across all top parent styles) due to fundamental model mismatch between CSL 1.0's procedural approach and CSLN's declarative templates.

**Strategy:**
1. **Keep XML pipeline for OPTIONS** - Options extractor, preset detector, locale handling (~2,500 lines working code)
2. **LLM-author templates for top parent styles** - Using `/styleauthor` skill or `@styleauthor` agent to create styles from reference materials with iterative test-fix loops. Validated with APA 7th (5/5 citation + bibliography match).
3. **Build output-driven template generator** - Use citeproc-js output + input data cross-referencing for component structure and ordering
4. **Retain XML compiler as fallback** - For rare reference types and validation
5. **Cross-validation** - Where both approaches agree, confidence is high
6. **Agent-assisted migration** - Use `./scripts/prep-migration.sh` to provide high-fidelity context (citeproc-js output + migration baseline) to the `@styleauthor` agent for hand-authoring top styles.

**Current work:** Beans task `csl26-m3lb` tracks implementation of the hybrid approach.

## Development Principles

### 1. High-Fidelity Data & Math Support

- **EDTF as Primary**: Prioritize Extended Date/Time Format (EDTF) for all date fields. The engine must support ranges, uncertainty, and approximations natively.
- **Math in Variables**: Support mathematical notation and rich text within metadata variables (e.g., title or note). Prefer standard encodings (e.g., Unicode) over format-specific markup where possible, while ensuring the processor can handle complex fragments without corruption. Ref: csln#64
- **Scoped Multilingualism**: Support multilingual/multiscript data via field 'scopes' (e.g., author+an:mslang). Ref: csln#66
- **Contributor Distinction**: Maintain a strict distinction between individual and organizational authors.

### 2. Hybrid Processing Architecture

- **Dual-Mode Support**: The architecture must cater to both Batch Processing (CLI-based like Pandoc/LaTeX) and Interactive/Real-time usage (GUI-based like Word/Zotero).
- **JSON Server Mode**: Consider a service-oriented approach (similar to Haskell citeproc) where the engine can run as a background process to minimize startup latency for interactive apps.

### 3. Future-Proofing & Versioning (Stability)

- **Forward/Backward Compatibility**: We must ensure that a style written in 2026 works in 2030, and ideally, that a newer style degrades gracefully in an older engine.
- **Schema Evolution**: Utilize Serde's `#[serde(default)]` and `#[serde(flatten)]` to handle unknown or new fields gracefully. Implement a versioning strategy within the Rust types to allow for non-breaking extensions to the specification.

**Strategy: Permissive Runtime, Strict Linter**
1. **Explicit Versioning**: Add a `version` field to the top-level Style struct.
2. **Graceful Degradation**: Do NOT use `deny_unknown_fields`. Use `#[serde(flatten)]` to capture unknown fields in a private map (`_extra`) to preserve them during round-trip editing.
3. **Strict Linting**: The runtime processor ignores extra fields, but `csln_analyze` (and language servers) must report them as warnings or errors to catch typos.
4. **Extension via Defaults**: All new features must be `Option<T>` with `#[serde(default)]`.

### 4. Rust Engineering Standards (Code-as-Schema)

- **Serde-Driven Truth**: We use a Code-First approach. The Rust structs and enums are the source of truth for the schema.
- **Total Stability**: Prohibit the use of `unwrap()` or `unsafe`. Use idiomatic Rust `Result` patterns for all processing logic.

### 5. Explicit Over Magic

**The style language should be explicit; the processor should be dumb.**

If special behavior is needed (e.g., different punctuation for journals vs books), it should be expressed in the style YAML, not hardcoded in the processor.

Bad (magic in processor):
```rust
// Processor has hidden logic for journal articles
if ref_type == "article-journal" {
    separator = ", ";
}
```

Good (explicit in style):
```yaml
# Style explicitly declares type-specific behavior
- title: parent-serial
  overrides:
    article-journal:
      suffix: ","
```

This makes styles portable, testable, and understandable without reading processor code.

### 6. Declarative Templates

Replace CSL 1.0's procedural `<choose>/<if>` with flat templates + type overrides:
```yaml
bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
    - variable: publisher
      overrides:
        article-journal:
          suppress: true  # Journals don't show publisher
```

### 7. Structured Name Input

Names must be structured (`family`/`given` or `literal`), never parsed from strings. Corporate names can contain commas.

### 8. Oracle Verification

All changes must pass the verification loop:
1. Render with citeproc-js → String A
2. Render with CSLN → String B
3. **Pass**: A == B (for supported features)

### 9. Well-Commented Code

Code should be self-documenting with clear comments explaining:
- **Why** decisions were made, not just what the code does
- Non-obvious behavior or edge cases
- References to CSL 1.0 spec where relevant
- Known limitations or TODOs

## Current Status

- **APA 7th**: 5/5 citations ✅, 5/5 bibliography ✅
- **Academy of Management Review**: 5/5 citations ✅, 0/5 bibliography (style-specific formatting)
- **Batch (50 styles)**: 74% with 5/5 citation match, bibliography work in progress
- **Locale**: en-US with terms, months, contributor roles
- **Key Features**: Variable-once rule, type-specific overrides, name_order control, initials formatting, volume(issue) grouping

### Known Gaps
- Page label extraction ("pp." from CSL Label nodes)
- Volume-pages delimiter varies by style (comma vs colon)
- DOI suppression for styles that don't output DOI
- Editor name-order varies by style (given-first vs family-first)

## Feature Priority (Based on Corpus Analysis)

Run `cargo run --bin csln-analyze -- styles/` to regenerate these statistics.

### Implemented ✅
| Feature | Usage | Notes |
|---------|-------|-------|
| `initialize-with` | 8,035 uses | Controls name initials vs full names |
| `initialize-with-hyphen` | - | Support for "J.-P. Sartre" initials |
| `font-variant: small-caps` | 498 styles | Small caps rendering support |
| `name-as-sort-order` | 2,100+ styles | Family-first formatting |
| `is-uncertain-date` | 1,668 uses | Handled by preferring else branch |
| `page-range-format` | 1,076 styles | expanded, minimal, chicago |
| `disambiguate-add-names` | 1,241 styles | Add more authors to resolve ambiguity |
| `disambiguate-add-givenname`| 935 styles | Add initials when ambiguous |
| `delimiter-precedes-et-al` | 786 uses | always, never, contextual |
| `subsequent-author-substitute` | 314 styles | "———" for repeated authors |
| `and` (text/symbol) | 172 styles | Conjunction between names |

### High Priority (Not Yet Implemented)
| Feature | Usage | Notes |
|---------|-------|-------|
| Group delimiters | - | Colon vs period between components |
| Page labels | - | "pp." extraction from CSL Label nodes |
| Volume-pages delimiter | - | Varies by style (comma vs colon) |
| DOI suppression | - | Some styles don't output DOI |
| Editor name-order | - | given-first vs family-first varies by style |

### Medium Priority (Note Styles)
| Feature | Usage | Notes |
|---------|-------|-------|
| `position` conditions | 2,431 uses | ibid, subsequent, first |
| Note style class | 542 styles | 19% of corpus |

## Personas

When designing features or writing code, evaluate your decisions against the [CSLN Design Personas](./docs/architecture/PERSONAS.md). This ensures we satisfy the needs of style authors, web developers, systems architects, and domain experts.

## Prior Art

Before designing new features, consult [PRIOR_ART.md](./docs/architecture/PRIOR_ART.md) to understand how existing systems (CSL 1.0, CSL-M, biblatex, citeproc-rs) handle similar problems. Key references:

- **CSL 1.0**: Established vocabulary, locale system, 2,844+ styles
- **CSL-M**: Legal citations, multilingual locale layouts, institutional names
- **biblatex**: Flat options architecture, EDTF dates, sorting templates
- **citeproc-rs**: Rust implementation patterns, incremental computation, WASM bindings

### Feature Roadmap (from Prior Art)

| Priority | Feature | Source | Issue |
|----------|---------|--------|-------|
| High | EDTF native date handling | biblatex | - |
| High | Locale-specific template sections | CSL-M | #66 |
| High | Entry-level `language` field | biblatex/CSL-M | #66 |
| High | Pluggable renderers (HTML, LaTeX, Typst) | citeproc-rs, jotdown | #105 |
| Medium | Presets for common configurations | CSLN-native | #89 |
| Medium | Hyperlink configuration | CSL Appendix VI | #155 |
| Medium | Separate citation/bibliography name limits | biblatex | #64 |
| Medium | Sorting shortcuts (`nty`, `ynt`) | biblatex | #61 |
| Medium | Extended legal types | CSL-M | - |
| Medium | `court-class` jurisdiction hierarchies | CSL-M | - |
| Medium | Djot integration (documents + fields) | - | #86 |
| Low | Parallel citation support | CSL-M | - |
| Low | `hereinafter` variable | CSL-M | - |
| Low | Extended position conditions | CSL-M | - |
| Low | Incremental computation (salsa) | citeproc-rs | - |

## Design Documents

Architectural decisions and design rationale:

- **[STYLE_ALIASING.md](./docs/architecture/design/STYLE_ALIASING.md)**: Style aliasing and presets strategy. Recommends presets for configuration reuse instead of CSL 1.0's parent/child aliasing. Refs: #89
- **[STYLE_EDITOR_VISION.md](./docs/architecture/design/STYLE_EDITOR_VISION.md)**: User stories and API requirements for a web-based style editor. Ensures the core library supports progressive-refinement UIs and JSON API exposure.

## Skills

Specialized expertise is available via the following skills in `.claude/skills/`:

- **[rust-pro](./.claude/skills/rust-pro/SKILL.md)**: Modern Rust engineering (1.75+), async patterns, and performance optimization. Use proactively for core processor development.
- **[git-advanced-workflows](./.claude/skills/git-advanced-workflows/SKILL.md)**: Advanced Git operations (rebasing, cherry-picking, bisecting).
- **[styleauthor](./.claude/skills/styleauthor/SKILL.md)**: LLM-driven style creation from reference materials. Iterative 5-phase workflow: research, author, test, evolve processor if needed, verify. Also available as `@styleauthor` agent for autonomous style creation.

### Style Classes
- **in-text**: 2,302 styles (80.9%) - author-date
- **note**: 542 styles (19.1%) - footnote-based

## Test Commands

See **[docs/RENDERING_WORKFLOW.md](./docs/RENDERING_WORKFLOW.md)** for detailed workflow guide.

```bash
# Run all tests
cargo test

# Recommended workflow test (structured oracle + batch impact)
./scripts/workflow-test.sh styles-legacy/apa.csl

# Run structured oracle comparison (component-level diff)
node scripts/oracle.js styles-legacy/apa.csl

# Run end-to-end migration test
node scripts/oracle-e2e.js styles-legacy/apa.csl

# Run batch analysis across top 10 styles
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10

# Legacy simple string comparison (rarely needed)
node scripts/oracle-simple.js styles-legacy/apa.csl

# Prepare for agent-assisted migration
./scripts/prep-migration.sh styles-legacy/apa.csl

# Run CSLN processor
cargo run --bin csln-processor -- styles/apa-7th.yaml

# Generate JSON Schema
cargo run --bin csln-cli -- schema > csln.schema.json

# Analyze all styles for feature usage
cargo run --bin csln-analyze -- styles-legacy/

# Build and check
cargo build && cargo clippy
```

## Issue Handling

### Domain Expert Context Packets

We use a specific issue template for Domain Experts to provide semantic context. When working on these issues:

1.  **Analyze Context First**: Read the "Domain Context", "Reference Materials", and "Real-World Examples" sections carefully.
2.  **Extract Rules**: Before writing code, explicitly state the rules you have extracted from the provided PDF/HTML references.
3.  **Identify Schema vs Logic**: Determine if the request requires a new schema field (in `csln_core`) or just a processing change (in `csln_processor`).
4.  **Verify Constraints**: Check the "Constraints" section for strict prohibitions (e.g., "Never use italics").

## Git Workflow (Rapid Development Mode)

**During rapid development, direct commits to `main` are allowed** to optimize for velocity and message economy (Pro Plan constraints). This mode is active until the project approaches production or onboards external contributors.

### Mandatory Pre-Commit Checks

**CRITICAL: Before EVERY commit to main, you MUST run:**

```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

These checks are non-negotiable:
1. **`cargo fmt`** - Format code (CI will fail without this)
2. **`cargo clippy`** - Zero tolerance for warnings
3. **`cargo test`** - All tests must pass

**If ANY check fails, DO NOT commit. Fix the issues first.**

### Commit Message Guidelines

Follow these conventions for all commits:
- **Conventional Commits**: Use `type(scope): subject` format.
- **Lowercase Subject**: Subject lines must be lowercase.
- **50/72 Rule**: Limit the subject line to 50 characters and wrap the body at 72 characters.
- **Explain What and Why**: The body should explain the rationale behind the change.
- **Issue References**: Include GitHub issue references where relevant (e.g., `Refs: #123` or `csln#64`).
- **Plain Text Body**: Do NOT use Markdown in the commit body. Uses asterisks for bullet points is okay, but do not backtick code elements.
- **No Escaped Backticks**: Never escape backticks (e.g., write `code` not \`code\`).
- **No Co-Authored-By**: Do NOT include `Co-Authored-By` footers in AI-authored commit messages.

Example:
```bash
cargo fmt && cargo clippy && cargo test && \
git add -A && git commit -m "fix(migrate): prevent duplicate list variables

Add post-processing step to detect variables appearing in both
List components and standalone components, adding suppress overrides
to prevent duplication in rendered output.

Refs: csl26-6whe, #127"
```

### When to Use Feature Branches (Optional)

Feature branches are **optional** but recommended for:
- Major architectural changes requiring extended review
- Risky experiments that might need rollback
- Changes you want to checkpoint before pushing to main

For normal bug fixes, small features, and refactoring, commit directly to main.

### Workflow Example

**Standard (direct to main):**
```bash
# 1. Make changes
# 2. Run pre-commit checks and commit
cargo fmt && cargo clippy && cargo test && git add -A && git commit -m "fix: your message"
# 3. Push to main
git push origin main
```

**Optional (feature branch for major changes):**
```bash
# 1. Create checkpoint branch
git checkout -b feat/major-change
# 2. Make changes
# 3. Run pre-commit checks and commit
cargo fmt && cargo clippy && cargo test && git add -A && git commit -m "feat: your message"
# 4. Push branch
git push -u origin feat/major-change
# 5. Optionally create PR for review, or merge locally and push to main
```

### Switching to PR Workflow

When the project reaches these milestones, switch back to mandatory PR workflow:
- Approaching production release
- Onboarding external contributors
- User requests stricter review process

At that point, restore the "NEVER commit to main" rule.

## Pull Request Convention (Optional)

**PRs are optional during rapid development.** Use them only when:
- You want feedback before merging a major architectural change
- Creating a checkpoint for later review
- Documenting complex changes with detailed PR description

**When you do create PRs:**

**Draft vs Ready:**
- **Draft PR** = Work in progress, more commits expected
- **Regular PR** = Complete and ready for immediate merge

**Branch naming conventions:**
- `feat/` - New features
- `fix/` - Bug fixes
- `refactor/` - Code refactoring
- `docs/` - Documentation changes

**Remember:** All pre-commit checks (fmt, clippy, test) must pass before creating PR.

## Coding Standards

- Use `#[serde(rename_all = "kebab-case")]` for YAML/JSON compatibility
- Use `#[non_exhaustive]` for extensible enums
- Use `#[serde(deny_unknown_fields)]` on untagged enum variants to prevent misparse
- Prefer `Option<T>` with `skip_serializing_if` for optional fields
- Add `#[serde(flatten)]` for inline rendering options
- Comment non-obvious logic; reference CSL 1.0 spec where applicable

## Priority Styles

**Important**: Do not over-optimize for any single style. Test changes against multiple parent styles to avoid regressions.

See **[docs/STYLE_PRIORITY.md](./docs/STYLE_PRIORITY.md)** for detailed impact analysis based on dependent style counts.

### Top Parent Styles by Impact

The CSL repository has ~7,987 dependent styles that alias ~300 parent styles. Prioritize by dependent count:

| Parent Style | Dependents | Format | Impact |
|-------------|------------|--------|--------|
| apa | 783 | author-date | 9.8% |
| elsevier-with-titles | 672 | numeric | 8.4% |
| elsevier-harvard | 665 | author-date | 8.3% |
| springer-basic-author-date | 460 | author-date | 5.8% |
| ieee | 176 | numeric | 2.2% |

**The top 10 parent styles cover 60% of dependent styles.**

### Development Order

1. **Author-date styles first** (40% of corpus) - APA, Elsevier Harvard, Springer, Chicago
2. **Numeric styles second** (57% of corpus) - Elsevier Vancouver, IEEE, AMA
3. **Note styles last** (2% of corpus) - Chicago Notes, OSCOLA

### Measuring Impact

When reporting progress, calculate impact as:
```
Impact = sum(dependent_count for passing parent styles) / 7987 * 100
```

### Running the Analyzer

```bash
# Rank parent styles by dependent count
cargo run --bin csln-analyze -- styles-legacy/ --rank-parents

# Filter by citation format
cargo run --bin csln-analyze -- styles-legacy/ --rank-parents --format author-date --json
```,old_string:
