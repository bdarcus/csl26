---
# csl26-6eak
title: Tune gb-t-7714-2025-author-date to full fidelity
status: in-progress
type: task
priority: normal
tags:
    - style
    - fidelity
    - multilingual
created_at: 2026-07-16T10:56:59Z
updated_at: 2026-08-02T14:19:30Z
parent: csl26-ccdt
blocking:
    - csl26-dxuo
blocked_by:
    - csl26-8uxa
    - csl26-m8la
    - csl26-yyrs
    - csl26-o333
---

Drive the embedded gb-t-7714-2025-author-date style to 100% fidelity on the upstream corpus and flip its verification-policy benchmark run back to count_toward_fidelity: true with min_pass_rate 1.0. Citation-side behavior (无日期 / n.d. terms, eight ordered clusters) differs from numeric; re-run the cluster triage before tuning.

## Root cause found (2026-07-22): two precise, mechanical bugs, not scattered defects

Investigated the 0/203 raw baseline (119 ordering issues, 22 year-missing, 32
contributor mismatches from `node scripts/oracle.js
tests/fixtures/csl-m/gb-t-7714-2025-author-date.csl --json --scope bibliography
--refs-fixture tests/fixtures/test-items-library/gb-t-7714-2025.json`). All of
it traces to exactly two mechanical bugs in
`crates/citum-schema-style/embedded/styles/gb-t-7714-2025-author-date.yaml`'s
`bibliography.type-variants` block — not 119 independent defects.

### Mechanism

`gb-t-7714-2025-base.yaml`'s `bibliography.type-variants` is keyed by
comma-joined type-selector strings (e.g. `book,thesis,map`,
`chapter,entry-dictionary,entry-encyclopedia`, 14 keys total, numeric-shaped:
leading `number: citation-number` bracket, date positioned near
publisher/pages). `merge_bibliography_spec`
(`crates/citum-schema-style/src/style/overlay.rs:286`) merges a child style's
`type-variants` **per map key** into the resolved parent's map: "base keys not
in overlay are preserved" (see source comment). `TemplateVariants = IndexMap<
TypeSelector, TemplateVariant>` (order-preserving;
`crates/citum-schema-style/src/template.rs:77`), and
`resolve_type_variant`/`resolve_localized_type_variant`
(`crates/citum-engine/src/processor/rendering/grouped/component_predicates.rs`)
return the **first** entry (by iteration/insertion order) whose selector
matches the item's type.

Consequence: when author-date's key **string** matches base's exactly
(`IndexMap::insert` on an existing key updates in place, same position),
author-date's own override correctly replaces base's. When the strings
**differ** even by one alias, `insert` appends a **new** entry at the end —
base's original (numeric-shaped) entry, positioned earlier, still matches
first and wins. Author-date's own entry becomes silently dead code.

### Bug 1 — key-string mismatches shadow author-date's own overrides

Author-date's `bibliography.type-variants` has 10 keys; base has 14.
Comparing:

| Base key | Author-date key | Match? |
|---|---|---|
| `article,dataset,preprint` | `article,dataset` | **mismatch** (missing `preprint`) |
| `book,thesis,map` | `book,thesis,map,software` | **mismatch** (extra `software`) |
| `manuscript,personal_communication,pamphlet` | `manuscript,personal_communication` | **mismatch** (missing `pamphlet`) |
| `periodical` | — | **missing entirely** |
| `software` | — | **missing entirely** (only reachable via the mismatched book key above, which itself never wins) |
| `graphic` | — | **missing entirely** |
| (7 others: article-journal,article-magazine / article-newspaper / chapter,entry-dictionary,entry-encyclopedia / paper-conference / patent / webpage,post,post-weblog / report / standard) | exact match | OK — author-date's own template is used |

Every item whose type falls under a mismatched or missing key (book=93,
thesis=9, map=7, article=3, periodical/software/graphic, manuscript=3 —
well over half the 203-item corpus) silently renders through base's
numeric-shaped template: leading `[N]` citation-number bracket, date
positioned near publisher/pages instead of after the author. This is exactly
what the failing examples show, e.g. `gbt7714.5.1:1` (book):
`[1]博伯尔. 银行业的未来与人工智能[M]. 徐超，译. 北京：清华大学出版社，2023：35`
— numeric's bracket and date position, not author-date's.

**Fix:** rename author-date's mismatched keys to match base's exactly
(`article,dataset` → `article,dataset,preprint`; split
`book,thesis,map,software` into `book,thesis,map` + a separate `software` key
matching base's; `manuscript,personal_communication` →
`manuscript,personal_communication,pamphlet`), and author `periodical` and
`graphic` entries (currently absent).

### Bug 2 — the type-variants that DO fire are missing delimiters

For the 7 keys that already match (so author-date's own template *is* used),
the components have almost no `prefix`/`suffix`/`delimiter`/wrap punctuation
specified — unlike base's heavily-punctuated equivalents (colons after
container info, commas between contributors, periods between groups). Compare
`gbt7714.8.3.2:1` (chapter, uses author-date's own — matching — key):
oracle `阿扬，2023. 谈谈记忆：与诺贝尔获奖得者埃里克·坎德尔的问答[M]. 姜海伦，译//《环球科学》杂志社. 认识记忆力：关于学习、思考与遗忘的脑科学. 北京：机械工业出版社：15-18`
vs citum
`阿扬无日期谈谈记忆：与诺贝尔获奖得者埃里克·坎德尔的问答姜海伦《环球科学》杂志社认识记忆力：关于学习、思考与遗忘的脑科学北京机械工业出版社15-18`
— components run together with no separating punctuation at all.

Also noticed in passing: the `book,thesis,map,software` entry has `number:
edition` listed **twice** (once plain, once with `label-form: short`) —
likely an authoring/migration artifact, worth removing regardless of the
bigger fix.

**Fix:** add delimiters/prefixes/suffixes to each component, matching base's
punctuation conventions (colon-separated container/place groups,
comma-separated contributor lists, period between top-level segments),
adapted for author-date's front-loaded author+date ordering. Base's
`book,thesis,map` entry (`crates/citum-schema-style/embedded/styles/
gb-t-7714-2025-base.yaml`, top-level `bibliography.type-variants`, the
zh-CN/default-locale one — NOT the `bibliography.locales[0]` EN-scoped
partial override) is the closest reference for the punctuation pattern to
port, with the leading `number: citation-number` component dropped and
`date: issued, form: year` (with the same copyright/printing/accessed
`fallback:` chain used there, for GB/T §7.5.4.3 uncertain-date items) moved to
right after the `contributor: author` component instead.

The existing `message: term.no-date, form: short` component (already used
successfully in author-date's own **citation** section, e.g. lines 395,
416, 457, 509, 529) is confirmed as the correct building block for the
"render year, or 无日期/n.d. if absent" behavior — reuse it in bibliography
type-variants rather than a bare `date:` component, so both scopes share the
same no-date behavior.

### Scope estimate

~13 type-variant keys × ~10-15 components each need delimiter authoring/
verification against the corpus — a substantial but now fully mechanical
task with a clear, demonstrated pattern (no further architecture/engine
investigation needed). Deferred per 2026-07-22 session scoping decision
(best-effort this session; numeric + note landed at full 203-corpus adjusted
fidelity, author-date needs a dedicated follow-up pass). No CSL-M-oracle
"gold" reference exists for author-date structure the way
`data/GB-T_7714—2025.original.toml` (official standard text) did for
numeric's era/EDTF questions — the oracle's own structural shape (field
order, positioning) is the best available reference for this part, since it's
purely a reordering/punctuation question, not a content-correctness one like
the era-annotation case.

- [x] Fix key-string mismatches (Bug 1): article,dataset,preprint /
      book,thesis,map + software / manuscript,personal_communication,pamphlet
      / add periodical, graphic keys
- [x] Add delimiters/punctuation to all ~13 type-variant entries (Bug 2),
      porting base's punctuation pattern with author-date's field ordering
- [x] Remove duplicate `number: edition` entry in book,thesis,map,software
- [x] Re-verify against tests/fixtures/test-items-library/gb-t-7714-2025.json
      (target: adjusted 203/203, matching numeric/note) — reached 125/203 raw,
      152/203 adjusted; target not met, residual findings characterized below
- [ ] Flip verification-policy.yaml count_toward_fidelity: true, min_pass_rate: 1.0

## Progress (2026-07-22, later session): recipe validated to 68%, then reverted — new blockers found

Implemented the Bug 1 + Bug 2 fix plan above: rewrote `bibliography.type-variants`
to 14 keys matching base's exact selector strings (`article,dataset,preprint`;
`article-journal,article-magazine`; `article-newspaper`;
`chapter,entry-dictionary,entry-encyclopedia`; `periodical`;
`manuscript,personal_communication,pamphlet`; `book,thesis,map`; `software`;
`paper-conference`; `patent`; `graphic`; `webpage,post,post-weblog`; `report`;
`standard`), each built from a shared leading block:

```yaml
- group:
    - contributor: author
      form: long
      name-order: family-first
      delimiter: { mark: comma }
      sort-separator: ' '
      and: none
    - date: issued
      form: year
      fallback:
        - date: copyright
          form: year
          prefix: c
        - date: printing
          form: year
          suffix: 印刷
        - date: accessed
          form: year
          wrap: { punctuation: brackets }
        - message: term.no-date
          form: short
  delimiter: { mark: comma }
  suffix: '. '
```
followed by the rest of base's per-type fields (delimiters ported from base),
with body-position `date:issued form:year` occurrences removed (redundant
with the front block) but body-position `date:issued form:year-month-day`
occurrences **kept** — GB/T §7.5.4.2 mandates full-precision dates in the body
for online/report/patent/newspaper/archival types regardless of the
author-date front-matter year.

This surfaced a genuine **engine bug**, independent of this style: dates were
being silently deduplicated by `TemplateComponentTracker` (a
Contributor-only, `cs:substitute`-equivalent suppression mechanism) whenever
the same `date: issued` variable appeared twice in a template — which is
exactly what author-date's front+body dual-date shape does on purpose. Fixed
in `crates/citum-engine/src/processor/rendering/mod.rs`
(`TemplateComponent::Date(_) => None` in `get_variable_key`, full exemption)
and **landed separately** from this bean's YAML work, since it's a real
cross-cutting defect (also fixed a second, pre-existing occurrence in
`taylor-and-francis-chicago-author-date-core.yaml`'s `interview:` variant).
See commit history on `fix/gbt-date-annotation-fidelity` — do not re-derive
this fix if resuming this bean; it's already in `main`/merged upstream by the
time this bean is picked back up.

With the YAML recipe above **and** the engine fix, the corpus went from
**raw 0/203 → 111/203, adjusted 0/203 → 139/203 (68%)** — real, verified
progress. But **the YAML change was reverted** (not committed) before
closing out, because triaging the remaining 64 failures surfaced three new,
unresolved design questions the recipe above doesn't answer — landing it
would have meant shipping (and test-pinning) behavior with evidence against
its correctness:

### New finding 1 — no-author items: 佚名 (anonymous term), not title-substitution

`gbt7714.7.5.4.1:1` (Minguo-era, no author): oracle renders
`佚名，1947. [M]` (佚名 = "anonymous" — a placeholder **term**, à la
`term.no-date`'s "无日期"). The recipe above instead lets `contributor:
author` fall through to Citum's existing title-substitution behavior,
producing `1947（民国三十六年）. [M]` with no leading placeholder at all (and
in an earlier synthetic test, title substituting into the author slot
instead). **The real GB/T author-date convention needs an explicit
`message: term.anonymous`-shaped fallback in the author position**, mirroring
how `term.no-date` handles the missing-date case — this doesn't exist yet
and needs its own small feature, not just a punctuation port. Also directly
affects the archival "does the era annotation legitimately render twice
(front block + body block)" question raised by the reverted
`date_annotations.rs` assertions — unresolved, do not re-pin those tests
without re-deriving against a confirmed-correct rendering.

### New finding 2 — organizational author for `standard` type

`gbt7714.8.9.2:1`: oracle shows the issuing committee as the author slot
(`全国信息与文献标准化技术委员会，2021. GB/T 3792—2021 title[S]`); Citum's
`standard` type-variant has no path to promote the issuing-org/publisher
field into the author position, so it falls through to some other
substitution and drops the org name entirely. Likely needs a
`substitute.template` extension (currently only covers `editor`) — a
distinct, possibly non-trivial sub-feature, not addressed by the recipe.

### New finding 3 — disambiguation-suffix ordering swap

`gbt7714.9.3.1.3:2` / `:3`: two same-author-same-year references get `2000b`
/`2000c` swapped relative to the oracle's ordering. This looks like a
pre-existing disambiguation-ordering algorithm difference (not
type-variant/punctuation related) — plausibly out of scope for this bean
entirely; may deserve its own bean if confirmed.

### Where things stand

- The 14-key type-variant restructuring **shape** (base-matching keys, shared
  leading block, full-precision body dates preserved) is validated and
  should be the starting point for the next attempt — just don't reuse it
  verbatim for the author slot until finding 1 is resolved.
- Do not re-add the reverted YAML from `git log` blindly; it encodes the
  title-substitution behavior finding 1 says is wrong.
- Recommend resolving finding 1 (佚名 term) first — it's likely the single
  highest-leverage fix (affects the `7.5.4.1`/`7.5.4.3` cluster and probably
  more no-author items in the wider corpus), then finding 2 (org-as-author,
  narrower blast radius — `standard` type only), then assess finding 3
  separately.

## Session progress (2026-07-23): recipe + finding 1 landed, 0/203 → 125/203 raw (61.6%), 152/203 adjusted (74.9%)

Rebuilt `gb-t-7714-2025-author-date.yaml`'s entire `bibliography.type-variants`
block from `gb-t-7714-2025-base.yaml`'s verified-correct structure (Bug 1 key
mismatches + Bug 2 missing punctuation, both fixed together as the bean
already anticipated), converting each of the 14 variants to author-date's
front-loaded shape: `contributor: author` (now carrying a `fallback:
[{message: term.anonymous, form: short}]`) followed by a front `date: issued,
form: year` (carrying either the book/thesis/map copyright→printing→accessed
chain from base, or a plain `fallback: [{message: term.no-date, form:
short}]` terminal). Full-precision body dates (GB/T §7.5.4.2's
online/report/patent/webpage/newspaper/archival types) are kept verbatim from
base and marked `suppress-note: true` on the new front occurrence so the
calendar-note annotation lands on the full-precision date only (csl26-gl0n's
mechanism, now with a real consumer). Duplicate `number: edition` (noted in
the original triage) is gone as a side effect of rebuilding from base.

**Finding 1 (佚名 anonymous-author term) is implemented, not just designed:**
- `TemplateContributor.fallback: Option<Vec<TemplateComponent>>` — new
  engine-level mechanism (`crates/citum-schema-style/src/template.rs`,
  `crates/citum-engine/src/values/contributor/mod.rs`'s
  `resolve_author_fallback`), mirroring `TemplateDate.fallback` exactly:
  consulted only when the entire `substitute.template` chain (editor/title/
  translator) is exhausted.
- `bibliography.options.substitute: {template: [editor, translator]}` —
  GB/T author-date has no title-as-author convention; the inherited global
  `substitute: standard` preset's `title` entry was winning before every
  no-author item reached the new fallback (nearly every reference has *some*
  title). An empty `template: []` override is a no-op per
  `Substitute::merge` (`if !other.template.is_empty()`), not a clear — this
  cost real debugging time before landing on the non-empty override.
- `options.messages: {term.anonymous: 佚名}` — style-owned MF2 message, not
  a locale term. `Processor::new` seeds a plain `Locale::en_us()` base
  locale; only a matched `locales:[]` branch or per-item `language` (with
  `multilingual.term-locale: item`) ever swaps in the real zh-CN locale. A
  locale-owned `term.anonymous` would have silently resolved through CSL's
  own pre-existing (legacy, English) generic-term catalog entry ("anon.")
  for any item lacking an explicit `language` tag — which is exactly what
  happened before this fix (traced via targeted `eprintln!` instrumentation,
  since three independent code-read hypotheses were each wrong in turn).
- New disambiguation-grouping key: `Disambiguator::build_author_slot_key`
  (`crates/citum-engine/src/processor/disambiguation.rs`) previously gave
  every no-author reference a *unique* singleton key when the substitute
  chain resolved to `None` — correct when a substituted *title* already
  distinguishes entries, wrong when a *constant* fallback term (佚名) is
  what actually renders, since every such reference then needs to collide
  on year like a real shared author. Added a stable sentinel key
  (`ANONYMOUS_FALLBACK_KEY`) for the `None` case.
- New `TemplateDate.suppress_disamb_suffix: Option<bool>` — the mirror of
  `suppress_note`: rendering `issued` twice meant the year-suffix
  disambiguator (`1947a`) was inlining into *both* occurrences, corrupting
  the full-precision body date (`2012c-05-03` instead of `2012-05-03`).
  Marked `true` on all 8 body full-precision `date: issued` occurrences.

`just pre-commit` green (2161/2161 tests, fmt, clippy). `just
check-core-quality` clean (157 styles, fidelity=1.0, 0 warnings) — no
regressions elsewhere in the corpus. `verification-policy.yaml` left
untouched (`count_toward_fidelity: false`) — not remotely close to the 1.0
bar it requires.

### Residual findings, precisely characterized (not attempted — real design
### calls or larger features, per 2026-07-23 session scoping decision)

Of the 78 remaining raw failures (51 after `adjusted` divergence
normalization), triaged by root cause:

1. **Finding 2 — org-as-author for `standard` type** (3 entries,
   `gbt7714.8.9.2:*`). Unchanged from the original triage: GB/T shows the
   issuing committee as author (`全国信息与文献标准化技术委员会，2021`);
   Citum's `standard` type-variant has no path to promote an org/publisher
   field into the author position. Needs a `substitute.template` extension
   (a new `SubstituteField` variant, or a `TemplateContributor`-shaped
   publisher-as-contributor path) — out of scope, unattempted.
2. **Finding 3 — disambiguation-suffix ordering swap** (5 entries,
   `gbt7714.9.3.1.3:2/3`, `gbt7714.8.5.3:4/5`, confirmed present on both
   real-author and 佚名-fallback items now). `2000b`/`2000c` (or
   `2024a`/`2024b`) assigned in the opposite order from the oracle's — a
   pre-existing disambiguation-ordering algorithm difference, unrelated to
   this bean's YAML/mechanism work. Likely deserves its own bean if pursued.
3. **NEW — disambiguation-suffix grouping doesn't fire for the anonymous
   fallback in the real style** (29 entries, the largest residual bucket).
   The `ANONYMOUS_FALLBACK_KEY` sentinel above only takes effect if the
   `Disambiguator` that computes bibliography hints is constructed with the
   *bibliography*-scoped `Substitute` config; it's actually constructed with
   the *citation*-scoped one
   (`crates/citum-engine/src/processor/setup.rs:635`'s `calculate_hints`,
   `config = self.get_citation_config()` passed as `Disambiguator::new`'s
   `config` field, not `bibliography_config`). This is a **pre-existing**
   citation/bibliography config-scoping inconsistency, invisible until now
   because no style previously needed the two `Substitute` chains to
   differ. Fixing it means changing a shared code path used by every
   style's disambiguation-hints calculation — too risky to land without
   dedicated cross-corpus testing (`just check-core-quality` across all 157
   styles) in this session. The `sort_config` field's doc comment
   ("Year-suffix ordering must use this — not `config`") suggests this
   distinction was already a known sharp edge.
4. **NEW — the anonymous-author term needs to be item-language-aware**
   (10 entries, `gbt7714.7.5.*`/`7.2.1:7`/`7.4:5-6`). GB/T's real
   convention is bilingual: Chinese items render `佚名`, English-language
   items render `Anon` (oracle: `Anon，1975. [M]. Macmillan`). The
   style-owned `messages: {term.anonymous: 佚名}` fix above is a uniform
   constant — correct for this corpus's Chinese majority (net *more*
   correct than before, since Chinese items now genuinely say 佚名 instead
   of accidentally-coincidentally-right-looking English "Anon." for
   everything), but wrong for English-language items, which is a real,
   measurable regression against the `adjusted` oracle count for exactly
   those 10 items (162→152). No `MessageArgSource` variant currently
   exposes an item's own language to an MF2 message pattern (the existing
   `gb-t-7714-type-code` message's `.match {$type :select}` pattern proves
   the *mechanism* works — reference-type and carrier are already
   selectable args); a `MessageArgSource::ItemLanguage`-shaped addition
   (or wiring through the existing `multilingual.term-locale: item`
   per-item-locale mechanism instead) would let `term.anonymous` select
   `佚名`/`Anon` by item language the same way the type-code message
   already selects by type/carrier.
5. **3 unclassified singles** — `gbt7714.8.5.1.1:7` (body date silently
   dropped rather than rendering `2024-05-09`), `gbt7714.7.2.1:4` (oracle
   shows a bare disambiguation letter `b` with no `无日期-` prefix — an
   outlier in the no-date suffix sequence, not yet understood),
   `gbt7714.8.11.3.2:5`/`gbt7714.8.11.2.2:1` (an approximate/bracketed year
   `[2025]`/`[2024]` falls through to the no-date term instead of the
   approximate-year value — the front `date: issued, form: year` component
   may not be reading `DateValue`'s uncertainty/approximation markers the
   same way the body's fuller date components do). Not triaged further.

### Recommended order for a future pass

Finding 4 (grouping-key config scoping) is the single highest-leverage
remaining item — 29 of 51 adjusted failures — but requires the most care
(shared code path, cross-corpus verification). Finding 5 (language-aware
anonymous term) is smaller (10 entries) and more contained (one new
`MessageArgSource` variant or a routing fix, no shared-path risk). Findings
2 and 3 are narrower, independent, and can be picked up in either order or
split into their own beans.

## Session progress (2026-07-23, later session): three engine gaps found and fixed, 125/203 → 141/203 raw (69.5%), 152/203 → 162/203 adjusted (79.8%)

Picked up from the prior 2026-07-23 session's 125/203 raw / 152/203 adjusted
baseline. Fresh triage of the 51 adjusted failures showed the dominant
pattern was missing year-suffix disambiguation on anonymous/no-date
fallback entries (oracle: `无日期-a … 无日期-v`, `2011a`/`2011b`; citum: bare
`无日期`/`2011`). Root-caused to **three independent, precisely-located
engine gaps**, not one — each confirmed via direct instrumentation
(`ProcHints` dumps, CLI render diffs) before fixing, per repeated
empirical checkpoints rather than assumption:

1. **Disambiguation grouping used the citation-scoped substitute, not the
   bibliography-scoped one** — `build_reference_cache`
   (`crates/citum-engine/src/processor/disambiguation.rs`) resolved the
   anonymous-fallback collision key from `self.config.substitute`
   (citation-scoped: includes `title`) instead of `self.sort_config`
   (already documented as the effective bibliography config). A style
   overriding only the bibliography-scope substitute (GB/T author-date's
   `[editor, translator]`, no `title`) therefore saw every anonymous
   reference resolve a *distinct* substituted-title key at the grouping
   layer, even though the bibliography itself renders the same constant
   `佚名` fallback for all of them — singleton groups, no suffix. Fixed by
   reading `sort_config.substitute`; all current call sites already pass
   the bibliography config as `sort_config`, so this is a no-op everywhere
   else (confirmed: 157-style `check-core-quality` clean).

2. **`TemplateDate`'s explicit `fallback:` branch never computed or
   attached a disambiguation suffix at all** — regardless of which
   fallback candidate resolved. The suffix mechanism
   (`compute_disamb_suffix_label` / `append_no_date_disamb_suffix` /
   `inline_disamb_suffix`, `crates/citum-engine/src/values/date.rs`) only
   existed in the implicit (no explicit `fallback:`) no-date branch and
   the real-date branch. GB/T author-date's date components always carry
   an explicit `fallback:` chain (front-loaded `date: issued` with
   `copyright → printing → accessed → message: term.no-date`), so even
   after fix (1) correctly grouped references, *nothing* ever attached a
   letter. Fixed by applying the same append-suffix convention when the
   winning fallback candidate is a `message:` component — by construction
   the terminal "no data available" case in a date's fallback chain —
   respecting `suppress-disamb-suffix` like the other two branches. This
   alone, with (1), still moved **zero** oracle numbers (verified) — see
   (3).

3. **The anonymous term was a locale-independent style constant, and
   grouping didn't scope by language either** — `佚名` was pinned via a
   style-owned `options.messages: term.anonymous: 佚名` override, which
   `TemplateMessage::values` checks *before* consulting the active
   locale, so it applied unconditionally regardless of item language.
   Oracle actually keeps **two independent** year-suffix letter sequences
   — `佚名，无日期-a…w` (Chinese) and `Anon，n.d.-a…j` (English) — because the
   rendered term differs by language, so CSL treats them as different
   "authors." Root cause for why `term.anonymous` couldn't just move to a
   locale file directly: `general_message_id()`
   (`crates/citum-schema-style/src/locale/message_ids.rs`), which decides
   which `GeneralTerm`s get MF2-message-first treatment ahead of the
   legacy structured `terms:` catalog, had `NoDate`/`Circa` but not
   `Anonymous` — so a locale's `messages.term-anonymous` entry was never
   even consulted; resolution always fell straight to the hardcoded
   legacy default (en-US: `"anon."`). Fixed in three parts: (a) add
   `Anonymous` to `general_message_id`'s allowlist, mirroring `NoDate`
   exactly; (b) drop the style-owned override, add
   `term.anonymous`/`term.anonymous-long` to `zh-CN.yaml` (`佚名`) and
   `en-US.yaml` (`Anon`); (c) scope the `ANONYMOUS_FALLBACK_KEY` sentinel
   in `build_author_slot_key` by `effective_item_language(reference)` —
   the same driver `process_bibliography_entry_with_format` already uses
   to pick a `bibliography: locales:` branch — so Chinese and English
   anonymous items collide into separate sequences instead of one merged,
   scrambled one.

Three previously-passing pinned tests
(`crates/citum-engine/tests/date_annotations.rs`) regressed once (3)
landed: they construct a bare `Processor::new` (seeds `Locale::en_us()`)
directly rather than resolving the style's `info.default-locale` the way
the real `citum render` CLI does (`create_processor`,
`citum-cli/src/style_resolver.rs`) — they only passed before because the
old override was locale-independent. Fixed the harness (constructed with
the embedded zh-CN locale, mirroring the existing
`localized_author_date_grouping_uses_the_selected_term_locale` precedent
in `crates/citum-engine/tests/i18n.rs`), not the assertions — the expected
`佚名` strings are unchanged and correct.

`just pre-commit` green (2161/2161, fmt, clippy). `just check-core-quality`
clean (157 styles, fidelity=1.0, 0 warnings) — the shared `en-US.yaml`
term.anonymous default change (`"anon." → "Anon"` for styles that
reference `message: term.anonymous`/`GeneralTerm::Anonymous`) does not
regress any other embedded style (verified no other embedded style uses
`message: term.anonymous` or `term: anonymous` as a component).

### Residual findings (41 adjusted failures), precisely characterized

1. **NEW, dominant — suffix *order within* the anonymous collision group
   doesn't match oracle's render order** (~28 of 41: the entire 无日期
   bucket plus `2011a/b`, `2012a/b`, `2023a/b/c`, `2024a/b` swapped
   pairs). Confirmed via direct evidence, not assumption: citum's own
   *render* position for the 佚名+无日期 bucket (0, 1, 2, 3…) does not
   correlate at all with its own assigned *letters* (k, w, x, a…) —
   `sort_group_for_year_suffix`'s no-`group_sort` branch
   (`disambiguation.rs`) hardcodes a title-alphabetical tiebreak
   (`a_title.cmp(b_title)`) as the *default* ordering whenever no
   explicit `bibliography.sort:`/`group_sort` is configured — which GB/T
   author-date doesn't set. That assumption doesn't match this style's
   actual (apparently insertion-order) bibliography arrangement. This is
   an **internal inconsistency** (citum's own suffix order disagrees with
   citum's own render order), not a deeper bibliography-sort divergence
   from oracle — the fix is in the same shared no-`group_sort` fallback
   used by *every* style without an explicit `group_sort`, so it needs
   its own dedicated pass with full `check-core-quality` verification.
   Single highest-leverage remaining item.
2. **Finding 2 — org-as-author for `standard` type** (3 entries,
   `gbt7714.8.9.2:1-3`, unchanged from prior triage): oracle promotes the
   issuing committee into the author slot; Citum's `standard`
   type-variant has no path to promote an org/publisher field into the
   author position. Needs a `substitute.template` extension (new
   `SubstituteField` variant or a publisher-as-contributor path).
3. **Date-parsing singles** (`gbt7714.8.11.2.2:1`, `8.11.3.2:1/5`):
   approximate/bracketed year (`[2025]`) falls through to the no-date term
   instead of the approximate-year value — the front `date: issued`
   component's `fallback:` chain doesn't reach `accessed` the way the
   rendered oracle text implies it should for these types.
4. **2 unclassified singles**: `gbt7714.8.5.1.1:7` (body date silently
   dropped), `gbt7714.7.2.1:4` (oracle shows a bare disambiguation letter
   `b` with no `无日期-`/`b` prefix pattern — an outlier, not understood).

`verification-policy.yaml` left untouched (`count_toward_fidelity: false`)
— 141/203 raw is nowhere near the 1.0 bar it requires. Not flipped.
