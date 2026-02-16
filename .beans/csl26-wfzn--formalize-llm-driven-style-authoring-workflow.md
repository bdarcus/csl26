---
# csl26-wfzn
title: Formalize LLM-driven style authoring workflow
status: completed
type: feature
priority: high
created_at: 2026-02-08T12:00:00Z
updated_at: 2026-02-08T12:00:00Z
---

Formalize the LLM-driven style authoring workflow validated during APA 7th creation into a reusable skill and agent.

## Deliverables

* Skill: .claude/skills/styleauthor/SKILL.md - 5-phase iterative workflow (research, author, test, evolve, verify)
* Agent: .claude/agents/styleauthor.md - autonomous style creation with iteration cap and regression guards
* Template: .claude/skills/styleauthor/templates/style-spec.md - style specification template
* Patterns: .claude/skills/styleauthor/templates/common-patterns.yaml - reusable YAML snippets
* Updated CLAUDE.md with skill reference
* Updated docs/architecture/PERSONAS.md with LLM-assisted authoring sub-persona
