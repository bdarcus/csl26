---
# csl26-02yg
title: Make style coverage packets auditable and reproducible
status: completed
type: task
priority: high
tags:
    - qa
    - harness
    - coverage
created_at: 2026-08-09T14:41:10Z
updated_at: 2026-08-09T17:54:39Z
---

Build the coverage packet contract specified by
docs/specs/STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md. The packet must be
auditable without trusting aggregate fidelity or an uncommitted working tree.

Required work:

- define a versioned audit manifest with style-chain, fixture, authority, and
  report hashes;
- assign a stable observation ID to every style, surface, fixture, reference,
  type, and field combination;
- record render disposition separately from comparison eligibility;
- declare relevant fields and intentional omissions with rationale;
- join exact-parity evidence by stable identity and reject a supplied report
  when every exact-match value is null;
- use deterministic codepoint ordering and emit the complete observation set;
- add a freshness test that regenerates committed evidence byte for byte;
- prefer an engine-produced resolved-template trace over a second resolver;
  if a mirror remains necessary, cover it with shared conformance fixtures.

The 2026-08-09 shortened-notes pilot is motivating evidence, not an accepted
baseline. Its working-tree provenance, missing row identity, broken parity
join, and truncated Markdown output must be fixed before its 23/482 parity or
119 uncovered observations are used as gates.

- [x] Audit manifest schema and validation are implemented.
- [x] Observation identities are stable and unique.
- [x] Relevance, intentional omission, and comparability are explicit.
- [x] Packet generation is complete, deterministic, and freshness-tested.
- [x] Supplied parity evidence produces joined non-null exact-match rows.
- [x] Resolver evidence comes from the engine or passes conformance fixtures.
- [x] Audited source revisions remain pinned while clean later revisions can regenerate packets.

Validation: all 259 JavaScript script tests pass, including byte-for-byte golden freshness and
the distinct audited-source/generator-revision provenance case. The repository Rust gate also
passes with 2,447 tests on the stacked child.
