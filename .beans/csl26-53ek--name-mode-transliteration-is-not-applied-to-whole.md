---
# csl26-53ek
title: name-mode transliteration is not applied to whole-name (SimpleName) contributors
status: todo
type: bug
priority: normal
tags:
    - multilingual
    - rendering
    - contributors
created_at: 2026-07-26T15:01:20Z
updated_at: 2026-07-26T15:01:20Z
---

A contributor written as an indivisible whole name (`name:`) accepts the full multilingual form, but `options.multilingual.name-mode: transliterated` does not apply to it — the original script renders instead of the romanization.

Reproduce with examples/multilingual-refs.yaml (the corrected Confucius entry) and styles/iso690-numeric.yaml, which sets `name-mode: transliterated`:

    [7] 孔子. Lúnyǔ [Analects of Confucius].: 人民文学出版社.
         ^^^ expected Kǒngzǐ under a transliterated name-mode

The title on the same entry IS transliterated (`Lúnyǔ`), so the mode is active — it just does not reach the name. `name-mode` appears to be wired only to `MultilingualName` (the structured original/transliterations/translations shape), not to a `SimpleName` whose `name` field is a `MultilingualString` carrying the same information.

## Why this matters now

Domain-expert review (discussion #828) established that names like 孔子 must NOT be decomposed into family/given. `name:` is the correct schema answer, and csl26-p7kj adopted it in examples/multilingual-refs.yaml. But adopting it currently costs the entry its romanization support — so the schema-correct way to model an indivisible name is also the way that loses transliteration. That tension should be resolved in favor of supporting both.

Scope: make name-mode/preferred-script resolution consult the `MultilingualString` inside `SimpleName.name` the same way it consults `MultilingualName`.
