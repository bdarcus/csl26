---
# csl26-kxhy
title: CJK personal names render with an inter-part space under GB/T
status: todo
type: bug
priority: normal
tags:
    - multilingual
    - rendering
    - gb-t
    - contributors
created_at: 2026-07-26T15:00:49Z
updated_at: 2026-07-26T15:00:49Z
---

A structured Chinese personal name (`family: 张` / `given: 伟`) renders as `张 伟` under the GB/T 7714—2025 styles. GB/T expects `张伟` — no inter-part space for CJK names.

Reproduce (citum 0.78.0 or main):

    references:
      - id: zhang
        class: serial-component
        type: article
        title: "环境司法制度改革对企业绿色创新的影响"
        author:
          - family: "张"
            given: "伟"
        language: "zh-CN"

    citum render refs -b x.yaml -s gb-t-7714-2025-numeric
    [1]张 伟. …        <- expected 张伟

The space is correct for the Western case in the same style (`Boobier T`), so the join needs to be script-aware.

## The configured mechanism does not reach it

`options.multilingual.scripts.Hani.delimiter` has no effect on this output. Tested with both `delimiter: ""` and `delimiter: "·"` plus `use-native-ordering: true` on a style extending gb-t-7714-2025-numeric — output is byte-identical either way. `Hani` is a valid candidate key (see `candidate_keys` in crates/citum-engine/src/values/contributor/names.rs:681), so either `script_configs` is not threaded into this style's name path or the delimiter is not consulted for the family/given join.

Investigate which, then decide whether the fix is engine-level (script-aware join) or a style-level `scripts` block on the GB/T base.

## Why it does not show in the corpus

The standard's own corpus writes Chinese personal names as literal whole names (`{"literal": "博伯尔"}`, 70 of them), so the pinned fixtures never exercise the structured path. Found while building demos for the citum-org news post (csl26-iooh), not by the oracle.
