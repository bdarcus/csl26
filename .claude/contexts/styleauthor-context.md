# Style Author Context

Working on **creating or updating CSLN citation styles** (YAML files).

## Workflow
Human-facing entrypoint:
- Use `/style-evolve` for all style workflows.
- Treat `/styleauthor` as a legacy alias only.

Internal routing:
- `style-maintain` for single-style fixes and updates
- `style-migrate-enhance` for batch migration waves
- `style-qa` as required verification gate before completion

## Gold-Standard Reference
- `styles/apa-7th.yaml` — the canonical example style (5/5 citation + bibliography match)

## Style File Location
- `styles/` — CSLN YAML styles (production)
- `styles-legacy/` — 2,844 CSL 1.0 XML styles (submodule, read-only reference)

## Key Concepts
- **Three-tier options**: Global (`options:`), citation-specific (`citation.options:`), bibliography-specific (`bibliography.options:`)
- **Template overrides**: Type-specific rendering via `overrides:` on components
- **Declarative templates**: Flat component lists, no procedural `<choose>/<if>` logic
- **Component types**: See `crates/csln_core/src/template.rs` for the full catalog

## Agent-Assisted Migration
```bash
# Prepare high-fidelity context for the @styleauthor agent
./scripts/prep-migration.sh styles-legacy/apa.csl
```

## Verification
```bash
# Run oracle comparison after authoring
node scripts/oracle.js styles-legacy/<style>.csl
./scripts/workflow-test.sh styles-legacy/<style>.csl
```

## Reference Docs
- [STYLE_PRIORITY.md](../../docs/STYLE_PRIORITY.md) — which styles to prioritize
- [PERSONAS.md](../../docs/architecture/PERSONAS.md) — who uses styles
- [PRIOR_ART.md](../../docs/architecture/PRIOR_ART.md) — how other systems handle styles
