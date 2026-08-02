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
updated_at: 2026-08-02T22:02:07Z
parent: csl26-ccdt
---

Follow-up to csl26-3az5: use the new LocaleOverride.dates month-name overrides and DateConfig.day-zero-pad capabilities to actually fix ieee's date rendering ("July 13, 2021" should be "Jul. 13, 2021"). Add locales/overrides/en-US-ieee.yaml (short: {6: Jun., 7: Jul., 9: Sep.}), register it in src/embedded/locales.rs + EMBEDDED_LOCALE_OVERRIDE_IDS, expand ieee's shared 'dates: short' preset into an explicit block with day-zero-pad: true and locale-override: en-US-ieee, then measure fidelity before/after via report-core.js. Out of scope for csl26-3az5 deliberately, since wiring a specific style moves fidelity numbers the schema/engine PR shouldn't.
