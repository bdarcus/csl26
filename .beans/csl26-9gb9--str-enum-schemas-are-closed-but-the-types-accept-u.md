---
# csl26-9gb9
title: str_enum schemas are closed but the types accept Unknown
status: todo
type: bug
priority: normal
tags:
    - schema
    - style
created_at: 2026-08-05T19:37:45Z
updated_at: 2026-08-05T19:37:45Z
---

Types built with the `str_enum!` macro carry an `Unknown(String)` fallback variant (crates/citum-schema-style/src/macros.rs:33), so they deserialize any string. Their derived JsonSchema is a closed enum, so the published schema rejects values the engine accepts.

Concrete instance: chicago-author-date-18th.yaml authors `contributor: narrator` and `contributor: contributor` in its book, broadcast and motion-picture variants. Neither is a ContributorRole variant, so both load as Unknown and almost certainly render nothing — two dead components in a shipped style, plus a schema that rejects the file.

Decide per case rather than globally: either model the missing roles (narrator and contributor are legitimate CSL roles) and add the variants, or fix the style. If neither, the schema for str_enum types should describe the open vocabulary instead of a closed enum.

Found by extending scripts/validate-schemas.js to cover the embedded tier, which had never been schema-checked. chicago-author-date-18th is currently the sole entry in that script's style skip list; remove it when this lands.

Same family as three defects already fixed in csl26-q4g5: TypeSelector, WrapConfig and TitlesConfig.type-mapping all had hand-written Deserialize or key types the derived schema failed to describe.
