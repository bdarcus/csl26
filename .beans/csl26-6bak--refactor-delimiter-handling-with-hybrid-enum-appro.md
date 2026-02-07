---
# csl26-6bak
title: Refactor delimiter handling with hybrid enum approach
status: todo
type: feature
priority: high
created_at: 2026-02-07T06:44:21Z
updated_at: 2026-02-07T12:11:38Z
parent: csl26-u1in
---

Current delimiter handling is scattered across the codebase.

Propose hybrid enum approach that combines predefined delimiters (comma, period, space) with custom text variant.

This will simplify the code and make delimiter logic more explicit.

Refs: GitHub #126