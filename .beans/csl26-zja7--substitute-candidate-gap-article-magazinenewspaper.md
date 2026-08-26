---
# csl26-zja7
title: 'Substitute candidate-gap: article-magazine/newspaper should promote container-title, not title'
status: todo
type: bug
priority: normal
tags:
    - engine
    - chicago
    - substitute
    - fidelity
created_at: 2026-08-26T14:50:36Z
updated_at: 2026-08-26T14:50:46Z
parent: csl26-h7oc
---

SubstituteField::Title (crates/citum-engine/src/values/contributor/substitute.rs, resolve_candidate) always resolves reference.title() and never considers container-title. For author-less article-magazine/article-newspaper entries where both title and container-title are present, real Chicago (CMOS 18) promotes the container-title (the magazine/newspaper name) into the author slot, with the article title surviving as its own quoted clause -- Citum promotes the wrong value. Found via docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md's taxonomy work on chicago-author-date-18th (5 rows, e.g. 6188419/Y7JIURAM Forbes/Aviation, 6188419/L4XXFEU2 Lake Forester/Pushcarts). Different defect surface than that spec's formatting mechanism -- this is which value gets chosen, not how the chosen value renders.
