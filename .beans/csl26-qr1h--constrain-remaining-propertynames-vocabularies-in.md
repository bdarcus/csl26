---
# csl26-qr1h
title: Constrain remaining propertyNames vocabularies in published schemas
status: todo
type: task
priority: normal
tags:
    - schema
    - engine
created_at: 2026-08-05T18:01:56Z
updated_at: 2026-08-05T18:04:33Z
---

propertyNames is now used for the four reference-type maps only. Nine other map-keyed vocabularies remain unconstrained across the eight published schemas: ContributorMerge.roles and RoleOptions.roles (contributor roles), LocatorConfig.kinds (locator types), MultilingualConfig.scripts (script codes), Config.messages (message keys), TitleRendering.locale-overrides and BibliographySortPartitioning.headings and the .localized heading fields (locale codes), GroupSelector.field (field names).

Closed vocabularies take propertyNames enum; open-ended ones (locale codes, message keys, field names) take propertyNames pattern, per the decision recorded in docs/specs/TYPED_TITLE_MAPPING.md.

Reuse crate::template::type_keyed_map_schema as the model. Note that schema_with bypasses schemars' Option detection, so any Option field needs serde(default) or it becomes required.
