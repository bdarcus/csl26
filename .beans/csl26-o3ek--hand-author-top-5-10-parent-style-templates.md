---
# csl26-o3ek
title: Hand-author top 5-10 parent style templates
status: todo
type: task
priority: high
created_at: 2026-02-08T00:39:09Z
updated_at: 2026-02-08T00:43:18Z
blocking:
    - csl26-m3lb
---

Hand-author CSLN bibliography templates for the highest-impact parent styles, using examples/apa-style.yaml as the gold standard model and style guides as primary source.

Target styles (by dependent count):

1. apa (783 dependents) - DONE (examples/apa-style.yaml)
2. elsevier-with-titles (672)
3. elsevier-harvard (665)
4. springer-basic-author-date (460)
5. ieee (176)
6. elsevier-vancouver (163)
7. american-medical-association (114)
8. nature (76)
9. cell (57)
10. chicago-author-date (55)

Approach:

- Read the style guide or representative output for each style
- Author the bibliography template section as CSLN YAML
- Verify against oracle comparison (node scripts/oracle.js)
- Each style produces ~10-15 template components

These 10 styles cover ~60% of dependent styles. Combined with the working XML options pipeline, this should achieve high bibliography match rates.

~5-10 hours of domain-expert time.

**AI-Assisted Authoring**

LLMs can accelerate this significantly. A workflow could look like:

1. Feed the LLM the existing apa-style.yaml as a model of the target format
2. Provide citeproc-js oracle output for the target style (multiple reference types)
3. Provide the CSL 1.0 source XML for additional context (conditional branches, locale terms)
4. Ask it to generate a CSLN bibliography template matching the observed output

This is essentially what a human author does: read example output, understand the pattern, write the declarative template. LLMs are well-suited to this because:
- The CSLN YAML format is human-readable and well within LLM capabilities
- The task is pattern-matching from examples, not algorithmic reasoning
- The APA gold standard provides a strong few-shot example
- Oracle verification provides a concrete pass/fail check

Practical considerations:
- Use oracle.js output as ground truth, not the LLM's knowledge of style guides (styles evolve, LLM training data may be outdated)
- Human review is still essential for edge cases (locale terms, rare types, suppress logic)
- Could batch-generate drafts for all 10 styles, then manually refine failures
- The XML source provides context the output alone cannot (e.g., which terms are locale-dependent vs hardcoded)

This could reduce the ~5-10 hours of domain-expert time to ~2-3 hours of review and refinement.