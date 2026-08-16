---
# csl26-yugl
title: 'Audit pre-migration-fix embedded styles for missing no-date fallback: []'
status: todo
type: task
priority: normal
tags:
    - styles
    - rendering
    - dates
    - migrate
created_at: 2026-08-16T10:48:13Z
updated_at: 2026-08-16T10:48:22Z
---

Every embedded style migrated before commit 63fdb104 (2026-07-16, when citum-migrate began auto-emitting fallback: [] on issued-date components) is a candidate for the same latent divergence fixed in csl26-7z59 for taylor-and-francis-council-of-science-editors-author-date: an undated reference renders an implicit n.d./no date term where real citeproc-js renders nothing, because the style's date: issued components predate the migrate fix and carry no fallback: key.

grep -rn 'fallback: \[\]' styles/ currently returns nothing across the whole shipped corpus, confirming this is corpus-wide, not isolated.

This is NOT a blanket find-and-replace: each candidate style's real .csl source must be checked individually first. Some styles genuinely author an explicit no-date fallback branch (e.g. an <if variable="issued">...<else><text term="no date"/></else></if> shape) and should keep the implicit engine term; only styles whose real macro has no fallback branch at all (matching the T&F CSE year-date shape) should get fallback: [].

## Investigation needed
- [ ] Identify all embedded styles migrated before 63fdb104 (git log the embedded style YAML files, filter by first-migration commit date).
- [ ] For each, grep the real .csl for the citation/bibliography year macro's fallback shape (if/else with a no-date term vs. bare <date> with no fallback).
- [ ] Style-by-style: add fallback: [] only where the real .csl has no fallback branch; leave others untouched.
- [ ] Re-run report-core.js per affected style before/after to confirm exactParity improvement with no fidelity-gate regression.

See div-016 in docs/adjudication/DIVERGENCE_REGISTER.md and bean csl26-7z59 for the mechanism and the T&F CSE precedent.
