# Contributing to CSLN

CSLN is an AI-first project where contributions span domain expertise, style authoring, and systems engineering. The most impactful contributions come from domain experts and style authors who provide context for the AI to work with, rather than from systems programmers alone.

## How to Contribute

### 1. Report Issues and Surface Real-World Gaps

Submit a GitHub issue describing citation formatting requirements or edge cases that existing systems handle poorly. Include:
- The style guide or official manual
- Sample references and expected output
- Current system output (CSL 1.0, CSL-M, or CSLN)

These become high-value context packets for our AI agents.

### 2. Provide Contextual Resources

- Share style guides, official manuals, and sample documents
- Include PDFs or images showing expected formatting
- Provide structured reference data (BibTeX, JSON, or YAML)

This material allows LLM agents to extract logic and implement it correctly.

### 3. Hand-Author Styles (with LLM Assistance)

For high-impact parent styles (APA, Chicago, IEEE, Vancouver, etc.), you can help author gold-standard CSLN templates:

**Workflow:**
```bash
# 1. Prepare migration context
./scripts/prep-migration.sh styles-legacy/apa.csl

# 2. Use the /styleauthor skill with the provided context
#    (The agent reads the style guide + output examples + reference data)
/styleauthor styles-legacy/apa.csl

# 3. Test iteratively against citeproc-js
node scripts/oracle.js styles-legacy/apa.csl

# 4. Submit the YAML style as a pull request
#    Example: styles/apa-7th.yaml (5/5 citation + bibliography match)
```

See `.claude/skills/styleauthor/SKILL.md` for detailed workflow and validation checklist.

### 4. Improve Documentation and Examples

- Expand `docs/` with tutorials and use cases
- Add examples to `docs/examples.html`
- Clarify error messages and schema validation

Use the `/humanizer` skill for technical documentation to ensure clarity and readability.

### 5. Expand Test Coverage

The test fixture in `tests/fixtures/references-expanded.json` powers oracle verification:

- Add rare reference types (legal, patent, dataset, etc.)
- Include multilingual examples
- Provide edge cases (missing dates, author variations)

Expanded fixtures improve inference accuracy for all approaches.

## Code Contributions

For systems programmers contributing directly to the Rust codebase:

### Core Components

**Highest priority:**
- `csln_processor/` - Citation/bibliography rendering engine
- `csln_core/src/template.rs` - CSLN schema and type system
- `scripts/` - Oracle verification and template inference scripts

**Lower priority** (mostly stable):
- `csln_migrate/` - Migration infrastructure
- `csl_legacy/` - CSL 1.0 parser (read-only, feature-complete)

### Before Submitting a PR

1. Install nextest for faster parallel test execution (optional but recommended):
   ```bash
   cargo install cargo-nextest
   ```

2. Run pre-commit checks:
   ```bash
   cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run
   ```

   If nextest is not installed, fall back to:
   ```bash
   cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
   ```

3. Verify against oracle (for rendering changes):
   ```bash
   node scripts/oracle.js styles-legacy/apa.csl
   ```

4. Follow [Conventional Commits](https://www.conventionalcommits.org/) format:
   ```
   type(scope): lowercase subject

   Explain the rationale and impact.
   ```

### Code Standards

- Maximum 300 lines per file, 50 lines per function
- Strict typing (no `unwrap`, no `any`, no `unsafe`)
- Idiomatic error handling (Result types, not panics)
- Well-commented, especially for non-obvious logic

## Task Workflow

Active development uses [beans](https://github.com/jdx/beans) for local task tracking. View current work:

```bash
/beans list              # All tasks
/beans next              # Recommended next task
/beans show BEAN_ID      # Task details
```

## Review Process

For pull requests:

1. All pre-commit checks must pass
2. Oracle verification validates rendering correctness (if applicable)
3. Maintainers review for design alignment with [PERSONAS.md](.agent/PERSONAS.md)

This is an AI-first workflow, so expect feedback emphasizing clarity, explicitness, and modularity.

## Community

- **GitHub Issues** - Bug reports, feature requests, discussions
- **GitHub Discussions** - Design proposals, RFC (Request for Comment)
- **Email** - Contact the maintainer directly for confidential domain expertise

## License

All contributions are licensed under MPL-2.0. See [LICENSE](LICENSE) for details.
