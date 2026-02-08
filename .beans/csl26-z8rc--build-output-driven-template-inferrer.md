---
# csl26-z8rc
title: Build output-driven template inferrer
status: completed
type: feature
priority: normal
created_at: 2026-02-08T00:39:18Z
updated_at: 2026-02-08T00:39:30Z
blocking:
    - csl26-m3lb
    - csl26-l05b
    - csl26-qb6h
---

Build a template inference engine that generates CSLN bibliography templates from citeproc-js output, for parent styles beyond the hand-authored top 10.

Architecture (~500-800 lines):
- Run citeproc-js with expanded test fixtures per style
- Use hardened oracle.js component parser to extract structured components
- Cross-reference extracted components with input reference data to map variables
- Compare outputs across reference types to infer type-specific suppress overrides
- Generate CSLN YAML template with correct component ordering and overrides
- Merge with XML-extracted options to produce complete style

Dependencies:
- csl26-l05b: Expanded test fixtures (need diverse reference types)
- csl26-qb6h: Hardened oracle.js parser (need reliable component extraction)

Key risks:
- Delimiter inference between components is non-trivial
- Formatting (italics/bold) detection depends on output format
- Compensating errors if CSLN processor has bugs
- Non-deterministic: different test data may produce different templates