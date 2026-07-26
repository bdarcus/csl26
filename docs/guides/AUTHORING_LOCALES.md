# Authoring Locales

This guide tells you when and how to add MessageFormat 2 (MF2) entries to a
Citum locale file in the
[embedded locale directory](../../crates/citum-schema-style/embedded/locales/).
It is the practical companion to the
[locale messages specification](../specs/LOCALE_MESSAGES.md).

## Status

- MF2 evaluator: live in
  [`message.rs`](../../crates/citum-schema-style/src/locale/message.rs).
- Engine call sites: live. `resolved_locator_term` and `resolved_role_term`
  are wired in the
  [engine value modules](../../crates/citum-engine/src/values/)
  (locator, number, contributor/labels)
  and consult the `messages:` map first, falling back to the legacy `terms:` /
  `roles:` / `locators:` maps.
- Gendered term values: live through `MaybeGendered<T>` in the legacy locale
  maps and through the scoped `$gender` x `$count` MF2 role-label pattern.
- Coverage as of writing: `en-US`, `de-DE`, `fr-FR`, `tr-TR`, `es-ES`, `zh-CN`,
  `ar-AR`, `ja-JP`, `ko-KR`, `ru-RU` carry v2 `messages:` blocks; `zh-CN` and
  `ar-AR` also now carry `grammar-options` and `date-formats` (previously
  absent, silently falling back to English typography/dates — see the
  locale-completeness lint in `lint_raw_locale`
  ([crates/citum-schema-style/src/lint.rs](../../crates/citum-schema-style/src/lint.rs)). `ja-JP`,
  `ko-KR`, and `ru-RU` are new locales authored at `de-DE`-level structural
  completeness (dates, roles, terms, messages, date-formats,
  grammar-options, legacy-term-aliases); their vocabulary has not yet had
  native-speaker review (see the EXPERT REVIEW NEEDED comment in each
  file). Adding new locales should follow the same shape.

If a `messages:` block is absent or empty, the engine silently uses the legacy
maps. There is no per-locale fallback alarm — author intent is signalled by
`evaluation.message-syntax: mf2` in the locale header.

## When to add a `messages:` entry

Author a v2 `messages:` entry whenever the rendered text depends on a
**parameter** that the legacy term tables can't carry compositionally:

| Pattern | Example | Use MF2? |
|---------|---------|----------|
| Plural-dependent abbreviation | `p.` vs `pp.` | **Yes** — `.match {$count :plural}` |
| Plural-dependent long form | `chapter` vs `chapters` | **Yes** |
| Plural-dependent verb | "with guest" vs "with guests" | **Yes** |
| Compositional pattern | `{$start}–{$end}` for page ranges | **Yes** |
| URL-bearing message | "retrieved from {$url}" | **Yes** |
| Static label | `term.and: "and"` | Optional — works either way |
| Gender-dependent label | Spanish `editor / editora`, French `éditeur / éditrice` | **Yes**, after test coverage — see "Gender" below |

The rule of thumb: if the message body would contain `{` (variable
substitution or `.match` block), MF2 is the correct home. If it's a bare
string, both paths render identically; MF2 is preferred when authoring a new
locale for consistency with the v2 schema.

## Required block when `message-syntax: mf2`

A v2-aware locale should ship the following four blocks at the bottom of the
file, after `terms:`, `roles:`, and `locators:`:

```yaml
messages:
  # plural-dispatched and parameterized strings — see catalog below
date-formats:
  numeric-short: "..."
  textual-long: "..."
  textual-full: "..."
  bib-default: "..."
  year-only: "yyyy"
  iso: "yyyy-MM-dd"
grammar-options:
  punctuation-in-quote: false
  open-quote: "..."
  close-quote: "..."
  serial-comma: false
  page-range-delimiter: "..."
  strong-terminal-comma-policy: keep-both
  delimiter-suppressing-terminal-marks: "?!…"
legacy-term-aliases:
  # bridge old engine keys to new message IDs
```

Use `strong-terminal-comma-policy: keep-terminal` for locales that suppress a
style-supplied comma after `?`, `!`, or `…` (currently German and French).
Styles can override either punctuation field under `options.punctuation`.
`delimiter-suppressing-terminal-marks` is also the shared locale vocabulary
for structured-title delimiter suppression; title rendering consumes it in
the separately tracked `csl26-zfqr` work.

Use [`en-US.yaml`](../../crates/citum-schema-style/embedded/locales/en-US.yaml),
[`fr-FR.yaml`](../../crates/citum-schema-style/embedded/locales/fr-FR.yaml), or
[`de-DE.yaml`](../../crates/citum-schema-style/embedded/locales/de-DE.yaml) as references for
gender-invariant messages. `es-ES.yaml` is the current concrete example of a
locale that carries MF2 messages while keeping gendered role labels in `roles:`
until MF2 can dispatch on both `$gender` and `$count` in one message. The same
limitation applies to other gendered locales, including French and Arabic.

## Message ID catalog (frequently used)

| ID | Variables | Notes |
|----|-----------|-------|
| `term.page-label`, `term.page-label-long` | `$count` | `p./pp.` and `page/pages` equivalents |
| `term.chapter-label`, `term.chapter-label-long` | `$count` | |
| `term.volume-label`, `term.volume-label-long` | `$count` | |
| `term.section-label`, `term.section-label-long` | `$count` | |
| `term.figure-label` | `$count` | |
| `term.role-conjunction` | — | Connector for dynamically composed combined-role labels; include surrounding spacing. |
| `term.note-label`, `term.note-label-long` | `$count` | |
| `term.archive-collection-label` | none | Archive hierarchy: collection name |
| `term.archive-series-label` | none | Archive hierarchy: series name |
| `term.archive-box-label` | `$count` | Archive hierarchy: box/container (plural-dispatched) |
| `term.archive-folder-label` | `$count` | Archive hierarchy: folder/dossier (plural-dispatched) |
| `term.archive-item-label` | `$count` | Archive hierarchy: item/piece (plural-dispatched) |
| `term.and`, `term.and-symbol`, `term.et-al`, `term.and-others` | none | Conjunctions |
| `term.accessed`, `term.retrieved`, `term.no-date`, `term.no-date-long`, `term.forthcoming`, `term.circa`, `term.circa-long` | none | Date and access labels |
| `role.editor.label`, `role.editor.label-long`, `role.editor.verb` | `$count`, optional `$gender` for labels | Use the two-selector pattern for gender-aware label nouns |
| `role.translator.label`, `role.translator.label-long`, `role.translator.verb` | `$count`, optional `$gender` for labels | Use the two-selector pattern for gender-aware label nouns |
| `role.guest.label`, `role.guest.label-long`, `role.guest.verb` | `$count` | |
| `pattern.page-range` | `$start`, `$end` | Spec'd; not yet consumed by the engine |
| `pattern.retrieved-from`, `pattern.available-at` | `$url` | Spec'd; not yet consumed by the engine |
| `pattern.date-full` | `$year`, `$month`, `$day` | Active. See "Date assembly" below. |
| `pattern.date-month-day` | `$month`, `$day` | Active. |
| `pattern.date-year-month`, `pattern.date-year-month-day`, `pattern.date-day-month-abbr-year`, `pattern.date-month-abbr-day-year` | as named | Active. See spec `LOCALE_MESSAGES.md` §2. |
| `pattern.date-range-<form>` | `$start`, `$end`, optional `$year` | Active for shared-year EDTF intervals. |
| `date.open-ended` | none | "present" / "heute" / "presente" |

> **Note on `pattern.*` messages.** No engine call site currently passes
> `$start` / `$end` / `$url` into the evaluator. These IDs are reserved by
> the spec and locales already author them, but consumers will land in a
> separate change. Author them in new locales for forward-compatibility,
> but don't expect them to render today.

The `legacy-term-aliases:` map bridges single-word legacy keys (e.g. `page`,
`et_al`, `editor`) to message IDs so styles authored against the v1 vocabulary
keep rendering. Mirror the en-US shape.

## Date assembly

The engine pre-formats `$year` / `$month` / `$day` from EDTF input, then looks
up a `pattern.date-<form>` message in the active locale. If a pattern is
authored, it is evaluated and used. If no pattern is authored, the engine
falls through to a hardcoded English assembly (`{month} {day}, {year}`).

When to author `pattern.date-*`:

- The locale wants a non-English component order (Spanish day-first, Basque
  year-first, …).
- The locale wants connector words ("de") or non-comma punctuation between
  components.
- The locale needs morphological suffixes on components (Basque genitive
  `-(r)en`, absolutive `-a`).

```yaml
# es-ES — day first, "de" connectors
messages:
  pattern.date-full:      "{$day} de {$month} de {$year}"
  pattern.date-month-day: "{$day} de {$month}"
```

```yaml
# eu-ES — Basque, year first with genitive month and absolutive day suffix
# (provisional — pending native-speaker review)
messages:
  pattern.date-full:      "{$year}ko {$month}ren {$day}a"
  pattern.date-month-day: "{$month}ren {$day}a"
```

Month inflection rides on `dates.months.long` / `dates.months.short`. Store
the citation-form month (e.g. `urtarrila`) and add case suffixes in the
pattern. If a locale ever needs more than one inflected form of the same
month, file a follow-up before extending — do not preempt.

A pattern that references a missing component (e.g. `pattern.date-full` uses
`{$day}` but the input has no day) returns `None`, and the engine falls back
to its English assembly. Author a separate `pattern.date-year-month` once
that ID is wired if you need a no-day form to stay locale-shaped.

### Shared-year date ranges

For a closed range whose endpoints share a year, locales can control the
connector and year placement with `pattern.date-range-<form>`. The engine
passes pre-formatted endpoint fragments as `$start` and `$end`; `$year` is
available when the selected date form normally displays a year.

```yaml
# es-ES
messages:
  pattern.date-range-year-month: "{$start} a {$end}, {$year}"
```

This renders a May-to-June 2026 interval as `mayo a junio, 2026`. Distinct-year
ranges remain separately formatted endpoints in this release.

### Sourcing minority-language grammar

For widely-resourced languages (English, Spanish, French, German, …) the
locale's morphology and date conventions are well documented and easy to
verify against a normative reference. For minority and lesser-resourced
languages, finding a citable shape is harder.

[Apertium](https://www.apertium.org) is the most useful starting point we've
found: it is a free/open-source rule-based machine translation platform that
maintains explicit morphological grammars for languages it supports, and its
wiki (`wiki.apertium.org`) contains relatively rigorous community-curated
notes on case marking, agreement, and date formation. The Basque locale
([`eu-ES.yaml`](../../crates/citum-schema-style/embedded/locales/eu-ES.yaml))
was bootstrapped from
[`wiki.apertium.org/wiki/Basque_to_English`](https://wiki.apertium.org/wiki/Basque_to_English)
for exactly this reason.

Treat Apertium notes as a **secondary source** — credible enough to be
better than invention, not authoritative enough to ship without a native
speaker confirming. Always:

1. Cite the specific wiki page (or other source) in the locale file header
   so reviewers can trace your derivation.
2. Mark the locale `PROVISIONAL` in the header until a native speaker has
   reviewed both the term content and any inflected patterns.
3. Prefer normative authorities (e.g. national language academies, major
   university-press style guides) when they exist and cover the question.

## Gender

`MaybeGendered<T>` is already live for locale term maps. The `roles:`,
`locators:`, and `terms:` fallback paths can store and resolve gendered values
for locales that need them, such as Spanish role nouns, French role nouns, and
Arabic gendered ordinals, using the requested gender from contributor data or
an explicit template override.

The MF2 path supports the scoped gender-aware role-label pattern:
`.match {$gender :select} {$count :plural}`. Use stable selector keys
`masculine`, `feminine`, `neuter`, and `common`, and include a full wildcard
fallback such as `when * *`.

For new or migrated locales:

- `messages:` is the preferred home for tested gender x plural role labels.
- Keep equivalent `roles:` values as fallback until deprecating that fallback is
  intentional and covered by tests.
- `resolved_role_term_neutral` passes `common` so mixed-gender contributor lists
  can resolve common MF2 variants.
- Gender-invariant role messages are safe in `messages:`.
- Gender-invariant locator and general terms are safe in `messages:`.

## Runtime locale selection

Styles can declare a default locale, but that is not the only intended source
of locale choice. `citum render refs --locale <locale-id>` is a user-facing
per-invocation override for choosing any available locale at render time; the
verification command below uses it as a smoke test because it exercises that
same runtime selection path. Persistent user default locale configuration is
tracked separately in the user style and locale store work (`csl26-erwz`).

## MF2 syntax cheat sheet

```yaml
# Static string
term.and: "and"

# Variable substitution
pattern.retrieved-from: "retrieved from {$url}"

# Plural dispatch (one / wildcard only)
term.page-label: |
  .match {$count :plural}
  when one {p.}
  when * {pp.}

# Select dispatch (arbitrary keys)
some.gendered: |
  .match {$gender :select}
  when masculine {él}
  when feminine {ella}
  when * {elle}

# Gender-aware role labels
role.editor.label-long: |
  .match {$gender :select} {$count :plural}
  when masculine one {editor}
  when masculine * {editores}
  when feminine one {editora}
  when feminine * {editoras}
  when common one {persona editora}
  when common * {equipo editorial}
  when * * {equipo editorial}
```

The current evaluator supports only `one` and `*` plural categories. Full
CLDR categories (`zero`, `two`, `few`, `many`) still require follow-up evaluator
work. The scoped multi-selector form for gender-aware role labels is active:
each `when` block must provide one key per selector, and two-selector messages
must include a full wildcard fallback such as `when * *`. ICU4X MF2 support is
tracked, but not treated as available in this branch.

## Verification

After editing a locale, run:

```bash
# 1. Schema check — ensures parse + evaluator setup is sound
cargo nextest run -p citum-engine -E 'test(locale) + test(spanish) + test(es_es) + test(german) + test(french)'

# 2. Portfolio gate — guards against accidental breakage of style fidelity
node scripts/report-core.js > /tmp/r.json && \
  node scripts/check-core-quality.js \
  --report /tmp/r.json \
  --baseline scripts/report-data/core-quality-baseline.json

# 3. Smoke test against a file-based style (sibling locales/ directory)
cargo run --bin citum -- render refs \
  -b tests/fixtures/references-expanded.json \
  -s styles/embedded/apa-7th.yaml --locale <your-locale>

# 4. Smoke test against a builtin-alias style (via the user store)
cargo run --bin citum -- locale add path/to/<your-locale>.yaml
cargo run --bin citum -- render refs \
  -b tests/fixtures/references-expanded.json \
  -s apa --locale <your-locale>
```

Step 4 installs the locale into the user data directory
(`~/.local/share/citum/locales/` on Linux, `~/Library/Application Support/citum/locales/`
on macOS) so the renderer can find it under any style — builtin or file-based.
The resolution order is: sibling locale directory (file-based styles only) → user
store → embedded.

`citum locale lint <file>` validates MF2 message structure before the render
path needs to evaluate a locale. `citum locale remove <id>` uninstalls.

## Related

- Spec: [docs/specs/LOCALE_MESSAGES.md](../specs/LOCALE_MESSAGES.md)
- Gender model: [docs/specs/GENDERED_LOCALE_TERMS.md](../specs/GENDERED_LOCALE_TERMS.md)
- ICU4X migration: bean `csl26-qrpo`
- Gender-aware MF2 role labels: bean `csl26-vm2g`
