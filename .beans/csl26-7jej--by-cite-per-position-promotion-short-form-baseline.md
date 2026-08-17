---
# csl26-7jej
title: 'by-cite: per-position promotion (short-form baseline) not modeled, only depth'
status: todo
type: feature
priority: low
created_at: 2026-08-17T12:26:25Z
updated_at: 2026-08-17T12:26:25Z
---

Follow-up from csl26-5753. Citum's by-cite implementation promotes every currently-shown author position from the style's short/family-only citation form to a long/given-name-revealing form *uniformly* (the single `expand_given_names` bool), then varies only *depth* (initials vs full) per position via `ProcHints.expand_given_names_full_positions`.

Real citeproc-js keeps the promotion itself per-position too, via a `request_base` floor in `evalname` (scripts/node_modules/citeproc/citeproc_commonjs.js): a shared, non-disambiguating position (e.g. a common first author) can stay at bare family form (no initial at all) while only the position that actually collides promotes to a given name. Verified against the official CSL test suite's `disambiguate_ByCiteBaseNameCountOnFailureIfYearSuffixAvailable.txt`/`disambiguate_ByCiteRetainNamesOnFailureIfYearSuffixNotAvailable.txt` fixtures (short-form + initialize-with baseline): oracle output "Asthma, Bosworth Bronchitis, et al." -- position 0 stays bare family, position 1 gets the full given name. Citum's uniform-promotion model would render "A. Asthma, Bosworth Bronchitis" for equivalent input (position 0 gratuitously promoted to show its initial).

This is not a correctness regression -- Citum's key-builder and renderer agree with each other, and the divergence is byte-identical before and after csl26-5753 (a `report-core.js --all-features` corpus sweep found zero regressions). It's an oracle-fidelity gap in the same family as csl26-5753, deferred because closing it requires threading the citation template's `ContributorForm` (Short vs Long) into `Disambiguator`, which currently has no visibility into template shape at all -- a larger architectural change than a per-position depth field.

## Scope
- [ ] Give `Disambiguator` visibility into the citation/bibliography template's contributor form (Short vs Long) per rendering context.
- [ ] Extend the by-cite positional model so a position's *promotion* (not just its depth) can stay at baseline when it never contributes to resolving a collision.
- [ ] Add native regressions mirroring `disambiguate_ByCiteBaseNameCountOnFailureIfYearSuffixAvailable`/`disambiguate_ByCiteRetainNamesOnFailureIfYearSuffixNotAvailable` for a short-form + initialize-with style.
- [ ] Update `docs/specs/DISAMBIGUATION.md` §2.1.1 and the `ProcHints.expand_given_names_full_positions` doc comment once closed.
