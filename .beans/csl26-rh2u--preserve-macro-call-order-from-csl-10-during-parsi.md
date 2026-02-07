---
# csl26-rh2u
title: Preserve macro call order from CSL 1.0 during parsing
status: todo
type: feature
priority: high
created_at: 2026-02-07T19:52:56Z
updated_at: 2026-02-07T19:52:56Z
blocking:
    - csl26-ifiw
---

The template compiler loses component order during tree traversal of conditionals. Components appear in the order they are first encountered, not the order macros are called in CSL 1.0 bibliography layout.

**Problem:**
- Oracle (CSL 1.0 APA): contributors → year → title
- CSLN output: title → contributors → year

**Root Cause:**
collect_occurrences traverses then_branch, else_if, else_branch sequentially. The first occurrence of each variable determines its position in the output. This doesn't match CSL 1.0 macro call order.

**Proposed Solution:**
1. During CSL 1.0 parsing/upsampling, track macro call order in bibliography layout
2. Pass this order through to template compilation
3. Use the tracked order to arrange final components

**Alternative:**
Infer order from the DEFAULT branch components (else_branch or no-condition path) since that represents the common case.