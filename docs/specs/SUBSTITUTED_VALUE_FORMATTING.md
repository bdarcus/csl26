# Substituted-Value Formatting Specification

**Status:** Draft
**Version:** 1.5
**Date:** 2026-08-26
**Supersedes:** None
**Related:** bean `csl26-p7a8`; bean `csl26-0dca`; bean `csl26-0u0f`;
`docs/adjudication/DIVERGENCE_REGISTER.md` div-011;
`docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md` (bibliography-context
companion, covers quote and italic formatting, not quoting alone)

## Purpose

`csl26-0dca` fixed the engine so a title promoted into the author slot (the
`contributors.substitute` chain, e.g. APA's author → editor → title
fallback) picks up its `titles:` category `emph`/`strong`/`small-caps`
whenever it is not being quoted. No embedded style opts into the
`contributors.substitute.title-quote: by-category` mode that controls this
in citation context, so every embedded style still quotes a substituted
title unconditionally there — the pre-existing default.

This spec answers the question `csl26-p7a8` was filed to evaluate: **when a
value is promoted into a slot it doesn't normally occupy, should it keep its
own formatting, or take on the slot's?** It covers every substituted-value
kind, not only title, and recommends which embedded styles should flip
`title-quote: by-category`, whether the flip is sufficient on its own, and
what stays out of scope, and records the decisions that gate implementation.

## Scope

In scope: how the legacy CSL corpus formats a `<substitute>`-promoted title,
parent-serial title, and contributor role, both in citation and bibliography
context; a recommendation for which embedded styles should set
`title-quote: by-category`; what additional per-style configuration (if any)
a flip requires; implementation decisions and follow-up work.

Out of scope (this doc only): writing the style YAML changes themselves
(stage 2, a stacked follow-on PR); a `citum-migrate` heuristic to infer
`title-quote` from a CSL style's `<substitute>` block (flagged as a
follow-up bean, §7); a per-substitute-key contributor rendering override
(flagged as a follow-up bean, §7); **bibliography-context** substitute-title
formatting (quote, italic, or otherwise) — this spec's recommendation and
acceptance criteria are citation-scoped only (§5). Bibliography-context
formatting of a substituted title is covered separately in
[`SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md`](./SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md),
filed to evaluate bean `csl26-0u0f`.

## Design

### 1. The axis, and why it's asymmetric between titles and contributors

CSL itself has no explicit slot-vs-source switch. `<substitute>` is just an
ordered list of elements, and each element renders however it's written —
often literally the *same* macro used for that value's normal rendering
elsewhere in the style. `styles-legacy/international-affairs.csl` is a
direct example: its `<substitute>` block calls `<text macro="title"/>`, the
identical macro invoked at eight other call sites in the same style for
normal (non-substitute) title rendering. There is no separate "substitute
title" formatting rule to speak of — the promoted value keeps its own,
category-driven formatting because the style never wrote a rule that says
otherwise.

Citum approximates this per-slot rather than per-element, because its
`<substitute>` equivalent (`options.substitute.candidates`) is a list of
*keys*, not a list of styled elements. The approximation lands on opposite
sides of the corpus for the two value kinds it covers:

- **Contributor roles** (editor/translator/etc. promoted into the author
  slot): Citum's default is to inherit the slot's own name-list formatting.
  Across 5304 `<names>` elements found inside `<substitute>` blocks in the
  2844-style legacy corpus, **94.6% (5017) are the bare, self-closed
  `<names variable="editor"/>` form** — no explicit `<name>`/`<label>`/
  `<et-al>` override, i.e. the style is relying on inherited/slot formatting
  by omission. Only 5.4% (287, across 107 styles) write an explicit
  override. Citum's slot-inheritance default matches the corpus norm.
  **Recommendation: no change for contributors.**

- **Titles** promoted into the author slot: the opposite skew. Of 1139
  legacy styles whose citation-context substitute chain reaches a title, the
  formatting is unconditional quoting — Citum's current, only implemented
  default — in just **23 styles (2%)**. The rest split across unconditional
  italic (509, 45%), type-conditional italic-or-quote (293, 26%, see the
  classifier caveat in §2), and plain (314, 28%). **Citum's default is the
  corpus outlier for titles, not the norm** — `title-quote: by-category`
  exists precisely to correct this, and needs embedded styles to opt in.

`by-category` already *is* the "keep source formatting" half of the switch
for titles — the finding here is that the *default* is mis-set relative to
the corpus, not that a new mode is missing.

**Parent-serial titles: mechanism-correct by construction, now with
corroborating corpus and oracle evidence too.** In the engine, a
`parent-serial` substitute (`SubstituteField::ParentSerial`) produces
`TitleType::ParentSerial`, not `TitleType::Primary`. The citation quoting
gate in `resolve_author_substitute`
(`crates/citum-engine/src/values/contributor/substitute.rs:980`) only
applies to `TitleType::Primary`:

```rust
let quote_in_citation = matches!(kind, TitleType::Primary)
    && match substitute.title_quote {
        Some(SubstituteTitleQuoteMode::ByCategory) => resolve_category_quote(reference, options),
        Some(SubstituteTitleQuoteMode::Always) | None => true,
    };
```

A `parent-serial` substitute is therefore **never quoted in citation
context and always receives category emphasis**, regardless of
`title_quote`. This has been true since `csl26-0dca` shipped. That's a
sound argument that the *mechanism* can't quote a `parent-serial`
substitute — but on its own it doesn't establish that real CSL styles
never *want* to quote one, which is a distinct, corpus-level question the
original draft of this spec didn't check (`scripts/audit-substitute-formatting.py`'s
title classifier only recognizes `variable="title..."`, so it's structurally
blind to `variable="container-title"`, the CSL analogue of `parent-serial`).
Added a narrower, second classifier for exactly this (see §2): across
82 legacy styles whose citation-context substitute chain reaches a
recognizable bare `container-title` element, **0 quote it — 51 italicize,
31 render it plain**. Confirmed directly with two oracle runs against
`chicago-author-date.csl` (whose real citation substitute chain is editor
→ translator → parent-serial → title): a synthetic title-less
`article-journal` with only a `container-title` renders italic in both the
oracle bibliography (`<i>Journal of Anonymous Studies</i>`) and Citum's
bibliography output; a synthetic title-less `entry-dictionary` (the type
Chicago's real `author-title-substitute-container` macro actually covers
in *citation* context) renders italic in the oracle citation output too
(`(Merriam-Webster.Com Dictionary 2020)`, italicized) — Citum's own
citation output for that second case renders `(n.d.)` instead, an
unrelated pre-existing gap in Citum's citation-context substitute-tier
eligibility for `entry-dictionary`, not something `title_quote` touches;
noted here only because it surfaced during this check, not investigated
further as out of scope for this spec. No case found,
corpus-wide or oracle-checked, where a real style quotes a substituted
`parent-serial`/`container-title` value. `chicago-author-date-18th`'s
`parent-serial` tier already behaves correctly and needs no further
change — the claim is now evidence-backed, not just code-reading.

### 2. Corpus method and limits

`scripts/audit-substitute-formatting.py` (committed alongside this spec)
regex-scans `styles-legacy/*.csl`, expanding macro calls up to 4 levels deep
to find the first title-bearing element inside each `<substitute>` block,
plus a narrower, second pass for bare `container-title` elements (the
`parent-serial` case — see the note in §1). Run it with `python3
scripts/audit-substitute-formatting.py` (add `--json` for machine output,
`--style a,b,c` to scope to specific styles).

**Corpus revision:** the numbers in this spec were computed against the
`styles-legacy` submodule pinned at commit `ca545f945a676a4b6319ba386ef3adaccacf9df9`
(2844 `.csl` files; run `git -C styles-legacy log -1 --format='%H'` to
check your checkout matches before treating a different count as a script
bug — upstream Zotero styles churn, so the corpus is a moving target). An
earlier draft of this spec reported slightly different citation-context
title numbers (24/294/311 instead of the 23/293/314 above) despite an
identical corpus and unchanged classification logic — that was a real bug
in the script, not corpus drift: `macro_closure()` returned a bare Python
`set()`, and CPython randomizes `str` hash values per process by default,
so which of several macro-closure-reachable `<substitute>` blocks got
classified first (for styles like `apa.csl` whose closure reaches both a
long-form and a short-form title macro with different rules — see the
next bullet) varied between runs of the identical script against the
identical corpus. Fixed by making `macro_closure()` return a deterministic,
breadth-first-ordered list; confirmed stable across repeated runs and
across explicit `PYTHONHASHSEED` values after the fix.

This is corroborating evidence, not ground truth. Known limits:

- **Only the first title-bearing element in the chain is classified.**
  Later fallback tiers (e.g. a style that quotes for `title` but not for
  `parent-serial`) are not distinguished.
- **"italic-or-quote-by-type" is inferred from co-presence**, not verified
  semantics: `font-style="italic"`, `quotes="true"`, and a `<choose>`
  anywhere in the macro closure. This produces a real false-positive mode,
  caught by manual verification (below): a style's citation-context
  substitute chain frequently reaches a **different, disambiguation-only
  short-title macro** (typically named `title-short` /
  `title-primary-short`) via macro expansion, and that macro's own
  `<choose>`/`quotes="true"` gets attributed to the primary substitute path
  even when the two macros encode different rules. `apa.csl` and
  `chicago-author-date.csl` both hit this: their real primary
  (bibliography-facing) substitute macro (`author-title-substitute`) calls
  a plain `title` macro with **no unconditional quoting at all** (italic
  only, or italic based on absence of `container-title`), while the
  *separate* citation-context macro (`author-short` → ... →
  `title-short`/`title-primary-short`) has its own, independently authored
  type-conditional italic/quote rule that happens to overlap in shape but
  not necessarily in type coverage. Treat every "italic-or-quote-by-type"
  classification as **needs manual verification against the real `.csl`
  source before acting on it** — do not batch-flip from the corpus numbers
  alone. This contamination isn't confined to that one bucket: the same
  macro-closure expansion feeds the "italic" and "plain" buckets too, so
  **all citation-context percentages in §1 reflect first-element authoring
  in the expanded closure, not verified rendered behavior.** The direction
  is sound — an unconditional-quote rule as the *sole* formatting rule is
  clearly rare in the corpus — but treat the exact percentages (including
  the headline "2%") as directional, not precise, until spot-checked the
  way §3/§4 spot-check the specific candidate styles.
- `<names>`-wrapper formatting (name form, delimiter, et-al truncation) on
  contributor substitute overrides is not further classified, only whether
  an override exists at all.
- The `container-title`/`parent-serial` classifier (§1) is deliberately
  narrower than the title one: it only recognizes a *bare*
  `<text variable="container-title" .../>` directly inside a `<substitute>`
  block, plus macro calls whose name contains "container" (left
  unclassified, reported separately as `macro-container-call`, 87 styles)
  — it does not expand those macros. Its 82-style, 0-quote finding is a
  lower bound on the corpus, not a percentage comparable to the title
  numbers above.

### 3. Per-style findings (embedded-style parents)

**An earlier draft of this section covered 4 of the 32 embedded styles**
(`crates/citum-schema-style/embedded/styles/*.yaml`) and stopped there.
The other 28 include `extends:`-chain descendants of the candidates
already covered, plus fully independent styles never checked. Both matter:
a descendant can inherit a parent's `substitute:` config while *clearing*
the `titles.type_mapping` fix a flip would need, and an independent style
can have its own citation-reachable substitute chain nobody looked at.

**The full embedded-style `extends:` graph** (`<none>` = independent, no
parent):

| Style | extends | Style | extends |
|---|---|---|---|
| american-medical-association | none | ieee | none |
| apa-7th | none | modern-language-association | none |
| chicago-18-base | none | springer-basic-author-date-core | none |
| chicago-author-date-18th | chicago-18-base | springer-basic-author-date | springer-basic-author-date-core |
| chicago-notes-18th | chicago-18-base | springer-basic-brackets-core | none |
| chicago-notes-18th-script | chicago-notes-18th | springer-basic-brackets | springer-basic-brackets-core |
| chicago-shortened-notes-bibliography-core | chicago-notes-18th | springer-vancouver-brackets-core | none |
| chicago-shortened-notes-bibliography | chicago-shortened-notes-bibliography-core | springer-vancouver-brackets | springer-vancouver-brackets-core |
| elsevier-harvard-core | none | taylor-and-francis-chicago-author-date-core | chicago-author-date-18th |
| elsevier-harvard | elsevier-harvard-core | taylor-and-francis-chicago-author-date | taylor-and-francis-chicago-author-date-core |
| elsevier-vancouver-core | none | taylor-and-francis-council-of-science-editors-author-date-core | none |
| elsevier-vancouver | elsevier-vancouver-core | taylor-and-francis-council-of-science-editors-author-date | taylor-and-francis-council-of-science-editors-author-date-core |
| elsevier-with-titles-core | none | taylor-and-francis-national-library-of-medicine-core | none |
| elsevier-with-titles | elsevier-with-titles-core | taylor-and-francis-national-library-of-medicine | taylor-and-francis-national-library-of-medicine-core |
| gb-t-7714-2025-base | none | | |
| gb-t-7714-2025-author-date / -note / -numeric | gb-t-7714-2025-base | | |

Re-ran `scripts/audit-substitute-formatting.py --style` against every
style's mapped legacy `.csl` source (via each YAML's `source.csl-id`) to
check citation-context reachability for every style not already in the
per-style table below. Confirmed **not affected** (no title reachable in
citation context, or a `-core`/leaf pair with no independent public
surface): `ieee`, `american-medical-association`, `elsevier-vancouver`,
`elsevier-with-titles`, `springer-basic-author-date`,
`springer-basic-brackets`, `springer-vancouver-brackets`,
`taylor-and-francis-national-library-of-medicine`, the GB/T 7714 family.
Three genuinely new findings, folded into the table below:

- **`taylor-and-francis-chicago-author-date-core` / `.yaml`** —
  descendant of `chicago-author-date-18th`. Its own YAML explicitly sets
  `titles.type-mapping: ~` (clears the inherited mapping — see the file's
  own comment citing `csl26-svfg` / `STYLE_INHERITANCE.md` rules 1 and 3)
  while declaring no `substitute:` of its own, so it inherits
  `chicago-author-date-18th`'s `substitute:` wholesale. **Any
  `title_quote`/`titles.default` fix added to `chicago-author-date-18th`
  to close the gap below does not propagate here** — the child would
  inherit the flipped quoting behavior but not the type-coverage fix,
  reintroducing the exact silent-plain-text regression in a style with its
  own distinct CSL parent (`taylor-and-francis-chicago-author-date.csl`,
  a dependent style, not necessarily identical to `chicago-author-date.csl`
  on this axis) that this spec has not oracle-checked at all.
- **`taylor-and-francis-council-of-science-editors-author-date(-core)`** —
  fully independent (no `extends` from any candidate above), uses
  `substitute: editor-translator-title-long` (a preset, not the explicit
  form). Its CSL parent's citation-context substitute title classifies as
  **plain** — the opposite problem from the other candidates: Citum's
  current always-quote default is *already wrong* for this style today,
  independent of any `by-category` flip.
- **`chicago-shortened-notes-bibliography(-core)`** — a *note* style
  (extends `chicago-notes-18th` → `chicago-18-base`), not an author-date
  style. Its CSL parent's citation-context substitute title classifies as
  `italic-or-quote-by-type`. Note styles render citations differently from
  parenthetical author-date citations (first-note vs. subsequent-note
  forms, `crates/citum-engine`'s note-position machinery), so this needs
  its own investigation before assuming it's comparable to the
  author-date candidates below — flagged, not evaluated to the same depth.

None of the three new candidates has been oracle-verified yet (§4)
— identified and traced to the same "not yet verified" standard already
used below for `elsevier-harvard`, not overclaimed as flip-ready.

Verified against the real `.csl` source (not just the classifier), plus
live engine tests on a scratch copy of the embedded style and — for the
two most-promising candidates — a real citeproc-js oracle run (§4). The
oracle run caught a manual-trace error the source-reading pass alone
missed (see MLA below); treat every row's "gap" column as provisional
until §4 has verified it directly.

| Legacy CSL parent | Real citation-context substitute rule | Embedded style | Gap under `by-category` |
|---|---|---|---|
| `apa.csl` | `author-short` → `title-and-descriptions-short` → `title-short`: type-conditional (bill/legislation/regulation/report → plain; legal_case/post → italic; hearing/webpage → italic; `container-title` present → quote; else → italic, i.e. the plain-book/manuscript default) | `apa-7th` | **Confirmed correct direction, one confirmed gap.** Oracle-verified (§4): `dead-sea-scrolls` (an author-less `manuscript`, no `container-title`) renders italic in real citeproc-js APA (`<i>The Community Rule (1QS)</i>`) but quoted in current Citum. `apa-7th`'s `monograph` category (`emph: true`) does **not** cover `manuscript` — it falls to the engine's hardcoded default table (`title_class.rs`), which doesn't recognize `manuscript` either, landing on `TitleCategory::Default` with no configured rendering. A naive flip renders this case **plain** (neither quoted nor emphasized) — see the `titles.default` gap below, which is the actual root cause, not a `component`-specific one. Separately, an author-less item **with** a `container-title` should quote per the real CSL rule, and `apa-7th`'s `titles.component` has no `quote` — same root cause. |
| `chicago-author-date.csl` | `citation-author-date-item` → `author-inline` → `author-title-substitute-short` → `title-short` → `title-primary-short`: same shape as MLA's split (type-conditional, own type partition) | `chicago-author-date-18th` | Same class of gap as `apa-7th`, not yet oracle-verified: `monograph`/`container-monograph`/`periodical`/`serial` already `emph: true`, `component` has no `quote`. Unlike apa-7th/MLA, this style already declares an explicit `titles.type_mapping` (`broadcast`/`collection`/`manuscript`/`motion-picture`/`song`/`webpage` → `component`), so its `manuscript` case is a **component-lacks-quote** gap specifically, not an unconfigured-`Default` one — narrower than apa-7th's, but any type still outside its `type_mapping` and outside the engine's hardcoded table falls to the same `Default` risk. |
| `modern-language-association.csl` | `author-short` → `title-short` → `title-primary-short`: type-conditional (bill/collection/legislation/regulation/treaty → plain; legal_case/book/classic/graphic/hearing/map/periodical → italic; post → quote; `container-title` present → quote; article/dataset/document/interview/manuscript/paper-conference/personal_communication/speech → quote **even without** a `container-title`; else → italic) | `modern-language-association` | **Not gap-free — corrected from an earlier draft of this table.** The `book`-type case is clean: live test confirms `(_Some Book Title_)`, matching the oracle direction. But **oracle-verified** (§4): the same `dead-sea-scrolls` manuscript that resolves correctly for APA renders quoted in real MLA citeproc-js (`(“The Community Rule (1QS)”)`) — MLA's own rule explicitly quotes `manuscript` regardless of `container-title`. **Live-tested and confirmed broken**: flipping `by-category` on a scratch copy of `modern-language-association.yaml` renders this case as plain `(The Community Rule (1QS))` — a regression from the correct, currently-shipping quoted default. Root cause: MLA's `titles: chicago` preset sets `component`/`monograph`/`periodical` only; `manuscript` isn't in the engine's hardcoded default table either (`title_class.rs` recognizes only a fixed component/monograph list — see below), so it falls through to `titles_config.default`, which MLA never sets. |
| `elsevier-harvard.csl` | **Corrected — v1.1/v1.2 wrongly recorded this as unconditional italic.** Citation context actually uses `author-short`, a type-conditional macro distinct from the bibliography-facing `author` macro the classifier picked up (the same short-vs-long-macro contamination §2 documents for apa.csl/chicago-author-date.csl) — `bill`/`book`/`graphic`/`legal_case`/`legislation`/`motion_picture`/`report`/`song` → italic; else → quote, no container-title conditionality | `elsevier-harvard` | Oracle-confirmed (§4) with `dead-sea-scrolls` (quote, matches current default) and three synthetic fixtures (`book`/`report` → italic, current default wrongly quotes; `article-journal` → quote, current default already correct). Fully type-representable, no `quote-when` needed — genuinely the simplest candidate, just not for the reason originally claimed. `bill` is CSL-polymorphic (converts to Citum's `document`, `hearing`, `bill-proceeding`, or `bill-record` depending on data shape per `CSL_TYPE_CONVERSION_CONTRACT.md`); verifying all four against the oracle is stage-2 scope, not resolved here. |
| `taylor-and-francis-chicago-author-date.csl` (dependent CSL style) | Not yet traced — needs its own `.csl` read, not assumed identical to `chicago-author-date.csl` | `taylor-and-francis-chicago-author-date-core` / `.yaml` | **Inheritance hazard, not a formatting gap per se.** Inherits `chicago-author-date-18th`'s `substitute:` wholesale but explicitly clears `titles.type-mapping`. A parent-only fix does not propagate here — needs its own `titles.default`/`type_mapping` addition even if the parent's is fixed, and its own oracle check against its own (dependent) CSL source, not the independent parent's. |
| `taylor-and-francis-council-of-science-editors-author-date.csl` | Not yet traced in detail; corpus classifier reports the reachable substitute title as plain | `taylor-and-francis-council-of-science-editors-author-date(-core)` | New candidate. Opposite direction from the others: Citum's current always-quote default is likely already wrong for this style (real CSL renders plain), independent of any `by-category` flip — worth checking whether `by-category` with an unstyled category (plain, matching) is actually a clean win here, once oracle-verified. |
| `chicago-shortened-notes-bibliography.csl` (note style) | Not yet traced; corpus classifier reports `italic-or-quote-by-type` | `chicago-shortened-notes-bibliography(-core)` | New candidate, not yet evaluated — note-style citation rendering (first/subsequent-note forms) differs structurally from author-date parenthetical citations; needs its own investigation before assuming the same `titles.default` remediation applies. |
| ieee, elsevier-vancouver, elsevier-with-titles, springer-\* (author-date, brackets), american-medical-association, `taylor-and-francis-national-library-of-medicine`, GB/T 7714 family, ACS, nature | No title reachable from the citation-context substitute chain (`cit=None`) | — | Unaffected by any flip; `citum-migrate` correctly has nothing to infer here. |

**The `titles.default` coverage gap is the real, general finding — bigger
than any one style.** `get_title_category_title_rendering` in
`crates/citum-engine/src/render/component.rs:530` resolves a `Primary`
title's category via the style's explicit `titles.type_mapping`, then
falls back to the engine's hardcoded `title_category()` table
(`crates/citum-schema-style/src/options/title_class.rs:225`), which
recognizes only a fixed, narrow set — `article-journal`/`article-magazine`/
`article-newspaper`/`chapter`/`entry`/`entry-dictionary`/
`entry-encyclopedia`/`paper-conference`/`post`/`post-weblog` →
`Component`; `book`/`thesis`/`report` → `Monograph`; **everything else →
`Default`**. If the style's `titles.default` is unset (true of both
`apa-7th` and `modern-language-association` today), any reference type
outside that narrow table — `manuscript`, `dataset`, `document`,
`interview`, `speech`, `webpage`, `legal_case`, `personal_communication`,
and any custom/extension type — resolves to **no rendering at all** under
`get_title_category_title_rendering`, so a `by-category` flip silently
drops to plain text: neither the historical unconditional quote nor the
category's intended emphasis. **This is a strictly worse outcome than not
flipping at all for those types**, and it is invisible from the
corpus-survey and `.csl`-source-reading passes alone — only the live
engine test against a real, unusual-type fixture item caught it. Every
stage-2 flip must declare explicit fallback intent in citation-scoped
`titles.default` and map any exceptions to suitable categories. A missing
category is not an acceptable substitute for an authored plain fallback.

**Structural limitation: instance-conditional rules can't be represented
by type mapping at all — this is a separate, deeper problem than the
`titles.default` gap above, and it isn't fixable by adding more type
coverage.** Re-read `apa.csl`'s real `title-short` rule carefully (the
table row above): the residual "default" branch is not "these specific
types are quoted" — it's **"quote if `container-title` is present on
*this instance*, italicize otherwise,"** independent of type. The same
type (e.g. `interview`, or any custom/extension type not covered by APA's
earlier explicit branches) can legitimately go either way depending on
whether a *specific reference* happens to carry a `container-title`.

Citum's category-resolution mechanism cannot see that. Both
`get_title_category_title_rendering`
(`crates/citum-engine/src/render/component.rs:463`, signature
`(title_type: &TitleType, ref_type: Option<&str>, language: Option<&str>,
config: &Config)`) and its citation-substitute caller
`resolve_category_quote`
(`crates/citum-engine/src/values/contributor/substitute.rs:709`) resolve
quoting purely from reference **type** plus style **config** — never from
instance data. Notably, `resolve_category_quote` *does* receive the full
`&Reference` as a parameter, but only ever extracts `ref_type()` and
`language()` from it; `container_title()` is never consulted, even though
it's one call away. **No `titles.type_mapping` or `titles.default`
addition can reproduce APA's real rule for its residual bucket** — any
type-only mapping is wrong for either the with-container or
without-container instances of that type, whichever way it's set.

This blocks `apa-7th` specifically (its residual bucket is the general
"has container-title" test, not an enumerable type list) more than it
blocks `modern-language-association` or `chicago-author-date-18th`
(both enumerate most container-associated types by name rather than
testing container-title presence directly — see their rows above — so
their residual buckets are smaller, though not necessarily empty; not
independently re-verified for this revision).

**Decision: implement the instance-conditional engine capability before
flipping `apa-7th`; do not accept a residual divergence.** The desired
behavior is known, the full `&Reference` is already available at both title
rendering call sites, and deliberately shipping a wrong result would conflict
with this spec's source-formatting recommendation. APA does not block simpler
style flips whose rules are representable today.

Add a typed condition to `TitleRendering`, serialized as, for example,
`quote-when: container-title-present`:

```rust
enum TitleQuoteCondition {
    ContainerTitlePresent,
}
```

Resolution precedence is normative:

1. An explicit `quote: true` or `quote: false` wins.
2. Otherwise, evaluate `quote-when` against the reference instance.
3. Otherwise, do not quote.

The condition must be applied consistently by normal title rendering
(`effective_title_quote_depth`) and category-based substitute rendering
(`resolve_category_quote`). The existing substitute path already skips
category emphasis when `quoted` is true, so it does not need a separate
container-presence branch. APA's condition and the paired type/category
mapping belong in `citation.options.titles`, alongside a citation-scoped
`substitute.title-quote: by-category`; they must not widen global title
behavior without separate oracle evidence.

### 4. Oracle ground truth (required before any style flip ships)

The regex survey and the manual `.csl` trace above are necessary but not
sufficient — they establish what the *style authors wrote*, not what
citeproc-js actually *renders*, and div-011's original discovery (`title`
was quoted where CSL styles clearly didn't intend that) came from exactly
this gap. **This section documents oracle runs already performed for this
spec** (not merely planned), using `node scripts/oracle.js`; the same
method should be repeated for chicago-author-date-18th and
elsevier-harvard before stage 2 flips them.

**`dead-sea-scrolls`** (`tests/fixtures/references-humanities-note.json` /
`citations-humanities-note.json`; an author-less, editor-less `manuscript`
with no `container-title`) against `apa.csl` and
`modern-language-association.csl`:

```
$ node scripts/oracle.js styles-legacy/apa.csl \
    --refs-fixture tests/fixtures/references-humanities-note.json \
    --citations-fixture tests/fixtures/citations-humanities-note.json --json
citation  oracle: (The Community Rule (1QS), 100 B.C.E.)     [italic, unquoted]
citation  citum:  ("The Community Rule (1QS)," 101 BC)       [quoted, unitalicized — current default]

$ node scripts/oracle.js styles-legacy/modern-language-association.csl \
    --refs-fixture tests/fixtures/references-humanities-note.json \
    --citations-fixture tests/fixtures/citations-humanities-note.json --json
citation  oracle: ("The Community Rule (1QS)")   [quoted — matches current Citum default]
citation  citum:  ("The Community Rule (1QS)")   [quoted — already correct]
```

APA's oracle output confirms the flip direction is right (italic, not
quoted) for this case, but also that the historical default is already
*correct* for MLA on this same reference. The bibliography-order and
date-formatting mismatches visible in the full oracle output for this pair
are pre-existing and unrelated to this spec.

**`parent-serial`** — see §1 for the full writeup: two synthetic
title-less fixtures (an `article-journal` and an `entry-dictionary`, both
with only a `container-title`) run against `chicago-author-date.csl`
confirm citeproc-js never quotes a substituted `container-title`, matching
the engine's mechanism-level guarantee.

Other fixtures worth the same treatment in stage 2, not yet run for this
spec: `references-author-date.json` (`ipcc2023` — has an editor, so it
exercises the editor-substitute tier, not `title`; useful as a negative
control), `references-secondary-roles.json` (`sr-translator-only`),
`references-expanded.json` / `references-heldout.json` (the
`TLIB-*-STANDARD`/`LEGISLATION`/`DICTIONARY`/`ENCYCLOPEDIA` entries and the
`legal-case`/`legal_case` set), and oracle checks for the three newly
identified candidates in §3 (`taylor-and-francis-chicago-author-date`,
`taylor-and-francis-council-of-science-editors-author-date`,
`chicago-shortened-notes-bibliography`) against their own CSL sources.

Sequential runs only, `systemd-run --user --scope -p MemoryMax=6G` at
concurrency 1–2, for any corpus-wide oracle sweep — the development
machine is memory-constrained and default-concurrency sweeps have crashed
it before.

### 5. Recommendation

- Keep `title_quote` unset (`Always`) as the schema default — this is the
  div-011 backward-compatibility contract and must not change.
- Put `title-quote: by-category` and any supporting `titles.default`,
  `titles.type-mapping`, or `quote-when` configuration in
  `citation.options` by default. A global title change requires independent
  evidence that normal citation and bibliography titles should also change.
- **No style flip ships without explicit fallback intent for every title
  reachable through its substitute chain.** Usually this means a
  `titles.default`; an intentionally plain fallback must be authored
  explicitly rather than inferred from a missing category. §3's coverage
  gap is the dominant risk for every candidate, not a per-style footnote.
- Contributors: no change. The 94.6% slot-inheritance default already
  matches the corpus norm.
- `parent-serial`: no change needed; already correct since `csl26-0dca`,
  now corroborated by corpus and oracle evidence too (§1).

APA's engine prerequisite does not block independently representable styles.
The stage-2 investigation and implementation order is:

1. Fully type-representable rules after oracle confirmation:
   `elsevier-harvard` (type-conditional italic-or-quote, no
   container-title dependency — see the corrected §3 row) and
   `taylor-and-francis-council-of-science-editors-author-date(-core)`
   (apparently unconditional plain).
2. `modern-language-association`, once its type partition and explicit
   fallback are complete.
3. `chicago-author-date-18th` and every affected descendant, especially
   `taylor-and-francis-chicago-author-date-core`, as one inheritance change
   surface.
4. `chicago-shortened-notes-bibliography(-core)`, after first-note,
   subsequent-note, and shortened-note behavior is oracle-verified.
5. `apa-7th`, after the instance-conditional `quote-when` capability ships.

The status table records the remaining gates:

| Candidate | Direction confirmed? | Blocked on |
|---|---|---|
| `apa-7th` | Yes (`manuscript` case, oracle-verified) | Instance-conditional `quote-when` engine support, citation-scoped mappings/fallback, and the paired same-type container-present/container-absent oracle cases |
| `chicago-author-date-18th` | Not yet oracle-verified | Explicit fallback coverage + its own oracle run; **and** its `taylor-and-francis-chicago-author-date-core` descendant needs independent resolved-config verification (§3) |
| `modern-language-association` | Partially — clean for `book`, confirmed broken for `manuscript` | `titles.default` coverage; do **not** ship on the strength of the `book`-only live test |
| `elsevier-harvard` | Yes — type-conditional rule, oracle-verified (§3, §4) | Nothing structural; `bill`'s CSL-polymorphic conversion (`document`/`hearing`/`bill-proceeding`/`bill-record`) is deliberately deferred to the safe `titles.default.quote: true` fallback rather than blocking the flip |
| `taylor-and-francis-council-of-science-editors-author-date(-core)` | Not yet traced/verified | Full trace of its real `.csl` substitute macro + oracle run (§3) — newly identified in this revision |
| `chicago-shortened-notes-bibliography(-core)` | Not yet traced/verified | Note-style citation rendering needs its own investigation before treating it as comparable to the author-date candidates (§3) — newly identified in this revision |

### 6. Rejected alternatives

- **Flip the engine-wide default to `by-category`.** Rejected: 28% of the
  1139 styles whose citation-context substitute chain reaches a title (not
  the full 2844-style corpus) render it plain, and every
  already-migrated style (with no `title-quote` set) would change output
  simultaneously with no per-style verification. Violates the same
  backward-compatibility contract div-011 established for `Always`.
- **A new `by-category-or-quote` fallback mode** (defer to category
  formatting, but quote if the category has no configured rendering).
  **Reconsidered but still not recommended for stage 1**, given the
  `titles.default` coverage gap in §3: that gap is exactly the scenario
  this mode would paper over (unconfigured category → fall back to quote
  rather than silently going plain). It would remove the sharp edge stage
  2 must otherwise engineer around per style. Still rejected as the stage-1
  recommendation because it's a new engine-wide fallback with its own
  behavior to specify and test, and the alternative — requiring
  `titles.default` completeness per flipped style — is more precise and
  keeps the blast radius scoped to styles that opt in. **Revisit in stage 2
  if requiring full type coverage per style proves too brittle in
  practice** (e.g. a style with many extension types where enumerating
  `type_mapping` is unwieldy).
- **Accept an APA residual divergence instead of adding instance-conditional
  title rendering.** Rejected: APA's behavior is known, the engine already
  has the necessary reference instance, and the divergence would be caused
  by an avoidable expressiveness gap rather than a deliberate Citum policy.
- **Per-substitute-key contributor rendering overrides**, to close the 5.4%
  names-override gap found in §1. Rejected as out of scope: real scope is
  wider than this bean (287 explicit overrides across 107 styles, each
  potentially different), and the corpus majority already matches Citum's
  default. Tracked as a follow-up bean, not fixed here.

### 7. Resolved decisions and follow-up work

1. **APA waits for engine support.** Add a typed `TitleQuoteCondition`
   with `container-title-present` semantics before flipping `apa-7th`.
   Do not register an avoidable residual divergence. This engine feature is
   a separate stage-2 change with schema generation, focused tests, and a
   `report-core.js` sweep.
2. **Supporting title configuration is citation-scoped by default.** Author
   `citation.options.substitute.title-quote: by-category` together with
   `citation.options.titles`. Global changes require separate evidence for
   ordinary citation and bibliography title rendering.
3. **Migrator inference is follow-up scope and must be proof-based.** File a
   separate bean. `citum-migrate` may emit `by-category` only when it can
   prove that the actual citation-reachable substitute element is
   representable, emit the required fallback/type configuration, and
   preserve any instance-data predicate. Otherwise it keeps `always` and
   emits a manual-review diagnostic. Macro-closure co-presence alone is not
   sufficient evidence.
4. **Contributor overrides need an inventory, not a schema commitment.**
   File a low-priority research bean to classify the 287 elements by name
   form/order, et-al, delimiter, label, nested-substitute behavior, and
   embedded/priority-style relevance before proposing per-substitute
   rendering configuration.
5. **Missing category coverage produces a warning, not a runtime error.**
   Plain rendering can be intentional. Validate the effective,
   extends-resolved citation configuration and warn when
   `title-quote: by-category` lacks explicit fallback intent. First-party
   embedded-style CI should treat this warning as a failure. An intentionally
   plain fallback should spell out false rendering flags where inheritance
   could otherwise retain a parent's values; an empty mapping is not a
   reliable clear under deep-merge semantics.
6. **Inheritance uses the same resolved-config diagnostic.** Do not add a
   special-case lint for raw `titles.type-mapping: ~`. The general hazard is
   an effective citation config that inherits `by-category` without a safe
   fallback after all parent/child overlays resolve. Provenance-aware wording
   may identify the responsible ancestor, but correctness is judged on the
   resolved config.
7. **The newly surfaced candidates remain hard stage-2 gates.** Trace their
   real `.csl` paths and run the oracle before changing them. Investigate
   T&F CSE first (apparently flat plain), T&F Chicago together with its
   Chicago parent, and Chicago shortened notes last because it needs
   first/subsequent/shortened-note coverage.
8. **Elsevier Harvard is an early candidate, not an architectural blocker.**
   Its unconditional italic rule appears directly representable with a
   citation-scoped emphasized default. Verify author-less title-only,
   container-bearing, and unusual/default reference types against the oracle
   before flipping it.

## Implementation Notes

Stage 2 (a stacked follow-on PR, only after this spec is reviewed):

- One logical commit per independent style change surface, each with its own
  `node scripts/report-core.js --all-features` before/after diff. A parent
  flip and required descendant compensation belong in the same commit so no
  intermediate revision silently regresses an inherited style.
- Implement `TitleQuoteCondition::ContainerTitlePresent` before the APA
  style flip. Apply the condition in both normal and substituted title quote
  resolution with explicit `quote` taking precedence.
- Add a resolved-config validation warning for `by-category` without explicit
  fallback intent; require the embedded-style validation gate to remain clean.
- Any accompanying schema change → `just schema-gen` in the same commit.
- Tests: parameterized cross-module behavior uses `#[rstest]` BDD
  `given/when/then`, ≥2 cases, and `assert_eq!` on captured output; pure unit
  behavior uses plain `#[test]`. Extend
  `crates/citum-engine/src/values/tests.rs` (the div-011 block) and
  `crates/citum-engine/tests/citations.rs`.
- The APA matrix must use the same reference type with and without
  `container-title`, both lacking author/editor/translator, plus an explicitly
  plain type (`report`), an explicitly italic type (`legal_case` or `post`),
  and an author-present negative control. Expected values come from the
  citeproc-js oracle, not the implementation under test.
- Elsevier Harvard must cover author-less title-only, container-bearing, and
  unusual/default types. Chicago shortened notes must cover first,
  subsequent, and shortened-note positions.
- `just pre-commit` gate, verbatim.

## Acceptance Criteria

- [x] `scripts/audit-substitute-formatting.py` reproduces the corpus numbers
      in §1 and §2 **when run against `styles-legacy` @
      `ca545f945a676a4b6319ba386ef3adaccacf9df9`** (pinned in §2 — verify
      with `git -C styles-legacy log -1 --format='%H'` before treating a
      different count as a script bug). Confirmed deterministic across
      repeated runs and explicit `PYTHONHASHSEED` values after fixing the
      `macro_closure()` nondeterminism bug (§2).
- [x] The classifier caveat in §2 is demonstrated (not just asserted) via
      the apa.csl / modern-language-association.csl worked examples in §3
      (the `title` vs. `title-short` macro conflation, and the
      manual-trace correction on MLA's `manuscript` case caught only by
      running the oracle).
- [x] §4's oracle comparison is run and recorded (citeproc-js oracle output
      vs. current Citum output) for apa-7th and modern-language-association,
      using the `dead-sea-scrolls` fixture item.
- [x] The full embedded-style graph (32 styles, all `extends:` chains) is
      enumerated in §3, not just the 4 originally-picked candidates.
- [x] The parent-serial "already resolved" claim in §1 is backed by corpus
      evidence (a dedicated `container-title` classifier) and an oracle
      run, not engine-code reading alone.
- [x] This spec is registered in `docs/specs/README.md` under
      "Text & Rendering."
- [x] `elsevier-harvard`'s oracle comparison is run and recorded (§3, §4) —
      also caught and corrected a factual error (v1.1/v1.2 wrongly recorded
      its citation-context rule as unconditional italic; it's actually
      type-conditional, the same short-macro contamination pattern as
      apa.csl/chicago-author-date.csl).
- [ ] The same oracle comparison is extended to chicago-author-date-18th
      and the three newly identified candidates
      (`taylor-and-francis-chicago-author-date-core`,
      `taylor-and-francis-council-of-science-editors-author-date(-core)`,
      `chicago-shortened-notes-bibliography(-core)`) before stage 2 flips
      any of them.
- [x] The `apa-7th` container-title fork is resolved: implement typed
      instance-conditional quote support before flipping the style; do not
      accept a residual divergence.
- [x] Supporting title configuration is citation-scoped by default, and the
      candidate order no longer lets APA block simpler representable flips.
- [ ] Follow-up beans described in §7 (migrator inference, contributor
      override inventory, and resolved-config warning) are filed before this
      spec becomes Active.

## Changelog

- v1.0 (2026-08-15): Initial version.
- v1.1 (2026-08-15): Revision responding to Codex review. Fixed a real
  nondeterminism bug in `scripts/audit-substitute-formatting.py`
  (`macro_closure()` returned an unordered `set()`); pinned the corpus
  revision; added a `container-title`/`parent-serial` classifier and
  oracle evidence (§1); enumerated the full 32-style embedded-style
  `extends:` graph and identified three new candidates (§3); documented a
  structural limitation (the category resolver can't see container-title
  presence, so `apa-7th`'s real rule isn't representable by type mapping
  alone) and demoted the flip order from a numbered recommendation to a
  blocked-status table (§3, §5); registered this spec in
  `docs/specs/README.md`.
- v1.2 (2026-08-15): Resolved the implementation decisions in §7. Chose a
  typed instance-conditional title-quote capability before the APA flip;
  made supporting title configuration citation-scoped by default; moved
  flat/simple styles ahead of APA; specified proof-based migrator follow-up,
  contributor-override research, and a resolved-config warning instead of a
  runtime error or raw-inheritance special case; added shape-complete test
  requirements for APA, Elsevier Harvard, and Chicago note positions.
- v1.3 (2026-08-15): Corrected a factual error in `elsevier-harvard`'s row
  (§3), found while starting its stage-2 implementation: its
  citation-context substitute rule is type-conditional
  (`author-short`, italic for book-like types, quote otherwise), not
  unconditional italic as v1.1/v1.2 claimed — the classifier had picked up
  the bibliography-facing `author` macro instead, the same short-vs-long
  macro contamination §2 already documents for `apa.csl`/`chicago-author-date.csl`.
  Oracle-confirmed with `dead-sea-scrolls` plus three synthetic fixtures.
  Still the simplest fully type-representable candidate, just not for the
  originally-claimed reason.
- v1.4 (2026-08-24): Pointer-only revision. Added
  `SUBSTITUTED_TITLE_BIBLIOGRAPHY_QUOTING.md` as the bibliography-context
  companion this spec's §5 scoping decision always implied but never named;
  no change to this spec's own citation-scoped analysis, recommendation, or
  acceptance criteria.
- v1.5 (2026-08-26): Pointer-only revision. The companion doc was renamed
  `SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md` after review found its v1.0
  scope (quoting only) was itself too narrow — it now covers every
  bibliography-context title-formatting axis. No change to this spec's own
  citation-scoped content.
