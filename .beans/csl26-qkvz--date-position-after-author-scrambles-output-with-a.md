---
# csl26-qkvz
title: 'date-position: after-author scrambles output with a leading empty contributor'
status: todo
type: bug
priority: low
tags:
    - engine
    - fidelity
created_at: 2026-09-05T21:49:39Z
updated_at: 2026-09-06T15:27:08Z
parent: csl26-ccdt
---

bibliography.options.date-position: after-author, combined with a
type-variant whose FIRST component is a contributor that's empty for
the given reference (e.g. an editor slot before author, empty when the
book has no editor) AND whose date is already manually placed right
after author in the template, scrambles the whole entry: the date and
an empty leading slot end up first, author gets moved into the middle
of the entry instead of the front.

Repro (isolated, single item, no grouping involved):
  citum render refs -b <single-book-item, no editor> -s elsevier-harvard
  # got: ", 1988. Hawking, S. A Brief History of Time. ..."
  # want: "Hawking, S., 1988. A Brief History of Time. ..."

Root cause not fully traced -- confirmed empirically (crates/citum-schema-style/embedded/styles/elsevier-harvard.yaml
had date-position: after-author on top of a book type-variant already
placing date: issued directly after contributor: author, with
contributor: editor (empty) as the type-variant's first component).
Removing the redundant date-position directive fixes it and was
verified via full oracle diff (+6, zero regressions) -- filed here
because springer-basic-author-date and taylor-and-francis-council-of-
science-editors-author-date also use date-position: after-author and
could hit the same interaction if a future type-variant edit adds a
leading empty contributor there.
