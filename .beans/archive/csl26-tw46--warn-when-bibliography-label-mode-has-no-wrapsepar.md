---
# csl26-tw46
title: Warn when bibliography label-mode has no wrap/separator
status: completed
type: task
priority: normal
tags:
    - engine
    - warnings
    - fidelity
created_at: 2026-08-13T11:51:49Z
updated_at: 2026-08-13T11:51:58Z
---

Follow-up from csl26-l5oh (springer/T&F-NLM numeric marker fix, PR #1177): nothing warned when a style declared bibliography.options.label-mode with no label-wrap and no label-separator, which is exactly the shape that let three shipped styles silently render markers flush against the entry body. Adds an engine warning scanner.

## Summary of Changes

Added `bibliography_label_missing_separator_warnings` to `crates/citum-engine/src/api/warnings.rs`, following the existing scanner pattern (`scan_bibliography_config_sort_for_citation_number`). Fires when the resolved bibliography config has `label_mode: Numeric | Alphabetic` with both `label_wrap` and `label_separator` absent — `author-date` mode is exempt since it never produces a bibliography marker (`marker.rs`). Advisory only, not an error: the empty default is correct for some shipped styles.

Wired into the two rendering entry points (`document.rs`, `session.rs`) and into `citum check` (`crates/citum-cli/src/commands/check.rs`) so style authors see it pre-flight.

5 unit tests added (bare label-mode triggers; label-wrap alone suppresses; label-separator alone suppresses; author-date mode suppresses; no label-mode at all is silent).

**Corpus validation** (`citum check` against every embedded + exemplar style file on pre-#1177 main): fires on exactly the three previously-broken styles (springer-basic-brackets, springer-vancouver-brackets, taylor-and-francis-national-library-of-medicine) plus one already-documented intentional case (royal-society-of-chemistry, flush second-field-align per its own in-file comment). Zero noise elsewhere across 35 styles. Confirms the warning would have caught the #1177 bug pre-merge.

`cargo nextest run`: 2506/2506 passed. `cargo clippy --all-targets --all-features -- -D warnings`: clean. `just check-core-quality`: gate passed, zero rendering/parity impact (this is a diagnostic-only addition, no render-path changes).
