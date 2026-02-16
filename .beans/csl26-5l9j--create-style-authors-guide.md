---
# csl26-5l9j
title: Create style authors guide
status: todo
type: task
priority: normal
created_at: 2026-02-07T06:53:44Z
updated_at: 2026-02-07T07:40:14Z
blocking:
    - csl26-c7rf
---

Create documentation for the style author persona explaining how to use the new style model.

Focus:
- YAML option and syntax
- Setting up IDE for autocomplete and validation
- Highlight options and presets
- Make sure it 100% accurately represents the code
- Link to style-hub repo for style wizard integration

**Partial coverage already exists:**
- `/styleauthor` skill (.claude/skills/styleauthor/SKILL.md) - LLM-driven 5-phase workflow
- Style spec template (.claude/skills/styleauthor/templates/style-spec.md) - captures formatting rules
- Common patterns (.claude/skills/styleauthor/templates/common-patterns.yaml) - reusable YAML snippets
- Gold standard example (examples/apa-7th.yaml) - annotated APA 7th style

This guide should build on these resources, providing a human-readable narrative for style authors who may not use LLM tooling.

Target audience: Style authors from docs/architecture/PERSONAS.md
Output: docs/guides/style-authoring.md
Effort: 1 week

Refs: GitHub #143, GitHub #96