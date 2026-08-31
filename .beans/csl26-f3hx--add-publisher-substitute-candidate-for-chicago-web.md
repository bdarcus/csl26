---
# csl26-f3hx
title: Add publisher substitute candidate for Chicago webpage citations
status: todo
type: feature
priority: normal
tags:
    - chicago
    - style
    - schema
    - authorability
created_at: 2026-08-31T12:24:24Z
updated_at: 2026-08-31T17:53:28Z
parent: csl26-h7oc
blocked_by:
    - csl26-lr1p
---

chicago-author-date-18th deletes its webpage citation type-variant (csl26-lr1p) because it can't correctly express CMOS 18 14.104's authorless-webpage rule: fall back to site owner/publisher only when no author/editor/translator/etc. is present, else fall back further to title. SubstituteField (crates/citum-schema-style/src/options/substitute.rs) has no Publisher variant, so citation.options.substitute.overrides.webpage cannot express [editor, translator, publisher, parent-serial, title]. Needs: add Publisher to SubstituteField, wire through crates/citum-engine/src/values/contributor/substitute.rs, set the override on chicago-author-date-18th, retire the known-divergences.json entry this created. Schema change -- needs a docs/specs/ spec reviewed in a docs-only PR first per CLAUDE.md.
