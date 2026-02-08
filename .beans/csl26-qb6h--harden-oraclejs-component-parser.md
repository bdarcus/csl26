---
# csl26-qb6h
title: Harden oracle.js component parser
status: todo
type: task
priority: high
created_at: 2026-02-08T00:38:59Z
updated_at: 2026-02-08T00:38:59Z
blocking:
    - csl26-m3lb
---

The oracle.js parseComponents() function uses fragile heuristics that must be hardened before serving as a template inference foundation:

Current fragility:
- Publisher detection: 10-char prefix substring matching
- Container-title: 15-char prefix matching
- Title: 20-char prefix matching
- findRefDataForEntry(): returns first author match even when multiple refs share an author
- Volume-only detection can't distinguish volumes from other numbers

Required changes (~300-500 lines):
- Replace substring prefix matching with full-field matching against known input data
- Use input reference data as primary matching source (not regex on output)
- Handle ambiguous matches (multiple refs with same author) via secondary field validation
- Add delimiter detection between matched components
- Add formatting detection (italics/bold) if output preserves markup

This is a prerequisite for the output-driven template inferrer.