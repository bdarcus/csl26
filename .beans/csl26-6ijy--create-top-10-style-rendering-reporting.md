---
# csl26-6ijy
title: Create Top-10 style rendering reporting
status: completed
type: task
priority: normal
created_at: 2026-02-18T04:39:43Z
updated_at: 2026-02-18T05:26:55Z
---

# Problem statement

We need an easy and consistent way to instantly see how well the Top 10 styles render.

The standard here is no longer just fidelity to CSL 1.0 via the "Oracle" output, but also to the actual expectations of the style.
For example, we know CSLN supports mode-dependent conjunctions in citations, but 1.0 does not. 
While it may be difficult to determine this generally, feasability should be addressed.

# Proposed Features

It should have a JSON output mode optimized for tools, and HTML for humans. 
For the HTML rendering, consider a table with stats, and a tabbed set of examples of different citation output, and diverse bib reference types. 

It should also include some metric of overall fidelity, but if it's not "perfect", identify what the problems are. 

Propose as well how this output will be generated, AND auto-updated for the website. 
