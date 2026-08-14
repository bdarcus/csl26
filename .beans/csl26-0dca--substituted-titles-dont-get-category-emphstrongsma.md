---
# csl26-0dca
title: Substituted titles don't get category emph/strong/small-caps rendering
status: todo
type: bug
priority: normal
tags:
    - title
    - substitute
    - rendering
    - engine
created_at: 2026-08-14T12:30:52Z
updated_at: 2026-08-14T12:31:01Z
---

When a style's substitute chain promotes a title into the author slot (e.g. APA's author -> editor -> title fallback for author-less references), the substituted title bypasses TemplateTitle's normal rendering entirely -- it flows through resolve_title_substitute() in crates/citum-engine/src/values/contributor/substitute.rs, not through the title category's TitleRendering (emph/strong/small_caps/vertical_align).

Repro: apa-7th sets titles.monograph.emph: true. An author-less monograph reference renders its title unitalicized in the substitute slot:

    citum render refs -b refs.json -s apa-7th -m bib --json
    # refs.json: {"id":"z","class":"monograph","type":"book","title":"Some Book Title","issued":{"date-parts":[[2020]]}}
    # => "text": "Some Book Title. (2020)"   (APA italicizes monograph titles; should be emphasized)

Compare: the same title WITH an author present correctly gets wrapped via component.rs's rendering.emph application (confirmed via the underscore-wrapped plain-text render: "_Library of Congress and _more__").

Discovered while fixing csl26-d3kj (Djot markup leaking through this same substitute path -- that fix did NOT touch category-level rendering flags, only Djot inline rendering/case/quotes). resolve_title_substitute() would need to consult the title category's TitleRendering (via the same get_title_category_title_rendering used by resolve_effective_title_rendering in title.rs) and apply emph/strong/small-caps through the OutputFormat, mirroring what component.rs does for the normal (non-substitute) title path.

Affects any style with per-category title emphasis (emph/strong/small_caps/vertical_align) applied to a title category that can also be author-substituted -- at minimum apa-7th's monograph/periodical categories.
