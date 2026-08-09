---
# csl26-hk3u
title: Complete the Coverage-Audit Stack
status: in-progress
type: feature
priority: high
created_at: 2026-08-09T20:38:17Z
updated_at: 2026-08-09T21:07:26Z
---

Integrate registered style coverage audits into report metadata, compat.html, CI freshness enforcement, and shared style workflows.

Related: docs/specs/STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md and docs/specs/STYLE_COMPATIBILITY_INHERITANCE_REPORT.md

## Acceptance Criteria

- [x] Registered audits are source-built, schema-validated, identity-checked, partition-checked, hash-checked, and byte-reproducible in CI.
- [x] Report JSON exposes registered audit data and compat.html renders the audit-first explorer without changing unaudited styles.
- [x] Shared style policy, execution guide, wrappers, and evaluations enforce freshness-aware audit use without requiring audits for every style.
- [ ] Generated report and all targeted, infrastructure, frontmatter, visual, and pre-push checks pass.
- [x] The exported jj change is one conventional commit on codex/style-coverage-workflow-integration.


Verification note: automated responsive/accessibility assertions pass, but the in-app browser exposed no browser instance for live desktop/mobile inspection.
