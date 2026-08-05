---
# csl26-s761
title: citation label-separator is absent from CitationOptions schema
status: scrapped
type: bug
priority: normal
created_at: 2026-08-05T18:01:56Z
updated_at: 2026-08-05T18:04:25Z
---

styles/numeric-comp.yaml sets citation.options.label-separator, which is not a property of CitationOptions, so the style fails schema validation with 'must NOT have unevaluated properties'. Pre-existing, unrelated to type selectors.

Found while wiring scripts/validate-schemas.js into CI. numeric-comp.yaml is skipped in that script pending this fix; remove the skip entry when it lands.

Same class as the reference-type vocabulary drift fixed alongside csl26-q4g5: an authored key the engine tolerates and the published schema does not describe. Determine whether label-separator is a real option that should be added to CitationOptions, or a stale key the style should drop.

## Reasons for Scrapping

Not a bug worth tracking: `bibliography.options` already declared the same `label-separator: '. '`, and `CitationOptions` has no such field, so the `citation.options` copy was inert. Removed the duplicate key in the same change that wired `scripts/validate-schemas.js` into CI.
