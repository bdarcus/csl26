---
# csl26-3m0u
title: 'Chicago: citation-group collapse delimiter drops semicolon before disambiguated year'
status: scrapped
type: bug
priority: high
created_at: 2026-09-04T13:02:23Z
updated_at: 2026-09-04T13:45:41Z
parent: csl26-h7oc
---

Citum renders (Garcia 2019b, 2019a) / (Chen 2022, 2024) where oracle has semicolon-joined (Garcia 2019b; 2019a) / (Chen 2022; 2024) -- a same-author multi-date collapse group is using the plain item-delimiter comma instead of the disambiguation-collapse delimiter. Also bundles a missing comma in (Forthcoming n.d.) -> (Forthcoming, n.d.). 4+2 confirmed sole-cause row flips. See plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md

## Reasons for Scrapping (2026-09-04)

Not a bug. This is div-017 in docs/adjudication/DIVERGENCE_REGISTER.md -- a registered, evidence-backed intentional divergence: Citum joins same-author-collapse repeated years with a comma per CMOS 15.30's own Q&A ("your first approach is preferred for its brevity"), while citeproc-js's semicolon is traced to chicago-author-date.csl's <layout delimiter="; "> leaking into an undocumented citeproc-js default, not a considered CMOS-following choice. chicago-author-date-18th.yaml's own comment at the collapse: same-author config says explicitly: "Do not add a delimiter override here to chase oracle parity -- read div-017 first." Regression-guarded by test_chicago_author_date_same_author_collapse_without_locator_stays_comma_joined (crates/citum-engine/tests/domain_fixtures.rs) and a CSL-suite replay test. disambiguate-year-suffix and subsequent-author-consecutive (the two citation rows this bean was filed against) are exactly the fixtures that test guards.

The bundled (Forthcoming n.d.) -> (Forthcoming, n.d.) case (no-date-single fixture, a single-item citation, unrelated to same-author collapse) is a real, narrow, separate gap: Chicago's citation template joins contributor+date with a bare space, but CMOS convention wants a comma specifically when the date falls back to the no-date term ("n.d."). This is a YAML-only fix (a render-when-conditioned group split mirroring the pattern chicago-author-date-18th.yaml already uses for original-published groups), not an engine change, and touches exactly one test row -- out of scope for the current engine-fix stack. Left unfiled as its own bean given the audit's own finding #5 (bean sprawl); revisit alongside future date-fallback/no-date work if picked up.
