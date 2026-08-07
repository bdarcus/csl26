---
# csl26-ey4f
title: 'Cluster 1: contributor-role and role-pattern localization'
status: completed
type: task
priority: high
created_at: 2026-08-07T13:20:37Z
updated_at: 2026-08-07T14:35:33Z
parent: csl26-h7oc
---

Convert hardcoded English role-label prefixes (~44 sites in chicago-author-date-18th.yaml, taylor-and-francis-chicago-author-date-core.yaml) to locale-driven verb forms and message: pattern.chicago-* calls, per docs/policies/LOCALIZATION_INTEGRITY.md. In progress under csl26-dfq0 in this session; see that bean for the working todo list. Includes 6 new en-US.yaml locale entries and wrap:parentheses cleanups on the same files.

## Progress (2026-08-07)

Converted all hardcoded role-label/pattern sites in chicago-author-date-18th.yaml
(25 sites) and taylor-and-francis-chicago-author-date-core.yaml (11 sites) to
locale-driven verb forms / message: pattern.chicago-* calls, per
docs/policies/LOCALIZATION_INTEGRITY.md. Verified: 0 entries changed against
either style's own pre-edit baseline (author-date 172/546 exact parity, 63/63
citations; T&F 172/546, 63/63 — both unchanged, zero regressions, zero masked
cancellations — checked entry-by-entry, not just aggregate counts).

Two real findings surfaced mid-conversion, handled per the plan's exception
policy rather than forced through:
- Two genuine pre-existing bugs found via oracle comparison (not localization
  issues): a duplicated "Released. Released." in the software type-variant
  (fixed — the redundant prefix, not the date-based one, was removed) and
  `contributor: author` reused with a "Directed by" prefix in T&F's
  motion-picture block (role/data mismatch, left as a documented STYLE010
  exception — fixing it would change which reference field is read, a
  correctness question out of scope here).
- `variable: version` ("V. ") has no clean locale attachment path (`variable:`
  components have no label-form mechanism; `number: version` isn't a valid
  NumberVariable) — documented STYLE010 exception.
- "Track " (chapter-number repurposed for track numbering) has no clean
  attachment either — documented exception, noted for csl26-rpza.

**One real engine bug found and fixed (Rust, in this same stack per explicit
user instruction — "add Narrator; this is the kind of thing you should NOT be
deferring, when it's easy to add"):** `citum-schema-style::template::ContributorRole`
was missing a `Narrator` variant entirely (had Performer, Illustrator, Writer,
but not Narrator), and `raw_conversion.rs`'s `parse_role_name` didn't recognize
"narrator" as a locale role key either — so the locale's `roles: narrator:`
entry was never parsed into the roles map, and `form: verb` on
`contributor: narrator` silently rendered the bare name with no label at all
(reproduced in isolation via `citum render refs`, fixed with a 4-line diff
across template.rs/mod.rs/substitute.rs/raw_conversion.rs, full workspace
`cargo nextest run` 2420/2420 passing, fmt+clippy clean, schemas regenerated).

Multilingual proof (the point of the whole exercise) confirmed:
`citum render refs -L fr-FR`/`-L de-DE` — narrator "Narrated by" -> "Lu par";
translator "Translated by" -> "Übersetzt von" (de-DE) / "Traduit par" (fr-FR,
T&F). Full portfolio non-regression check in progress.

## Summary of Changes

Converted all hardcoded English role-label/date-pattern strings in
chicago-author-date-18th.yaml (25 sites) and
taylor-and-francis-chicago-author-date-core.yaml (11 sites) to locale-driven
`form: verb` / `message: pattern.chicago-*` calls, per
docs/policies/LOCALIZATION_INTEGRITY.md. 3 documented STYLE010 exceptions
remain (variable:version "V.", chapter-number-as-track "Track ", T&F's
author-credited-as-director "Directed by") — each has an inline comment
explaining why no clean locale mechanism attaches, verified not silently
dropped.

Verified zero regression, entry-by-entry against each style's own baseline:
author-date 172/546 exact parity (0 changed), T&F 172/546 (0 changed), both
63/63 citations. Portfolio-wide: check-core-quality.js gate passed (19
embedded-core baseline styles, 0 exact-parity regressions). Full workspace
cargo nextest run: 2420/2420. Multilingual proof: citum render refs -L
fr-FR/-L de-DE shows narrator/translator role labels correctly localized
("Narrated by" -> "Lu par"; "Translated by" -> "Übersetzt von"/"Traduit par").

Found and fixed one real engine bug along the way (Rust, per explicit
instruction not to defer it): citum-schema-style::template::ContributorRole
was missing a Narrator variant (had Performer/Illustrator/Writer, not
Narrator), and raw_conversion.rs's parse_role_name didn't recognize
"narrator" as a locale role key, so the locale's roles: narrator: entry never
made it into the parsed roles map — form: verb on contributor: narrator
silently rendered no label at all. Fixed across template.rs, mod.rs,
substitute.rs, raw_conversion.rs (4 lines); schemas regenerated via just
schema-gen.

Also found and fixed a real pre-existing duplication bug unrelated to
localization: the software type-variant rendered "Released {date}. Released.
{medium}." (doubled "Released") — confirmed via citeproc-js oracle comparison
on the Gran Turismo 7 fixture item, which wants only one "Released" clause.
