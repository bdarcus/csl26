# Documentation Context

Working on **documentation, architecture docs, or design proposals**.

## Documentation Structure
```
README.md                           # Project overview and quickstart
CLAUDE.md                           # Agent instructions and project context
docs/
  RENDERING_WORKFLOW.md             # How to run and verify rendering
  STYLE_PRIORITY.md                 # Parent style impact rankings
  TASKS.md                          # Task tracking and roadmap
  TIER_STATUS.md                    # Feature tier completion status
  WORKFLOW_ANALYSIS.md              # Development workflow analysis
  BIBLATEX_MAPPING.md               # BibLaTeX field mapping reference
  architecture/
    MIGRATION_STRATEGY_ANALYSIS.md  # Hybrid migration strategy
    PERSONAS.md                     # Design personas for evaluation
    PRIOR_ART.md                    # CSL 1.0, CSL-M, biblatex, citeproc-rs
    design/
      STYLE_ALIASING.md             # Presets vs parent/child aliasing
      STYLE_EDITOR_VISION.md        # Web-based style editor vision
      PUNCTUATION_NORMALIZATION.md  # Punctuation handling rules
      TEST_STRATEGY.md              # Testing philosophy
```

## Conventions
- Evaluate designs against the [Personas](../../docs/architecture/PERSONAS.md)
- Ground proposals in [Prior Art](../../docs/architecture/PRIOR_ART.md)
- Reference GitHub issues with `Refs: #123` or `csln#64` syntax
- Use hyphens (-) for list items, not asterisks
- Link to related design docs when proposing changes
