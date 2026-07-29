---
# csl26-e5xl
title: ParentMonograph title dispatch ignores ref-type
status: todo
type: bug
priority: normal
created_at: 2026-07-29T18:00:47Z
updated_at: 2026-07-29T18:00:47Z
---

TitleType::ParentMonograph in crates/citum-engine/src/render/component.rs (get_title_category_title_rendering) always resolves via titles_config.container_monograph.or(titles_config.monograph), regardless of the reference's type. Unlike ContainerTitle, it never consults container_title_category(ref_type), so a style whose monograph config sets emph:true applies that emphasis uniformly to every parent-monograph title -- including entry-dictionary and entry-encyclopedia containers, which citeproc-js does not italicize (confirmed via oracle for american-society-of-mechanical-engineers: Merriam-Webster.com dictionary and Encyclopedia of World History). Worked around at the wrapper level in ASME's csl26-svfg fix (explicit emph:false on entry-dictionary/entry-encyclopedia type-variants) but the dispatch itself should consult ref-type classification the way ContainerTitle does, so other styles inheriting a monograph.emph config don't need the same per-wrapper workaround. See docs/specs/STYLE_INHERITANCE.md wrapper-compat audit.
