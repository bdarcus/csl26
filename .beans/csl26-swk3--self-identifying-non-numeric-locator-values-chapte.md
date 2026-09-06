---
# csl26-swk3
title: Self-identifying non-numeric locator values (chapter/line/verse without label)
status: todo
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
created_at: 2026-09-06T17:47:39Z
updated_at: 2026-09-06T17:47:39Z
parent: csl26-ccdt
---

apa.csl's label-locator macro (styles-legacy/apa.csl:204-236) has a branch
for canonical, self-identifying locator values that carry their own
formatting and therefore need no label at all:

  <else-if is-numeric="locator">
    <label text-case="capitalize-first" variable="locator"/>
  </else-if>
  <!-- a non-numeric canonical reference is identified by its formatting
       and does not need a label, similar to a timestamp -->
  <else-if locator="chapter line verse" match="any"/>
  <else>
    <label text-case="capitalize-first" variable="locator"/>
  </else>

i.e. when the locator value for chapter/line/verse is NOT a plain number
(e.g. "2.14.3"), no label is rendered at all -- distinct from the existing
LabelForm::None (which always suppresses the label for a kind regardless of
value shape). Today LocatorKindConfig has no notion of "conditional on the
value's shape."

Deferred out of the Label Case and Attachment v1.1 addition to
docs/specs/LOCATOR_RENDERING.md (bean csl26-7652) as an explicit scope cut.
Needs: a value-shape-aware label suppression, e.g.
LocatorKindConfig.label-form: numeric-only (label only when the value is a
plain number) or similar. Confirm with an oracle sweep whether any embedded
style currently exercises non-numeric chapter/line/verse values before
prioritizing.
