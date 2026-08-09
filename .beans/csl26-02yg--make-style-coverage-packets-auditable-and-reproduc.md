---
# csl26-02yg
title: Make style coverage packets auditable and reproducible
status: todo
type: task
priority: high
tags:
    - qa
    - harness
    - coverage
created_at: 2026-08-09T14:41:10Z
updated_at: 2026-08-09T14:41:10Z
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

- [ ] Audit manifest schema and validation are implemented.
- [ ] Observation identities are stable and unique.
- [ ] Relevance, intentional omission, and comparability are explicit.
- [ ] Packet generation is complete, deterministic, and freshness-tested.
- [ ] Supplied parity evidence produces joined non-null exact-match rows.
- [ ] Resolver evidence comes from the engine or passes conformance fixtures.
