---
# csl26-629e
title: 'Migrate hand-rolled DOI/URL prefixes onto declarative links: config'
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:14:41Z
updated_at: 2026-08-07T13:14:41Z
---

Declarative link support already exists and works — this is a migration gap,
not a missing feature. Origin: https://github.com/bdarcus/csln/issues/155
(the predecessor project's original ask: `target: url-or-doi`, DOI-derived
URL construction, other DOI-like identifiers).

## What already exists
- Schema: `LinksConfig { doi, url, target: LinkTarget, anchor: LinkAnchor,
  strip_protocol }` — crates/citum-schema-style/src/options/mod.rs:605-655.
- `LinkTarget` covers exactly the issue's ask plus more:
  `Url | Doi | UrlOrDoi | Pubmed | Pmcid`.
- `LinkAnchor` covers `Title | Url | Doi | Component | Entry`.
- Engine resolution is real, not a stub — crates/citum-engine/src/values/mod.rs:718-737:
  `UrlOrDoi` is the *default* target and correctly builds `https://doi.org/{doi}`
  from a bare DOI when no URL is present.
- In use today: elsevier-with-titles-core, elsevier-harvard-core,
  elsevier-vancouver-core, gb-t-7714-2025-* (`target: doi`), ieee,
  chicago-shortened-notes-bibliography, american-medical-association,
  springer-basic-author-date-core.

## The gap
Most embedded styles don't use it and instead hand-roll the DOI prefix as a
literal template string. Portfolio count of literal `"https://doi.org/"`
prefix variants: ~38 sites (`". https://doi.org/"` x20, `", https://doi.org/"`
x8, bare x10), across far more files than the 9 using the declarative form.
The Chicago family alone carries 15 of these
(chicago-author-date-18th.yaml, taylor-and-francis-chicago-author-date-core.yaml)
— this is exactly "DOI prefix convention" item #5 in
docs/architecture/audits/2026-06-30_CHICAGO_FAMILY_AUDIT.md Section A, which
recommended a shared DOI component; that recommendation was never connected to
the `links:` config that already existed to serve it.

## Scope
- Audit the ~38 hand-rolled sites; convert to `links: { target: url-or-doi }`
  (or `target: doi` where a style is DOI-only) + `anchor:` where each site's
  URL/DOI branching logic is genuinely just picking a target — not where a
  style has a real structural reason to keep them separate (verify per site,
  same rule as the locale-message conversions in the Chicago cluster work).
- Chicago family conversions should land as part of (or immediately after) the
  Chicago cluster-driven rewrite (see docs/specs/CHICAGO_FAMILY_STRATEGY.md,
  csl26-h7oc) rather than duplicated separately.
- Re-check the issue's "other DOI-like URLs, like pubmed" ask against
  `Pubmed`/`Pmcid` — confirm those two targets are exercised by at least one
  style/fixture, not just implemented and untested.
- Consider whether a comment back on csln#155 (closing the loop on the
  predecessor project) is warranted once this lands.

YAML-only migration; no engine/Rust change expected. Verify via
report-core.js exact-parity, same as any other style-YAML change.
