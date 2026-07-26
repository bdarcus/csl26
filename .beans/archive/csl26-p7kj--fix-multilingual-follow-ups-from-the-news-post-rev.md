---
# csl26-p7kj
title: Fix multilingual follow-ups from the news-post review
status: completed
type: bug
priority: normal
tags:
    - multilingual
    - schema
    - docs
created_at: 2026-07-26T14:44:00Z
updated_at: 2026-07-26T15:01:41Z
---

Loose ends surfaced while drafting the citum-org multilingual follow-up post (csl26-iooh).

## Findings

1. **MultilingualConfig is replaced, not merged, on extends/scope merge.** Config::merge lists `multilingual` in the whole-value `merge_options!` macro, so any style that extends a parent and declares ANY multilingual key drops every inherited multilingual field. Reproduced: adding an unrelated `scripts.Hang` block to a style extending gb-t-7714-2025-numeric silently reverts Chinese punctuation from full-width to half-width, because the inherited `punctuation-width: mixed` is lost. Every other composite config in the same file (contributors, substitute, punctuation) already has a field-wise merge; multilingual is the outlier.

2. **YDX-2147483647's corrections to the multilingual examples were never applied** (discussion #828): examples/multilingual-refs.yaml splits 孔子 into family 孔 / given 子 (the name cannot be split) and romanizes 论语 with the wrong tone (Lùnyǔ, should be Lúnyǔ).

3. **styles/experimental/multilingual-partitioned.yaml collapses Chinese and Japanese** into one `Hani` section headed "Chinese & Japanese Sources".

## Todo

- [x] Add MultilingualConfig::merge and call it from Config::merge
- [x] Regression test for the inherited punctuation-width case
- [x] Fix the Confucius entry per YDX's guidance
- [x] Fix the partitioning example so Chinese and Japanese separate
- [x] File follow-up beans for what is out of scope
- [x] just pre-commit green

## Summary of Changes

**Engine fix.** Added `MultilingualConfig::merge` (field-wise) and a private `Config::merge_multilingual`, removing `multilingual` from the whole-value `merge_options!` list. `scripts` merges per key; `realization_default` and `term_locale` are not `Option`, and both are `skip_serializing_if`-elided at their default, so a default value is treated as "unset" and leaves the inherited value in place.

Verified end to end: a style extending `gb-t-7714-2025-numeric` with an unrelated `scripts.Hang` block previously rendered `北京: 清华大学出版社, 2023: 35.` (half-width) and now correctly renders `北京：清华大学出版社，2023：35.`. Shipped styles are byte-identical; an explicit `punctuation-width` override still wins.

Coverage: `given_partial_multilingual_overlay_when_merging_then_inherited_fields_survive` (rstest, 2 cases) and `given_multilingual_overlay_setting_a_field_when_merging_then_overlay_wins`.

**Examples.** `examples/multilingual-refs.yaml`: 孔子 is now a single `name:` whole-name carrying its own romanization and English form, instead of being split into `family: 孔` / `given: 子`; pinyin for 论语 corrected to `Lúnyǔ`. `styles/experimental/multilingual-partitioned.yaml`: switched to `by: language`, which separates Chinese and Japanese into their own sections (six sections verified), with a header comment explaining why script partitioning cannot.

`just pre-commit` green: 2197 tests, fmt + clippy clean. `just schema-gen` produces no diff — the change adds methods only, no public shape change.

## Follow-ups filed

- [[csl26-kxhy]] — CJK personal names render with an inter-part space under GB/T; `scripts.Hani.delimiter` does not reach the name join.
- [[csl26-53ek]] — `name-mode: transliterated` is not applied to whole-name (`SimpleName`) contributors, so the schema-correct way to model 孔子 loses romanization.
- [[csl26-zq9u]] — `by: script` partitioning folds all CJK into `Hani` and cannot separate Chinese from Japanese; needs a decision on resolving from language metadata.
