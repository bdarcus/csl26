# Number-range abbreviation is inconsistent across the citation

- **Date:** 2026-08-26
- **Bean:** `csl26-zepc` (epic: `csl26-awlo`)
- **Audience:** PM / style domain expert. Implementation evidence for
  engineers is in the appendix.

## What's wrong

Most citation styles abbreviate a page range: Chicago writes `112–118` as
`112–18`, not the full four digits on both ends. Citum gets this right in the
bibliography but not in the in-text citation, for the same reference, in the
same style:

```
Bibliography entry (correct):
  Kuhn, Thomas S. 1970. "Scientific Paradigms and Normal Science."
  Philosophy of Science 37 (1): 1–13.

In-text citation, same book, same style (wrong):
  (Wilson 2019, 112–118)     ← Citum
  (Wilson 2019, 112–18)      ← what Chicago/citeproc-js actually does
```

That's the bug that started this audit. Looking for it turned up something
bigger: **abbreviation behavior isn't one setting that's occasionally
misapplied — it's five different code paths that each decide, on their own,
whether and how to shorten a number range**, and they don't agree with each
other. Some respect the style's stated abbreviation rule; some ignore it
entirely; two of them don't even use the same punctuation mark for the range
dash. A style author writing YAML currently has no reliable way to say "make
every range in this style follow rule X" — only "make ranges in the
bibliography page count follow rule X, and hope the rest come out right."

## Where it shows up

| Where a range appears | Follows the style's stated rule? |
|---|---|
| Bibliography page count (`Philosophy of Science 37 (1): 1–13`) | ✅ Yes |
| In-text citation locator (`Wilson 2019, 112–18`) | ❌ No — always renders in full, no matter what the style asks for |
| Publication-date year ranges (`2021–26`) | ⚠️ Partially — there's a rule, but it's a completely separate on/off switch from the page-range rule above, so the two can (and do) disagree within one style |
| Consecutive citation numbers in numeric styles (`[1–3]`) | ✅ Works, but always fully expanded — a numeric style that wants the CMOS-style short form (`[1–3]` vs. `[1-3]` vs. some other convention) can't ask for it |
| Same-author, same-year suffix runs (`Smith 2020a–c`) | Works, but through a fourth independent switch |
| Alphabetic sub-entry runs in a compound citation (`1a-c`) | Works, but joins with a plain hyphen (`-`) instead of the en dash (`–`) every other range in the document uses |

Four different features, four different on/off switches, and the two that
use a dash disagree on which dash. None of the four can be told "use the
style's page-range rule" — each one hardcodes its own answer, so the actual
behavior a reader sees depends on *which kind* of range it is, not on
anything the style author configured.

## Why this happened

Page-range abbreviation was implemented once, correctly, for the bibliography
page count. When locators, dates, citation-number lists, and compound
sub-entries were added later, each one got its own small, separate switch
instead of reusing the first one — so today there is no single place a style
author (or Citum itself) can point to and say "this is how ranges abbreviate
in this style." Two of the newer switches (citation-number ranges and
same-author year-suffix ranges) don't even expose a switch — the dash they
use is a hardcoded literal in the Rust code, so no style, however it's
written, can change it.

There's also a documentation gap: `docs/guides/style-authoring/` — the guide
a style author would actually read — never explains any of this. The one
mention of range abbreviation in the guide is a locale message that, per the
appendix below, nothing in the engine actually reads.

## What this means for the fix

This is a design problem, not a one-line bug fix. The locator bug could be
patched in isolation, but patching it without addressing the rest would leave
three more silently-inconsistent switches in place, plus the dead
documentation. `docs/specs/RANGE_COLLAPSE_MODEL.md` proposes one shared rule
that all five switches follow, and asks for a decision on the handful of
places where the "shared rule" needs a judgment call — that's the doc worth
reading and deciding on next.

A separate, narrower defect — 9 styles that lost their citation-number
abbreviation entirely somewhere in the CSL 1.0 → Citum conversion — is
tracked as `csl26-rgys`. It's related, but it's a data-migration gap, not a
design gap, so it's kept out of this audit and should wait for the design
decision above rather than being patched ad hoc.

---

## Appendix: implementation surfaces (for engineers)

Twelve config fields and four rendering code paths implement the six rows
above. Full inventory, with `file:line` evidence for every claim in the body
of this audit:

| # | Surface | Type | Consumer(s) | Note |
|---|---|---|---|---|
| 1 | `options.page-range-format` | `Option<PageRangeFormat>` (5 variants: `expanded`, `minimal`, `minimal-two`, `chicago`, `chicago16`) | `citum-engine/src/values/number.rs:47` | Only reached by the `page` variable (bibliography row) |
| 2 | `options.page-range-delimiter` | `Option<String>` | `citum-engine/src/values/number.rs:42` | Falls back to locale #12 when unset — only surface that does |
| 3 | `citation.options.page-range-format` | `Option<PageRangeFormat>` | `citum-schema-style/src/options/mod.rs:292` → merges into `Config` | Scoped format override with no sibling scoped delimiter field; see incoherence 4 |
| 4 | `bibliography.options.page-range-format` | `Option<PageRangeFormat>` | `options/mod.rs:415` → merges into `Config` | Scoped format override with no sibling scoped delimiter field; see incoherence 4 |
| 5 | `options.locators.range-format` | `PageRangeFormat`, **non-optional, hardcoded default `Expanded`** | `citum-engine/src/values/locator.rs:225` (`effective_range_format`) | Shadows #1 for every locator kind — this is the citation-locator row |
| 6 | `options.locators.kinds.<kind>.range-format` | `Option<PageRangeFormat>` | same fn, checked first | Already optional; the pattern #5 should follow |
| 7 | `options.dates.range-format` | `DateRangeFormat` (2 variants: `expanded`, `chicago`) | `citum-engine/src/values/date.rs:510` | Parallel enum to #1, not the same type — the date-range row |
| 8 | `options.dates.range-delimiter` | `String`, default `"–"` | `citum-engine/src/values/date.rs:422,855` | Parallel to #2, independent field |
| 9 | `citation.collapse: citation-number` | enum variant, no associated config | `citum-engine/src/processor/rendering/collapse.rs:85` (`collapse_numeric_citation_chunks`) | Builds `format!("{n}–{m}")` — literal en-dash, always fully expanded, ignores #2 and #12 entirely. The citation-number-list row |
| 10 | `citation.collapse.same-author.year_suffix: ranged` | enum variant | `citum-engine/src/processor/rendering/grouped/year_suffix.rs:22` (`RANGE_DELIMITER` const) | A **third**, independently hardcoded `"–"` literal — the const's own doc comment notes it deliberately does *not* reuse #2/#12. The same-author-suffix row |
| 11 | `bibliography.options.compound-numeric.collapse-subentries` | `bool` | `citum-engine/src/processor/rendering/mod.rs:487` | **Not** a range-formatting consumer — gates whether subentry grouping happens at all. The actual alphabetic range join (`collapse.rs:198`, `format!("{a}-{b}")`, literal ASCII hyphen, comma-joined) is instead gated by `compound_numeric.sub_label == Alphabetic`, a separate field. The compound-sub-entry row |
| 12 | locale `grammar-options.page-range-delimiter` | `String` | `number.rs:43`, `locator.rs:97,116,151,183` | The one value multiple consumers share — but reached through two different resolution rules (see incoherence 2) |
| 13 | locale `messages: pattern.page-range` | MF2 message (`"{$start}–{$end}"`) | **nothing reads it** | Defined in `en-US`, `es-ES`, `eu-ES`, `fr-FR`, `tr-TR`; referenced by zero styles and zero engine code — this is the "one mention in the style-authoring guide" that turns out to be dead |

Also checked and confirmed **not** a range-formatting consumer:
`NumberVariable::Volume` / `::Issue` (`values/number.rs:26,28`) render via a
bare `reference.volume()/.issue().to_string()` — a hyphenated volume value
like `"3-4"` passes through unnormalized, touching none of #1/#2/#5/#12.

### Reproducer

```
$ citum render refs -s styles/embedded/chicago-author-date-18th.yaml \
    -b <chapter with pages "112-118"> -c <citation with locator page 112-118> -m cite

Citum:      (Wilson 2019, 112–118)
citeproc-js: (Wilson 2019, 112–18)
```

`chicago-author-date-18th.yaml` sets `options.page-range-format: chicago16`
(inherited from `chicago-18-base.yaml:11`) and `options.locators: note`. The
bibliography `page` variable reads that setting correctly. The citation
locator does not, because it never sees it — surface #5.

### Concrete incoherences

**1. Format shadowing.** #5 is non-optional and every `LocatorPreset::config()`
arm (`options/locators.rs:199` Note, `:221` AuthorDate, `:230` Numeric)
hardcodes `PageRangeFormat::Expanded`, so #1 is structurally unreachable for
any locator in any style using a preset — which is every style in the
portfolio except the four with an explicit `locators:` map
(`american-medical-association`, `oscola`, `oscola-no-ibid`, `mhra-notes`).
Only `mhra-notes.yaml:11` sets #5 directly, to `expanded`.

**2. Delimiter resolution runs backwards between the two nearest surfaces.**
`values/number.rs:42-44` resolves #2 first, falling back to locale #12 only
when the style leaves #2 unset. `values/locator.rs:97,116,151,183` reads #12
directly and never consults #2 at all — the same axis, two different and
incompatible resolution rules, in the same crate.

**3. Three independently hardcoded dash literals.** `collapse.rs:85`
(citation-number ranges, `"–"` en-dash, always fully expanded — a numeric
style that wants `[1-3]` collapsed to `[1–3]` gets it, but one that wants the
CSL `minimal` abbreviation instead has no way to ask), `collapse.rs:198`
(compound sub-labels, `"-"` ASCII hyphen — a different character from the
other two), and `year_suffix.rs:22`'s `RANGE_DELIMITER` const (en-dash again,
with a comment explicitly declining to reuse #2/#12). None of the three reads
any config surface.

**4. Scoped formats have no sibling delimiter override.** `CitationOptions`
and `BibliographyOptions` expose `page_range_format` but not
`page_range_delimiter`. Their `Config` conversions set
`page_range_delimiter: None` (`options/mod.rs:855-856`, `:989-990`), and
`Config::merged` preserves the style-level delimiter because `None` does not
override it. A style can therefore choose a scoped abbreviation format, but it
cannot pair that choice with a scoped delimiter; it must use the style-level
or locale delimiter.

**5. Two parallel enums encode the same idea.** `PageRangeFormat` (5 variants)
and `DateRangeFormat` (2 variants) are distinct Rust types with no conversion
between them, yet `values/date.rs:530` already calls
`values::number::format_chicago_range_end` — the page-range module's own
Chicago-abbreviation algorithm — to implement `DateRangeFormat::Chicago`. The
algorithm is shared; the configuration vocabulary that selects it is not.

**6. Dead surface.** #13 (`pattern.page-range`) is authored in five locale
files and consumed by nothing. Either it predates #1-#12 as an earlier design
for the same problem, or it was never wired up.

**7. Zero public documentation.** `grep -ri range docs/guides/style-authoring/*.html`
returns exactly one hit: the dead #13, in `locales.html`. A style author
reading the guide has no way to learn that `page-range-format`,
`locators.range-format`, `dates.range-format`, and `collapse` exist, let alone
how they interact.
