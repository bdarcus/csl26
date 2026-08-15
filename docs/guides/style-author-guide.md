# Style Author Guide

This guide is for people who write and maintain Citum styles.
Prefer working with an AI agent? See the [AI-assisted authoring guide](style-authoring-skill.html) — the `citum/skills` package handles the workflow for you.

## [compare_arrows] How Citum Differs

Citum introduces a modern, declarative approach to citation styling compared to CSL 1.0's procedural XML language. Understanding these differences is essential for writing Citum styles effectively.

| Aspect | CSL 1.0 (XML) | Citum (YAML) |
|---|---|---|
| Format | Procedural XML markup | Declarative YAML |
| Logic | `choose`/`if`/`else` conditionals | Type variants + inheritance |
| Name Formatting | Inline XML attributes | Global presets + options |
| Dates | Object with year/month/day | EDTF string format |
| Inheritance | Parent style aliasing | Extends + scoped options |
| Readability | Verbose and nested | Concise and explicit |

> [!TIP]
> **Explicit Over Magic**
> Citum styles are explicitly declarative. Special behavior is expressed in the style itself, not hidden in processor logic. If you need a different layout for journals vs books, you declare it with type variants. This makes styles portable, testable, and understandable without reading source code.

## [folder_special] Style Anatomy

Every Citum style file contains four top-level sections: metadata, options, citation template, and bibliography template.

### Minimal Style Skeleton

```yaml
# yaml-language-server: $schema=https://citum.github.io/citum-core/schemas/style.json

info:
  title: "My Style Name"
  id: "my-style"
  description: "Optional short description"
  default-locale: "en-US"

options:
  processing: author-date

citation:
  template:
    - contributor: author
    - date: issued

bibliography:
  template:
    - contributor: author
    - date: issued
    - title: primary
```

> [!TIP]
> **Editor autocomplete and validation**
> The `yaml-language-server` comment in the skeleton above enables full autocomplete and inline validation in editors that support the [YAML Language Server protocol](https://github.com/redhat-developer/yaml-language-server).

### Info Fields

- `title`: Human-readable name (e.g., "American Psychological Association 7th Edition").
- `id`: Unique identifier in kebab-case (e.g., "apa-7th").
- `default-locale`: BCP 47 language tag (e.g., "en-US", "de-DE").
- `fields`: Discipline categories (e.g., anthropology, biology, history).

## [tune] Global Options

Global options control the processing mode and apply defaults to all components in both citation and bibliography templates.

### Processing Modes

| Mode | Description | Example |
|---|---|---|
| `author-date` | Author+year/page citations | (Smith, 2020) |
| `numeric` | Numbered citations | [1] |
| `note` | Footnote-based citations | Smith, "Title," 2020 |
| `label` | Alphabetic or numeric keys | [Kuh62] |

### Contributor Presets

Named presets control name formatting without spelling out every field:

- `apa`: Family-first, "&" symbol, initials with period-space.
- `chicago`: Family-first, "and" text, full names.
- `vancouver`: All family-first, no conjunction, compact initials.
- `ieee`: Given-first, "and" text, initials with period-space.
- `harvard`: All family-first, "and" text, compact initials.
- `springer`: All family-first, no conjunction, compact initials.

### Date Presets

| Preset | Format | Example |
|---|---|---|
| `long` | Full month names, EDTF markers | January 15, 2024 |
| `short` | Abbreviated month names | Jan 15, 2024 |
| `numeric` | Numeric months | 1/15/2024 |
| `iso` | ISO 8601, no EDTF markers | 2024-01-15 |

### Name Memory

Two independent memory features track how author names are displayed across a document, reducing repetition after first mention.

#### `integral-name-memory` (personal names)

Tracks structured personal authors (given + family name). On first integral citation the full name is shown; on subsequent citations only the family name appears. Applies to `StructuredName` and `Multilingual` contributors — `SimpleName` (organisations) is excluded.

```yaml
options:
  integral-name-memory:
    scope: document   # document | chapter | section
    contexts: body-only  # body-only | body-and-notes
```

Tracking key is `given|family|suffix`, so two authors who share a surname (e.g. "Jane Smith" and "John Smith") are tracked independently and each receives "First" on their debut.

#### `org-abbreviation-memory` (organisation short names)

Tracks `SimpleName` contributors that carry a `short-name` field. **Off by default** — the feature activates only when `org-abbreviation-memory` is present in style options or document frontmatter.

```yaml
options:
  org-abbreviation-memory:
    scope: document
    contexts: body-only
    display: full-then-parenthetical  # see variants below
```

| `display` variant | First mention | Subsequent |
|---|---|---|
| `full-then-parenthetical` (default) | `World Health Organization (WHO)` | `WHO` |
| `full-then-bracketed` | `World Health Organization [WHO]` | `WHO` |
| `short-then-parenthetical` | `WHO (World Health Organization)` | `WHO` |
| `short-then-bracketed` | `WHO [World Health Organization]` | `WHO` |

#### Per-document overrides

Both name-memory features can be overridden per-document in frontmatter without changing the style file:

```yaml
---
options:
  integral-name-memory:
    scope: section
  org-abbreviation-memory:
    display: full-then-bracketed
---
```

#### `bibliography.repeated-author-rendering`

The 3-em-dash convention — replacing a consecutive repeated author group in the bibliography with `———` — is controlled via this field. CMOS §14.67 calls it a *publisher's prerogative*; standard Chicago styles intentionally omit it by default (matching CSL 18th-edition behaviour).

Values: `full` (always print the full name), `dash`, `dash-with-space`.

**Opt in per document** (e.g. for final publisher output):

```yaml
---
options:
  bibliography:
    repeated-author-rendering: dash
---
```

| Without override (`full`) | With `dash` |
|---|---|
| `Chen, Mei. 2017. The Social Life of References…` | `Chen, Mei. 2017. The Social Life of References…` |
| `Chen, Mei. 2020. Citation and Authority…` | `———. 2020. Citation and Authority…` |

**Suppress in a house style that bakes it in** (e.g. a 17th-edition–derived style):

```yaml
---
options:
  bibliography:
    repeated-author-rendering: full
---
```

See `docs/specs/PER_DOCUMENT_CONFIG_OVERRIDES.md` for the full eligible-option set.

## [layers] Template Components

### Contributor
Renders author, editor, translator, and other contributors.

```yaml
- contributor: author
  form: long          # long | short | verb | verb-short
  name-order: family-first  # family-first | given-first
```

### Date
Renders date fields using EDTF format.

```yaml
- date: issued
  form: year  # year | year-month | full | month-day | year-month-day
```

### Title
Renders the title of the item.

```yaml
- title: primary
  form: long  # long | short
```

### Number
Renders numeric data: volume, issue, pages, edition, etc.

```yaml
- number: pages
  form: numeric  # numeric | ordinal | roman
```

## [translate] Gender-Aware Locale Terms

Citum locale terms can now vary by grammatical gender when the language requires it.

### Reference Data

Contributor-driven role labels use an explicit `gender` field on contributor entries:

```yaml
contributors:
  - role: editor
    contributor:
      family: "Martinez"
      given: "Ana"
    gender: feminine
```

Mixed-gender contributor groups prefer a locale's neutral/common form when one exists. If a locale only provides gendered masculine/feminine variants and no neutral/common form, Citum does not silently fall back to a masculine-specific label for the mixed group.

### Template Overrides

Use a template-level `gender` override when a term or number label must request a specific agreement form directly:

```yaml
- contributor: editor
  form: long
  gender: feminine

- term: volume
  form: short
  gender: masculine

- number: volume
  label-form: short
  gender: feminine
```

### Locale YAML

Locale terms still accept plain strings, but they can now also use gendered maps:

```yaml
roles:
  editor:
    long:
      singular:
        masculine: editor
        feminine: editora
        common: persona editora
      plural:
        masculine: editores
        feminine: editoras
        common: equipo editorial
```

Locator terms can also declare lexical gender metadata for noun agreement:

```yaml
locators:
  page:
    long:
      singular: página
      plural: páginas
    short:
      singular: p.
      plural: pp.
    gender: feminine
```

> [!TIP]
> **Current scope**
> Gender-aware rendering currently applies to locale term selection and contributor role labels. Verb-form role terms such as `edited by` remain ungendered in this release, even though they share the same underlying Rust type.

## [formatting] Rendering Options

Every component can be modified with rendering options that control punctuation, formatting, and text wrapping.

> [!WARNING]
> **Avoid locale-specific strings**
> Do not hardcode text content like `"In: "`, `"Editor"`, or `"pp. "` in `prefix` or `suffix`. Always use the `term` component for localized text. Reserve prefix and suffix for punctuation and spacing.

| Option | Values | Purpose |
|---|---|---|
| `prefix` | `" "`, `", "`, `". "` | Text before the component |
| `suffix` | `".", ",", ": "` | Text after the component |
| `wrap` | `parentheses`, `brackets`, `quotes` | Automatically wrap value |
| `emph` | `true`, `false` | Render in italics |
| `strong` | `true`, `false` | Render in bold |
| `quote` | `true`, `false` | Wrap in the locale's quotation marks (title components only) |

These apply to one template component at a time. Titles usually need the
same rendering repeated across many components and reference types — see
[Title Categories](#title-categories) below for the type-driven alternative.

## [format_quote] Title Categories

Instead of setting `emph`/`quote`/`text-case` on every `title:` component
individually, group titles into a small vocabulary of rendering categories
and configure each category once under `titles:`. This is how most real
styles work: a journal article title renders one way everywhere it appears
in the document, a book title another way, regardless of which template
component happens to be rendering it.

```yaml
options:
  titles:
    type-mapping:
      thesis: monograph
      graphic: monograph
    component:
      quote: true
    monograph:
      emph: true
```

**Categories** (a closed vocabulary — anything else is a schema error):

| Category | Typical reference types | Meaning |
|---|---|---|
| `monograph` | book, thesis, report | a standalone title |
| `component` | article, chapter, entry | a title contained in something larger |
| `container-monograph` | (the book a chapter is in) | the container's own title |
| `periodical` | journal, magazine, newspaper | a periodical's name |
| `serial` | a series | a series name |
| `default` | everything else | fallback when no other category applies |

Every reference type already has a **built-in default category**
(`article-journal` and similar → `component`; `book`/`thesis`/`report` →
`monograph`; anything else → `default`), so most styles never need
`type-mapping` at all — only add an entry to override that default for a
specific type. `elsevier-harvard`, for example, maps `graphic`/
`motion-picture`/`song` to `monograph` because that style's source rule
italicizes those types too, and maps `thesis` to `default` because it
specifically does *not* want the built-in book/thesis/report grouping.

Each category accepts the same fields as a title component's rendering
options (`emph`, `strong`, `quote`, `text-case`), plus `locale-overrides`
for per-language variants. See the
[style schema](https://citum.github.io/citum-core/schemas/style.json) for
the full field list, and
[`TYPED_TITLE_MAPPING.md`](../specs/TYPED_TITLE_MAPPING.md) for the
underlying rules.

> [!WARNING]
> **An unconfigured category renders plain, not an error.**
> If a reference type resolves to a category you never configured (or one
> you configured with no `emph`/`quote`), Citum renders that title as plain
> text — no italics, no quotes, no warning. That's intentional (some styles
> genuinely want plain titles for some types), but it means a typo or an
> unmapped type fails silently. Give every category your style can reach an
> explicit rendering — even "nothing" — rather than leaving it to omission.
> This matters even more once contributor substitution is involved; see
> the warning under Author-less References below.

## [swap_horiz] Author-less References (Substitution)

When a reference has no author, `substitute:` tells Citum what to promote
into the author position instead — an editor, a translator, or, as a last
resort, the reference's own title:

```yaml
options:
  substitute:
    template:
      - editor
      - translator
      - title
```

Citum tries each candidate in order and stops at the first one the
reference actually has. `title` is special: a title is being promoted into
a *name* slot, so Citum has to decide how to format it, and that's
controlled by `title-quote`:

| `title-quote` | Behavior | When to use |
|---|---|---|
| `always` (default, or omitted) | Quote the title, regardless of type | matches legacy/historical behavior; the safe default |
| `by-category` | Defer to the title's own [category rendering](#title-categories) — italicize a book, quote an article, exactly as if it had rendered normally | most real-world citation styles do this |

```yaml
citation:
  options:
    substitute:
      template: [editor, translator, title]
      title-quote: by-category
    titles:
      type-mapping:
        book: monograph
        report: monograph
      monograph:
        emph: true
      component:
        quote: true
      default:
        quote: true
```

> [!TIP]
> **Scope `title-quote: by-category` to `citation.options`**
> A style's bibliography never quotes a substituted title — it always
> applies category emphasis instead, unconditionally. Only citation-context
> rendering needs `title-quote` at all, so put the supporting `titles:`
> config under `citation.options.titles` too, unless you specifically want
> it to also change how titles render outside the substitute path.

> [!WARNING]
> **Cover every type your substitute chain can reach, not just the common ones**
> `by-category` makes *quoting itself* — not just emphasis — depend on
> category coverage. A reference type that falls through to an unconfigured
> category renders **plain, un-quoted and un-italicized** — worse than the
> `always` default for that type. Before setting `title-quote:
> by-category`, list every reference type an author-less reference in your
> style could plausibly have, and confirm each one resolves to a category
> with an explicit `quote` or `emph`. See
> [`SUBSTITUTED_VALUE_FORMATTING.md`](../specs/SUBSTITUTED_VALUE_FORMATTING.md)
> for the corpus evidence behind this default and a worked example
> (`elsevier-harvard`).

## [auto_awesome] Style Inheritance

Inherit from a named base style using `extends:`. The base style supplies all
templates; the inheriting style can only set metadata and normal typed options
(see below).

```yaml
extends: springer-basic-author-date-core
```

`extends:` also accepts a URI (`file://…`, `https://…`, `git+https://…`, or
`cid:bafkrei…`) for parents that live outside the embedded builtin set. To
lock the parent to a specific version, add a sibling `extends-pin:` whose
value is the parent's CID:

```yaml
extends: https://hub.citum.org/styles/apa-7th.yaml
extends-pin: cid:bafkreicpx6nc4rll4eahyfid2nbxjjli65tf2vjjed75xtl2ymjtjref44
```

Generate a paste-ready pair with `citum style pin <name|path>`. The full
distributed-registry workflow lives in
[DISTRIBUTED_REGISTRIES.md](DISTRIBUTED_REGISTRIES.md).

## [tune] Scoped Options

When a style uses `extends:`, it tunes behaviour through the same scoped option
blocks used by standalone styles.

Use the option block that matches the scope of the behavior:

- `options.*` for style-wide configuration such as contributor presets
- `citation.options.*` for citation-only behavior
- `bibliography.options.*` for bibliography-only behavior

**Allowed values for the common scoped fields**

| Field | Allowed values | Use when | Example value |
|---|---|---|---|
| `bibliography.options.date-position` | `after-author`, `after-title`, `terminal` | the style should move the year within bibliography entries | `after-author` |
| `options.contributors` | contributor presets such as `apa`, `chicago`, `springer`, `vancouver` | the style should switch contributor formatting | `springer` |
| `citation.options.label-mode` | `none`, `numeric`, `alphabetic` | the style should generate or suppress a reference marker (`[1]`, `[Kuh62]`) | `numeric` |
| `citation.options.label-wrap` | `none`, `parentheses`, `brackets`, `superscript` | punctuation should wrap the marker **alone** (AMA's `[1](p737)`) | `brackets` |
| `citation.options.item-wrap` | `none`, `parentheses`, `brackets`, `superscript` | punctuation should wrap the marker **and the item body** (IEEE's `[1, p. 737]`) | `brackets` |
| `bibliography.options.label-mode` | `none`, `numeric`, `alphabetic`, `author-date` | the style should change bibliography marker display | `numeric` |
| `bibliography.options.label-separator` | any string | a gap should sit between marker and entry body; empty (the default) renders flush | `' '` |

Reference markers are processor-owned: declare `label-mode` rather than writing
a `number: citation-number` or `number: citation-label` component, which are not
template components and are rejected. See
[REFERENCE_MARKERS](../specs/REFERENCE_MARKERS.md).

```yaml
# Standalone style: configure the style directly
options:
  contributors: springer
  dates: short
```

When a style uses `extends:`, treat the file as an override layer. Any option you
do not restate continues to come from the base style, so the wrapper usually
shows only the behavior it wants to change.

```yaml
# Wrapper style: configure the inherited base through normal scoped options
# Omitted options still come from `springer-basic-author-date-core`
extends: springer-basic-author-date-core
options:
  # Override only contributor formatting; other top-level options are inherited
  contributors: springer
bibliography:
  options:
    # Override only bibliography date placement; labels/templates still come from the base
    date-position: after-author
```

```yaml
# Omitted citation/bibliography options still come from `elsevier-vancouver-core`
extends: elsevier-vancouver-core
citation:
  options:
    # Override citation label punctuation only
    label-wrap: brackets
bibliography:
  options:
    # Override bibliography label mode only
    label-mode: numeric
```

> [!TIP]
> **Beginner rule**
>
> Ask this first:
>
> - does this file use `extends:`?
>   - yes: use the same `options.*`, `citation.options.*`, and `bibliography.options.*` blocks you would use anywhere else
>   - no: use those same blocks directly
>
> If you need to change templates or `type-variants`, you are no longer making
> a profile. You need a new base style or an independent style.

## [category] Type Variants

When reference types need a different layout, use `type-variants`. Citum
supports two forms:

- **Full variants** replace the default template for a reference type.
- **Diff variants** start from another template and declare only the structural
  changes.

Prefer a diff variant when the entry is a small delta from `default`,
`chapter`, `article-journal`, or another nearby variant. Use a full variant
when the reference type has a materially different structure, such as a legal
case, statute, patent, or another entry that does not share stable anchors with
the default template.

### Full Variants

A full variant is a complete template for that reference type. If a reference
type matches a full `type-variants` entry, that template is used instead of the
default template.

```yaml
bibliography:
  template:
    - contributor: author
    - title: primary
  type-variants:
    article-journal:
      - contributor: author
      - title: primary
      - title: parent-serial
        emph: true
```

> [!TIP]
> **Use full variants sparingly**
> Full variants are easiest to read when the structure is truly different, but
> they duplicate the parent template. If only a component changes punctuation,
> label form, emphasis, or placement, use a diff variant instead.

### Diff Variants

A diff variant is an object with any of these keys:

| Key | Purpose |
|---|---|
| `extends` | Optional parent variant in the same section. If omitted, the default `template` is the parent. |
| `modify` | Change rendering fields or supported component options on a matched inherited component. |
| `remove` | Delete a matched inherited component. |
| `add` | Insert a new component before or after a matched inherited component. |

```yaml
bibliography:
  template:
    - contributor: author
    - date: issued
      form: year
      wrap: parentheses
      prefix: " "
    - title: primary
    - title: parent-monograph
      prefix: " "
    - variable: publisher
      prefix: ". "
    - number: pages
      prefix: ", "

  type-variants:
    chapter:
      modify:
        - match:
            number: pages
          label-form: short
      add:
        - before:
            number: pages
          component:
            term: volume
            form: short
            prefix: ", "
            suffix: " "
      remove:
        - match:
            variable: publisher
```

The `match`, `before`, and `after` selectors are partial component matches. A
selector such as `{ number: pages }` matches a component with `number: pages`
even if that component also has `prefix`, `suffix`, or `label-form`.

Use selectors that identify one inherited component. Ambiguous selectors make a
variant fragile, especially in templates that contain several titles,
contributors, or dates.

### Extending Another Variant

Inside `type-variants`, `extends` means "start from this other variant in the
same section." It is variant-local structural reuse. It is different from:

- top-level `extends`, which inherits a whole style from another style.
- section-level `template-ref`, which reuses a named citation or bibliography
  template preset.

```yaml
bibliography:
  template:
    - contributor: author
    - title: primary
    - title: parent-monograph
    - number: pages

  type-variants:
    chapter:
      modify:
        - match:
            number: pages
          prefix: ", "
          label-form: short

    paper-conference:
      extends: chapter
      add:
        - after:
            title: primary
          component:
            title: parent-serial
            emph: true
            prefix: ". "
```

In this example, `paper-conference` first inherits the `chapter` page-label
change, then inserts the conference proceedings title.

### Operation Examples

Change affixes or formatting on an inherited component:

```yaml
type-variants:
  article-journal:
    modify:
      - match:
          title: parent-serial
        emph: true
        prefix: ". "
```

Use localized labels for pages. Do not write `prefix: "pp. "`.

```yaml
type-variants:
  chapter:
    modify:
      - match:
          number: pages
        prefix: ", "
        label-form: short
```

Remove an inherited component:

```yaml
type-variants:
  article-journal:
    remove:
      - match:
          variable: publisher
```

Insert a component before or after a matched component:

```yaml
type-variants:
  chapter:
    add:
      - before:
          number: pages
        component:
          title: parent-monograph
          emph: true
          suffix: ", "

  webpage:
    add:
      - after:
          title: primary
        component:
          variable: url
          prefix: ". "
```

Diff operations are resolved in the order written within each operation list.
The key order of `modify`, `remove`, and `add` does not matter.

## [article] Citation Modes

Use different citation templates for narrative vs. parenthetical citations, shortened forms, and special cases like ibid.

### Integral vs Non-Integral Citations

Use `integral:` and `non-integral:` blocks for narrative ("Smith (2020) argued...") vs parenthetical ("...was argued (Smith, 2020)") styles:

```yaml
citation:
  wrap: parentheses       # default wrapping
  template:               # used as fallback if no mode-specific block
    - contributor: author
    - date: issued
  non-integral:           # (Smith, 2020)
    wrap: parentheses
    template:
      - contributor: author
        form: short
      - date: issued
        form: year
  integral:               # Smith (2020)
    delimiter: " "
    template:
      - contributor: author
        form: short
      - date: issued
        form: year
        wrap: parentheses
```

### Note-style: Subsequent and Ibid

Use `subsequent:` for shortened second-and-later citations, and `ibid:` for same-source repetitions:

```yaml
citation:
  template:               # Full first citation
    - contributor: author
      form: long
    - title: primary
      prefix: ", "
    - variable: locator
      prefix: ", "
  subsequent:             # Short form for later citations
    options:
      contributors:
        name-form: family-only
    template:
      - contributor: author
        form: short
      - title: primary
        form: short
      - variable: locator
        prefix: ", "
  ibid:                    # Same source, possibly different locator
    suffix: "Ibid."
    template:
      - variable: locator
        prefix: ", "
```

### Multi-cite and Collapse

For numeric styles, use `multi-cite-delimiter` (default "; ") to separate multiple citations, and `collapse: citation-number` to render ranges like [1–3]:

- `multi-cite-delimiter`: String to separate multiple citations.
- `collapse: citation-number`: Consecutive numbers are collapsed into ranges.

## [share] Options Inheritance

Global options apply to all components, but can be overridden at the citation and bibliography level.

```yaml
options:
  contributors: apa       # Global: family-first, initials, up to 20 names

citation:
  options:
    contributors:
      shorten: { min: 3, use-first: 1 }   # Citation-level override

bibliography:
  options:
    contributors:
      shorten: { min: 20, use-first: 19 }  # Bibliography-level override
```

### Inheritance Chain

1. Component-level options (highest priority)
2. Citation/Bibliography-level options
3. Global options (lowest priority)

## [sort] Bib Sort & Groups

Control bibliography ordering and split it into labeled sections based on reference properties.

### Sort

The `bibliography.sort` field controls ordering. Use a preset string or a custom sort template:

```yaml
bibliography:
  sort: author-date-title   # preset: sort by author, then date, then title
```

**Preset sort values:** `author-date-title`, `author-title-date`.

### Groups

The `bibliography.groups` field splits the bibliography into labeled sections:

```yaml
bibliography:
  groups:
    - id: primary
      heading:
        localized:
          en-US: "Primary Sources"
      selector:
        type: legal-case
    - id: other
      heading:
        localized:
          en-US: "Secondary Sources"
      selector:
        not:
          type: legal-case
```

## [list] Reference Types

References use a `class` (top-level discriminator) and `type` (subtype). In styles, use the `type` value as keys under `type-variants`.

| Class | Types |
|---|---|
| Monograph | `book`, `manual`, `report`, `thesis`, `webpage`, `post`, `interview`, `manuscript`, `document` |
| Collection | `anthology`, `proceedings`, `edited-book`, `edited-volume` |
| Component | `chapter`, `article-journal`, `article-magazine`, `article-newspaper`, `broadcast`, `post` |
| Standalone | `legal-case`, `statute`, `treaty`, `hearing`, `regulation`, `brief`, `patent`, `dataset`, `standard`, `software` |

> [!TIP]
> **Using Type Values in type-variants**
> - Use the `type` value (e.g., `article-journal`, `book`) in `type-variants:`.
> - The special keyword `default` also works.
> - There is no wildcard selector. A type with no entry renders the section
>   `template:`, which is also the implicit parent of every variant that omits
>   `extends`.
> - For components, parents are embedded under the `parent:` key.

## [code] Complete Examples

### Example 1: Minimal Author-Date Style

```yaml
info:
  title: "Simple Author-Date"
  id: "simple-author-date"

options:
  processing: author-date
  contributors: apa

citation:
  template:
    - contributor: author
    - date: issued
      prefix: " "
      wrap: parentheses

bibliography:
  template:
    - contributor: author
    - date: issued
      prefix: " "
    - title: primary
      prefix: " "
      suffix: "."
```

### Example 2: Minimal Numeric Style

```yaml
info:
  title: "Simple Numeric"
  id: "simple-numeric"

options:
  processing: numeric

citation:
  options:
    label-mode: numeric
    label-wrap: brackets
  collapse: citation-number
  template:
    - variable: locator
      prefix: ", "

bibliography:
  options:
    label-mode: numeric
    label-wrap: brackets
  template:
    - contributor: author
    - title: primary
```

For an integral numeric citation, the generated label follows the authored
narrative content:

```yaml
citation:
  options:
    label-mode: numeric
    label-wrap: brackets
  integral:
    delimiter: " "
    template:
      - contributor: author
        form: short
```

### Example 3: Style Inheritance with Scoped Options

Inherit from a shared base and tune it via normal scoped options. No templates
needed — the base supplies them.

```yaml
info:
  title: Springer - Basic (author-date)
  description: >-
    Springer Author Date Style for Medicine, Life Sciences,
    Chemistry, Geosciences, Computer Science, and Engineering.

extends: springer-basic-author-date-core
options:
  contributors: springer
bibliography:
  options:
    date-position: after-author
```

```yaml
info:
  title: Elsevier - NLM/Vancouver (citation-sequence)
  description: A style for some Elsevier journals, resembles Vancouver style.

extends: elsevier-vancouver-core
citation:
  options:
    label-wrap: brackets
bibliography:
  options:
    label-mode: numeric
```

## [tune] Document-Level Options

Document-level options control rendering behavior that belongs to the document rather than the style. They are passed at render time and do not modify the style itself.

### Abbreviation Map

The `abbreviation-map` document option substitutes full rendered strings with abbreviations before output. It accepts both `abbreviation-map` (YAML frontmatter) and `abbreviation_map` (JSON API) as the key name.

```yaml
abbreviation-map:
  Estates Gazette: EG
  "Lloyd's Law Reports": "Lloyd's Rep"
  "World Health Organization": WHO
```

Keys are full rendered strings (exact, case-sensitive). The map applies to:

- Title fields (main title, container title, collection title)
- Variable fields (publisher, archive, series)
- Contributor literal names (corporate/institutional authors)

Abbreviations are applied after value extraction and before output assembly. The style has no knowledge of the map — it is purely a document-level transform.

## [lightbulb] Workflow & Tips

### Recommended Workflow

1. **Start with a reference style**: Use an existing style as a template.
2. **Write metadata**: Set title, id, and default locale in `info`.
3. **Define global options**: Set mode and contributor/date presets.
4. **Write citation template**: Start with author, date, and title.
5. **Test with oracle**: Compare output against reference implementation.
6. **Add type-variants**: Only for types needing a structurally different template.

### Common Mistakes to Avoid

> [!WARNING]
> **Over-using type-variants**
> Only add `type-variants` for types that need a genuinely different component set. Use presets and a well-designed generic template for the common case.

> [!WARNING]
> **Over-complicated options inheritance**
> Keep global options simple. Override only at citation/bibliography level when truly needed.

> [!WARNING]
> **Mismatching prefix/suffix pairs**
> Always pair opening prefix with closing suffix. For structural wrapping (e.g. parentheses), use the `wrap` option instead.
