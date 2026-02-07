---
# csl26-iek4
title: Add baseline tracking for regression detection
status: todo
type: feature
priority: high
created_at: 2026-02-07T06:44:23Z
updated_at: 2026-02-07T06:44:23Z
---

Implement baseline snapshots to detect rendering regressions automatically.

Store oracle outputs for each style and compare against baseline on changes.

This prevents silent regressions during refactoring.

Refs: GitHub #125