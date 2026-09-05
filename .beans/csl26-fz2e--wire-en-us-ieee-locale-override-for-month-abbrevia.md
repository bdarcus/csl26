---
# csl26-fz2e
title: Wire en-US-ieee locale override for month abbreviations + day-zero-pad
status: todo
type: task
priority: normal
tags:
    - fidelity
    - style
created_at: 2026-08-02T22:01:56Z
updated_at: 2026-09-05T21:39:30Z
parent: csl26-ccdt
---

Follow-up to csl26-3az5: use the new LocaleOverride.dates month-name overrides and DateConfig.day-zero-pad capabilities to actually fix ieee's date rendering ("July 13, 2021" should be "Jul. 13, 2021"). Add locales/overrides/en-US-ieee.yaml (short: {6: Jun., 7: Jul., 9: Sep.}), register it in src/embedded/locales.rs + EMBEDDED_LOCALE_OVERRIDE_IDS, expand ieee's shared 'dates: short' preset into an explicit block with day-zero-pad: true and locale-override: en-US-ieee, then measure fidelity before/after via report-core.js. Out of scope for csl26-3az5 deliberately, since wiring a specific style moves fidelity numbers the schema/engine PR shouldn't.

Found the same underlying gap for a different term while tuning
csl26-on47: springer-basic-brackets and springer-basic-author-date
need "et al" WITHOUT a trailing period (oracle: "Vaswani A, ... et al
(2017)"), but the shared en-US locale's et_al term
(embedded/locales/en-US.yaml:329, message key term.et-al) is "et al."
for every style. Same fix shape: a new
locales/overrides/en-US-springer.yaml with
messages: {"term.et-al": "et al"}, registered the same way as
en-US-ieee would be, referenced via locale-override on both springer
styles. ~2-4 rows, not chased in csl26-on47 for the same reason IEEE's
month abbreviation wasn't: this is new-locale-file + registration
work, not a style YAML edit.
