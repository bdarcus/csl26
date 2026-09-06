---
# csl26-bg3f
title: APA legal type-class locator labels (Bluebook § form)
status: todo
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
created_at: 2026-09-06T17:47:18Z
updated_at: 2026-09-06T17:47:30Z
parent: csl26-ccdt
---

apa.csl's label-locator macro (styles-legacy/apa.csl:204-236) has a branch
for legal reference types (bill, hearing, legal_case, legislation,
regulation, treaty) that this project has not implemented:

  <else-if match="any" type="bill hearing legal_case legislation regulation treaty">
    <choose>
      <if locator="chapter paragraph section" match="any">
        <label form="symbol" variable="locator"/>   <!-- "§ 12" -->
      </if>
      <else>
        <label text-case="capitalize-first" variable="locator"/>
      </else>
    </choose>
  </else-if>

Deferred out of the Label Case and Attachment v1.1 addition to
docs/specs/LOCATOR_RENDERING.md (bean csl26-7652) as an explicit scope cut:
the v1.1 addition only reuses the existing LocatorPattern.type_class gate,
it doesn't add a new per-kind-and-type-class label override mechanism.

Needs: a way to override label-form (here: symbol) for specific
LocatorType/TypeClass combinations, likely extending LocatorPattern or
adding a type-class-scoped kinds override to LocatorConfig. Not yet a live
fidelity residual for any embedded style (no legal-type APA fixtures
currently exercise this branch) -- confirm with an oracle sweep before
prioritizing.
