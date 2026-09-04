---
# csl26-wbak
title: Given-first long-name assembly misorders a numeral generational suffix
status: todo
type: bug
priority: normal
tags:
    - style
    - chicago
    - fidelity
    - contributors
created_at: 2026-09-04T13:02:57Z
updated_at: 2026-09-04T13:03:07Z
parent: csl26-h7oc
---

assemble_given_first_long_name in crates/citum-engine/src/values/contributor/names.rs appends the suffix as the last pushed part unconditionally, producing Robert, III DeYeso instead of Robert DeYeso III. assemble_inverted_long_name already positions the suffix correctly relative to family/given for the inverted case; given-first needs equivalent positional logic. 4 confirmed rows. See plan: /home/bruce/.claude/plans/review-the-remaining-large-encapsulated-hearth.md
