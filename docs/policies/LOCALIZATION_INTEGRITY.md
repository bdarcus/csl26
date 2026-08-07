# Localization Integrity Policy

**Status:** Active
**Version:** 1.0
**Date:** 2026-08-07
**Related:** [`CHICAGO_FAMILY_STRATEGY.md`](../specs/CHICAGO_FAMILY_STRATEGY.md), [`AUTHORING_LOCALES.md`](../guides/AUTHORING_LOCALES.md), [`LOCALE_MESSAGES.md`](../specs/LOCALE_MESSAGES.md), bean `csl26-dfq0`

## Rule

A style template must not hardcode natural-language prose as a literal
`prefix`/`suffix` string when an equivalent locale role verb, term, or
`message: pattern.*` already exists. Use the locale mechanism instead, so the
same template renders correctly under every installed locale.

## Rationale

Citum ships a real locale system — `verb`/`verb-short` role-label forms and
MF2 `messages:` (`docs/guides/AUTHORING_LOCALES.md`) — but nothing currently
measures whether style authors actually use it. The citeproc-js oracle that
drives fidelity and exact parity is English-only, so a style that hardcodes
`prefix: "Translated by "` scores identically to one that calls `message:
pattern.chicago-written-by` — even though only the latter renders correctly
under `-L fr-FR`. This was found empirically in the Chicago family: one sibling
style (`chicago-notes-18th`) already used locale messages for 17 sites while
another (`taylor-and-francis-chicago-author-date-core`) hardcoded 17 literal
strings with zero locale-message use — same family, same underlying facts,
opposite practice, and no metric caught the drift.

## Application

| Situation | Action |
|---|---|
| `prefix`/`suffix` is a role-label verb phrase (e.g. `"Translated by "`) and the role already has a `verb`/`verb-short` locale entry | Use `contributor: <role>, form: verb` (or `verb-short`) instead of the literal string |
| `prefix`/`suffix` matches an existing `pattern.*` MF2 message once normalized (trimmed, casefolded) | Use `message: pattern.<id>` with the appropriate `args:` |
| `prefix`/`suffix` is structural punctuation (`" ("`, `". "`, `": "`) or a non-linguistic literal (a URL scheme, `"https://doi.org/"`) | Not a violation — out of scope for this rule |
| `prefix`/`suffix` is English prose with **no** existing term/message equivalent | Not silently left as-is: add the missing locale entry (plain `verb:`/`term:` for non-parameterized text, MF2 `message:` when the text needs a variable or plural dispatch — see `AUTHORING_LOCALES.md`'s "when to add a `messages:` entry" table) and then convert the site. Do not defer the locale addition to a follow-up bean while leaving the hardcoded string in place. |
| Two locale entries could both serve one site (e.g. a `pattern.chicago-interview-by` message vs. a role `verb` that intentionally corrects upstream CSL wording) and would render *different* text | Let the citeproc-js oracle decide per `CHICAGO_FAMILY_STRATEGY.md`'s authority rule; the losing text becomes a registered divergence, not a silent locale edit |

Detection: `STYLE010` in `scripts/style-structure-lint.js` — the existing
deterministic style-shape linter, not the oracle-driven SQI/fidelity pipeline
in `report-core.js` (`docs/reference/SQI.md`: "SQI is not the structural lint
... enforced separately by `style-structure-lint.js`"). It normalizes each
authored `prefix`/`suffix` (trim punctuation/whitespace, casefold) and flags
it only when it matches the locale's role-verb/term/message-pattern value set
(`loadLocaleAffixValueSet`, built from `en-US.yaml`'s `roles`, `terms`, and
single-line `messages`). It is **not** in `FATAL_RULE_IDS` — report-only for
now, same as `STYLE006`, so it carries no CI-blocking weight yet. It cannot
detect prose with no term equivalent (including multi-line MF2 `.match`
messages, which have no single literal to compare against); that class is
caught by the "no existing equivalent" row above, applied by the author, not
the tool. Run it directly against embedded styles — `node
scripts/style-structure-lint.js --json crates/citum-schema-style/embedded/styles/*.yaml`
— since the default CI wrapper (`scripts/validate-production-styles.sh`) only
targets the in-repo `styles/` tier; wiring embedded styles into that default
is a separate, portfolio-wide decision not made here.

## Exceptions

A site may keep a literal string if converting it would lose exact parity
against the citeproc-js oracle and the capitalization/punctuation interaction
cannot be resolved through the existing `form: verb` + `text-case:
capitalize-first` mechanism. Record the reason inline next to the literal
string and file a bean for the underlying engine gap — do not silently accept
the parity loss and do not silently accept the literal string without a
recorded reason.

## Changelog

- 2026-08-07: Initial version, established from the Chicago-family
  localization-adoption audit.
