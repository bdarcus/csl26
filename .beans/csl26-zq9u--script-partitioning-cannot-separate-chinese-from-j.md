---
# csl26-zq9u
title: Script partitioning cannot separate Chinese from Japanese
status: todo
type: feature
priority: normal
tags:
    - multilingual
    - sorting
    - architecture
created_at: 2026-07-26T15:01:20Z
updated_at: 2026-07-26T15:01:20Z
---

`sort-partitioning: { by: script }` derives its key from the characters of the author sort key (`script_partition_key` → `script_code_for_char`, crates/citum-engine/src/sort_partitioning.rs:150-166) and explicitly folds `Hant | Hans | Jpan | Kore | Hira | Kana` into a single `Hani` bucket. Chinese and Japanese therefore cannot be separated under script partitioning at all.

Domain-expert review (discussion #828) is clear that they must be: they are different languages with different writing systems, and GB/T 7714—2025 §9.3.2.1 requires grouping by Chinese / Japanese / Western / Russian / other. The Unicode Consortium's own guidance is that inferring language from Han characters is "largely impossible and the attempt basically meaningless" — which is exactly what character-derived script keys attempt.

## Current workaround (already applied)

styles/experimental/multilingual-partitioned.yaml now uses `by: language`, which reads the item's declared `language` and separates the two correctly. That is a genuine fix for the example, and arguably the right primitive, but it leaves `by: script` unable to express a requirement a national standard imposes.

## Decision needed

Should `by: script` resolve from the item's declared language via the ISO 15924 resolver landed in csl26-30ga (`zh` → Hans, `ja` → Jpan; see crates/citum-engine/src/values/mod.rs:438), falling back to character detection only when there is no language evidence — preserving the positive-evidence rule?

That would change partition keys from `Hani` to `Hans`/`Jpan` for items with language metadata, which is a visible behavior change for any style configuring `order`/`headings` with `Hani`. No in-repo style does (verified: zero styles declare multilingual at more than one scope, and only 6 have any multilingual block), so blast radius is limited to externally authored styles. Needs a call on whether to make the change and whether `Hani` should keep matching as a compatibility alias.

## Related

- Unlisted partition keys render with no heading, so their entries read as belonging to the section above (hit while fixing the example — every expected language must appear in `order` AND `headings`). Worth a lint or an explicit "other" bucket.
