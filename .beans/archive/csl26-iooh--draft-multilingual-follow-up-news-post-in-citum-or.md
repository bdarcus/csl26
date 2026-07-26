---
# csl26-iooh
title: Draft multilingual follow-up news post in citum-org
status: completed
type: task
priority: normal
tags:
    - docs
    - multilingual
    - gb-t
created_at: 2026-07-26T14:25:47Z
updated_at: 2026-07-26T14:34:05Z
---

Follow-up to citum-org docs/news/posts/multilingual-design-review.md, demoing what shipped from epic csl26-0ugp (multilingual architecture hardening) plus the GB/T 7714-2025 styles and date.note.

Deliverable: draft PR against citum/citum-org adding docs/news/posts/multilingual-shipped.md. No citum-core changes.

Plan: /home/bruce/.claude/plans/you-will-create-a-warm-reddy.md

## Todo

- [x] Re-run all four demos against citum 0.78.0, capture real output
- [x] Write docs/news/posts/multilingual-shipped.md
- [x] Verify snippets round-trip (copy out of post, run fresh)
- [x] Re-check live gb7714-bench figure
- [x] Build site locally, review rendered page
- [x] Verify all external links resolve
- [x] Open draft PR on citum-org

## Summary of Changes

Draft PR citum/citum-org#27 adds `docs/news/posts/multilingual-shipped.md` (single file; generated HTML stays gitignored).

Demos four features, all in released v0.78.0: GB/T 7714—2025 across three styles and three languages; punctuation realization via a one-line `punctuation-width: bylan` override; per-item term locale (de/fr/en role labels from one template); and `date.note` (same data rendered by `apa-7th` vs `gb-t-7714-2025-numeric`).

Verification: all five YAML snippets were re-extracted from the finished Markdown, re-run, and diffed against the outputs the post claims — 8/8 blocks matched exactly. Chinese/Japanese content taken verbatim from the pinned GB/T corpus. All 21 external links plus the install URL return 200. Site builds clean.

Before/after comparison blocks use a scoped `<style>` at the end of the post (positioned last so `.prose > *:first-child` still applies to the opening paragraph), using only existing `theme.css` variables. PR body offers to move them into `theme.css` if reuse is wanted.

## Follow-ups not filed

- YDX's corrections to the *first* post's examples are still unfixed in citum-core: `examples/multilingual-refs.yaml` splits 孔子 as family/given and has the wrong pinyin tone; `styles/experimental/multilingual-partitioned.yaml` still collapses Chinese and Japanese into one `Hani` partition. The post admits this openly.
- Possible engine gap found while building demos, deliberately excluded from the post as unverified: a structured Chinese name (`family` + `given`) renders with a space under the GB/T styles; the standard's own corpus uses literal `name:` forms, so it does not surface in the demos.
