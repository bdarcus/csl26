# Migration Context

Working on the **CSL 1.0 → CSLN conversion** pipeline.

## Hybrid Strategy
1. **XML pipeline for options** — Extract global options (name formatting, et-al rules, dates, locales) from CSL 1.0 XML. Achieves 87-100% citation match.
2. **Output-driven template generation** — Use citeproc-js output + input data cross-referencing for template structure and ordering.
3. **LLM-authored templates** — For top parent styles, use `/style-evolve migrate` with `prep-migration.sh` context (implemented by the style agents).
4. **XML compiler as fallback** — For rare reference types and validation.

## Key Crates
| Crate | Role |
|-------|------|
| `csln_migrate` | Hybrid migration: XML options extractor + template generator |
| `csl_legacy` | CSL 1.0 XML parser |

## Key Scripts
| Script | Purpose |
|--------|---------|
| `scripts/batch-infer.sh` | Batch template inference across styles |
| `scripts/infer-template.js` | Single-style template inference |
| `scripts/prep-migration.sh` | Prepare context for agent-assisted migration |
| `scripts/lib/template-inferrer.js` | Core inference logic |

## Priority Styles
The top 10 parent styles cover 60% of dependent styles. See [STYLE_PRIORITY.md](../../docs/STYLE_PRIORITY.md).

| Style | Dependents | Format |
|-------|------------|--------|
| apa | 783 | author-date |
| elsevier-with-titles | 672 | numeric |
| elsevier-harvard | 665 | author-date |
| springer-basic-author-date | 460 | author-date |

## Reference Docs
- [MIGRATION_STRATEGY_ANALYSIS.md](../../docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md)
- [STYLE_PRIORITY.md](../../docs/STYLE_PRIORITY.md)
