---
# csl26-7yxy
title: Chicago styles hardcode English prose/terms in options.messages
status: todo
type: task
priority: normal
tags:
    - chicago
    - style
    - localization
created_at: 2026-08-26T22:30:21Z
updated_at: 2026-08-26T22:30:24Z
parent: csl26-awlo
---

13 pattern.chicago-* message definitions with literal English values across 4 style files (chicago-author-date-18th.yaml:36, chicago-notes-18th.yaml:38, chicago-shortened-notes-bibliography-core.yaml:38, taylor-and-francis-chicago-author-date-core.yaml:11), 3 keys duplicated verbatim across 3 files. Invisible to STYLE010 (scripts/style-structure-lint.js), which only scans prefix:/suffix: lines, not options.messages blocks — csl26-dfq0's localization sweep never saw this class. Two fixes: (1) chicago-volume/-volume-lower/chicago-version duplicate existing locale terms (term.volume.short etc, locales/en-US.yaml:846) — delete the messages, use number: volume + label-form: short instead (resolve_number_label in values/number.rs:110). (2) chicago-of/chicago-of-number/chicago-episode are genuine prose — move to locales/en-US.yaml's existing pattern.chicago-* run (line 1048), not the en-US-chicago override (author-date/notes don't set locale-override). Add STYLE011 lint rule for this class (report-only, like STYLE010). Acceptance bar per dfq0 precedent: 0 oracle entries changed, plus -L de-DE showing translated output. Independent of csl26-awlo's range/collapse redesign — no Rust changes, can land in parallel. Related: csl26-dfq0, csl26-boha (migrate-side emission, distinct).
