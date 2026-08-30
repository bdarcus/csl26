---
# csl26-7yxy
title: Chicago styles hardcode English prose/terms in options.messages
status: completed
type: task
priority: normal
tags:
    - chicago
    - style
    - localization
created_at: 2026-08-26T22:30:21Z
updated_at: 2026-08-30T18:24:59Z
parent: csl26-awlo
---

13 pattern.chicago-* message definitions with literal English values across 4 style files (chicago-author-date-18th.yaml:36, chicago-notes-18th.yaml:38, chicago-shortened-notes-bibliography-core.yaml:38, taylor-and-francis-chicago-author-date-core.yaml:11), 3 keys duplicated verbatim across 3 files. Invisible to STYLE010 (scripts/style-structure-lint.js), which only scans prefix:/suffix: lines, not options.messages blocks — csl26-dfq0's localization sweep never saw this class. Two fixes: (1) chicago-volume/-volume-lower/chicago-version duplicate existing locale terms (term.volume.short etc, locales/en-US.yaml:846) — delete the messages, use number: volume + label-form: short instead (resolve_number_label in values/number.rs:110). (2) chicago-of/chicago-of-number/chicago-episode are genuine prose — move to locales/en-US.yaml's existing pattern.chicago-* run (line 1048), not the en-US-chicago override (author-date/notes don't set locale-override). Add STYLE011 lint rule for this class (report-only, like STYLE010). Acceptance bar per dfq0 precedent: 0 oracle entries changed, plus -L de-DE showing translated output. Independent of csl26-awlo's range/collapse redesign — no Rust changes, can land in parallel. Related: csl26-dfq0, csl26-boha (migrate-side emission, distinct).

## Summary of Changes

Implemented with one correction to the bean's proposed mechanism: `chicago-volume`/`chicago-volume-lower` route through `message: term.volume` (not `number: volume, label-form: short`) because `text-case` on a number component only affects the numeric value, not its label prefix — `message: term.volume` is the mechanism already proven in `apa-7th.yaml`. `chicago-version` uses the equivalent `message: term.version` (added `version` to `ALLOWED_TERM_MESSAGE_IDS` in `style-structure-lint.js`, matching the existing `volume` entry).

- `chicago-volume`/`chicago-volume-lower`/`chicago-version` (11+3+1 call sites across 3 files): replaced with `message: term.volume`/`term.version` (`form: short`, `text-case: capitalize-first` where the original was capitalized) + a bare number/variable component, nested in a `delimiter: ""` group.
- `chicago-of`/`chicago-of-number`/`chicago-episode`: moved from 3 style files' local `options.messages` into `en-US.yaml`'s existing `pattern.chicago-*` run (lowercase canonical form); added `text-case: capitalize-first` at the one call site needing the capitalized "Episode" form (chicago-author-date-18th).
- `taylor-and-francis-chicago-author-date-core.yaml`'s fully redundant 3-key `options.messages` block deleted (it duplicated chicago-author-date-18th's, both non-locale-override).
- Added `STYLE011` (report-only) to `scripts/style-structure-lint.js`: flags an `options.messages` (or `citation.options.messages`/`bibliography.options.messages`) entry whose normalized value already exists in the locale's role/term/message set — the class this bean's bug belonged to, invisible to `STYLE010`. 3 new tests in `style-structure-lint.test.js`.
- Updated `docs/policies/LOCALIZATION_INTEGRITY.md` (v1.0 → v1.1) to state the rule covers `options.messages` declarations, not just `prefix`/`suffix` call sites.

Verification: oracle run on all 4 affected legacy CSL sources (`chicago-author-date`, `chicago-notes`, `chicago-shortened-notes-bibliography`, `taylor-and-francis-chicago-author-date`) shows identical pass/fail counts and the same pre-existing `containerTitle`/`title` value_mismatch entries as the pre-edit baseline — 0 regressions. `de-DE` confirmed to translate `term.volume` (en-US "vol." → de-DE "Bd.", per `locales/de-DE.yaml:693-698`) and quoting; did not chase a live fixture render for `chicago-of`/`chicago-episode` specifically since no `de-DE`/`de-DE-chicago` translation exists yet for those (English prose that's now locale-owned but not yet translated — future work, not a regression). `cargo nextest run -p citum-schema-style -p citum-engine`: 1935/1935 pass. `node --test scripts/style-structure-lint.test.js`: 29/29 pass. `cargo fmt --check`: clean (no `.rs` changes in this PR).
