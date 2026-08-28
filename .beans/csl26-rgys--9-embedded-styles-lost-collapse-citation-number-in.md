---
# csl26-rgys
title: '9 embedded styles lost collapse: citation-number in migration'
status: todo
type: bug
priority: normal
tags:
    - style
    - migrate
    - chicago
created_at: 2026-08-26T22:30:13Z
updated_at: 2026-08-26T22:30:21Z
parent: csl26-awlo
---

Cross-checked every style's info.source.csl-id against styles-legacy/*.csl <citation collapse=...>: 9 styles declare a collapse in CSL and have none in the YAML. 5 embedded -core styles already carry processing: numeric (elsevier-vancouver-core, elsevier-with-titles-core, springer-basic-brackets-core, springer-vancouver-brackets-core, taylor-and-francis-national-library-of-medicine-core) — one-line fix, add collapse: citation-number. 2 have no processing: key at all (american-medical-association-alphabetical, american-society-of-mechanical-engineers), which is why extract_citation_collapse (crates/citum-migrate/src/assembly.rs:707) dropped it — adding processing: numeric changes more than collapse, needs per-style oracle evidence. entomological-society-of-america declares collapse="year" (same-author mechanism, not citation-number) — separate check. american-mathematical-society-label is correctly absent (inert Label regime). Overlaps surface #9 of csl26-awlo's audit; wait for that spec before implementing so the fix lands in the coherent model, not ad hoc.
