# Text-Case Protection Policy

**Status:** Active
**Version:** 1.0
**Date:** 2026-08-11
**Related:** [`DESIGN_PRINCIPLES.md`](../architecture/DESIGN_PRINCIPLES.md), bean `csl26-4kt3`

## Rule

Case protection for titles is declared in the data — `.nocase` Djot spans, `<span class="nocase">`
CSL-JSON markup, biblatex brace-protected groups — never inferred from a word list, a capitalization
pattern, or any other heuristic. The one permitted inference is **internal capitalization**: a word
whose casing goes beyond a single leading capital (`NIPS`, `iPhone`, `McDonald`) is presumed
deliberately cased and is left untouched by sentence- and title-case transforms. Non-English content
falls back to `AsIs` before any of this applies.

## Rationale

`csl26-4kt3` was filed against the premise that citeproc-js applies a "stop-word/proper-noun
heuristic" under sentence case, and that Citum should replicate it so titles like `Cambridge` or
`La Ciotat` survive lowercasing. That premise does not hold: `CSL.Output.Formatters.sentence`, in
citeproc-js's vendored bundle (the citeproc npm package installed under scripts/node_modules/,
not tracked in git), capitalizes only the first word and calls `toLocaleLowerCase` on every other
word, unconditionally. The stop-word list (`skipWordsRex`) exists solely in
`CSL.Output.Formatters.title` for *title* case, not sentence case.

A proper-noun heuristic for sentence case cannot be built correctly:

- **Undecidable without a lexicon.** `Cambridge` carries exactly one capital letter — structurally
  identical to `Effect` in title-case source data. Preserving every non-initial capitalized word
  makes sentence case a no-op on the majority of real-world CSL-JSON, which arrives title-cased.
- **Wrong outside English.** German capitalizes every noun; the heuristic would preserve entire
  titles. Turkish/Azerbaijani need the dotted-ı mapping already provided by `icu_casemap`, not a
  word list. Uncased scripts (CJK, Arabic, Hebrew, Devanagari) make it a no-op. `resolve_text_case`
  already falls back to `AsIs` for non-English, so the heuristic would only ever fire on `en*`
  content anyway.
- **Fights the parity milestone.** `csl26-w0hf` targets 100% embedded oracle parity against
  citeproc-js. Every word a heuristic preserves against citeproc-js's flat lowercasing is a
  deliberate byte-level divergence.
- **The explicit mechanism already exists.** `.nocase` protection is honored end-to-end: Djot
  `[mRNA]{.nocase}`, CSL-JSON `<span class="nocase">` (`citum-schema-data/src/reference/citeproc_markup.rs`),
  and biblatex escaped-HTML spans (`citum-refs/src/formats/biblatex/mapping.rs`) all survive sentence
  case. biblatex's own brace protection is the same prior art. Per `DESIGN_PRINCIPLES.md`
  ("explicit over magic"), protection belongs in the data, declared by whoever authored it — not
  guessed at render time.

What *was* a genuine bug (and is what `csl26-4kt3` actually fixed) is that sentence case had two
divergent implementations depending on whether a title happened to contain Djot markup: the
plain-text path preserved internally-capitalized words via `has_internal_uppercase`, while the
Djot-markup path flat-lowercased every non-first text leaf regardless of casing. `NIPS` survived in
`"The Effect of NIPS on Cognition"` but became `nips` in `"_The Effect_ of NIPS on Cognition"` —
same style, same data, different answer because of an unrelated emphasis span. That inconsistency is
fixed by sharing one word-level rule (`sentence_case_words`,
`crates/citum-engine/src/values/text_case.rs`) between both paths.

## Application

| Surface | Behavior |
|---|---|
| `to_sentence_case_with_language_id` (plain-text titles) | Word-level via `sentence_case_words`; internal-caps words preserved verbatim |
| `make_case_transform` (Djot text leaves, `values/title.rs`) | Same `sentence_case_words` rule; `.nocase` spans bypass the transform's *output* but still advance its "first word" state, so words after a protected span are not re-capitalized |
| `apply_to_structured_parts_with_language`, `SentenceNlm` subtitles | Flat lowercase — deliberate NLM behavior (subtitle receives no leading capital), unaffected by this policy |
| `to_title_case_with_language_id` | `TITLE_CASE_STOP_WORDS` list + the same internal-caps guard |
| `apply_sentence_initial_transform` (`processor/rendering/grouped/sentence_initial.rs`) | `CapitalizeFirst` only, never sentence case — orthogonal to this policy |
| `resolve_text_case` | Non-English language tags fall back to `AsIs` before any transform below this row runs |

**Accepted citeproc-js divergence:** Citum's sentence case preserves internal-caps words
(`NIPS`, `iPhone`) where citeproc-js's `sentence()` formatter lowercases them (see
`CSL.Output.Formatters.sentence` above). This is deliberate (commit `34181280`, 2026-07-04,
extended to the markup path by this policy's introducing change) and should not be "corrected" by
a future parity sweep without a conscious decision to trade it away.

**Out of scope by design:** a single-capital proper noun with no internal capitalization
(`Cambridge`, `La Ciotat`) is indistinguishable from title-case source data and is not preserved
unless the source data marks it with `.nocase` (or the equivalent CSL-JSON/biblatex markup).

## Exceptions

None. If a style needs proper-noun protection it cannot get from `.nocase` data markup, that is a
data-quality gap in the reference (missing `.nocase` markup), not an engine gap — file it against
the reference/import path, not this policy.

## Changelog

- v1.0 (2026-08-11): Established; documents the rejected proper-noun heuristic and the
  plain-text/markup sentence-case unification from `csl26-4kt3`.
