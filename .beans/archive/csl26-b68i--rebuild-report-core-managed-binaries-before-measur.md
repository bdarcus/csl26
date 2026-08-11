---
# csl26-b68i
title: Rebuild report-core managed binaries before measurement
status: completed
type: bug
priority: high
tags:
    - reporting
    - harness
    - fidelity
created_at: 2026-08-09T18:48:19Z
updated_at: 2026-08-11T18:41:53Z
---

The 2026-08-09 shortened-notes audit ran report-core twice at source revision 0b2e44f9 with empty cache directories. The first default-feature run reused target/debug/citum and reported 20/473; after cargo build -q --bin citum and an explicit --citum-bin, the same source and fixtures reported 34/473. Fresh report data caches do not protect against stale managed binaries, and the report cache provenance does not include a binary/source identity. Make the default managed-binary path ask Cargo to validate the binary, include a renderer identity in cache/provenance, and add a regression test. Evidence: docs/architecture/audits/2026-08-09-shortened-notes-coverage/authority-report.json.



## Summary of Changes

Made every managed report-core binary path run `cargo build -q --bin citum` before measurement while preserving explicit binary overrides. Added regression coverage for an existing default-feature binary. Existing cache keys already include `citumBinHash`, so stale renderer outputs are no longer silently reused after validation.
