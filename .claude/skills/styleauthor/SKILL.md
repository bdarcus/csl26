# Style Author (Legacy Alias)

**Type:** User-Invocable, Agent-Invocable
**LLM Access:** Yes
**Purpose:** Legacy alias for `/style-evolve` to preserve backward compatibility.

## Status
Use `/style-evolve` for all new human-facing workflows.

## Behavior
Forward all requests to `../style-evolve/SKILL.md`.

## Mapping
- `/styleauthor upgrade ...` -> `/style-evolve upgrade ...`
- `/styleauthor migrate ...` -> `/style-evolve migrate ...`
- `/styleauthor create ...` -> `/style-evolve create ...`

## Notes
- Keep this alias for compatibility and gradual migration.
- New documentation should reference `/style-evolve`, not `/styleauthor`.

## Standard Templates
- `./templates/simple-migration-checklist.md`
- `./templates/migrate-enhance-checklist.md`
- `./templates/update-checklist.md`
- `./templates/common-patterns.yaml`
- `./templates/style-spec.md`
