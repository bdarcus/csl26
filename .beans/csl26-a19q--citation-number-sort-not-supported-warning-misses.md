---
# csl26-a19q
title: citation_number_sort_not_supported warning misses inherited bibliography.sort
status: todo
type: bug
priority: normal
tags:
    - engine
    - schema
created_at: 2026-08-06T12:43:41Z
updated_at: 2026-08-06T12:43:51Z
parent: csl26-ccdt
---

The citation_number_sort_not_supported style-load warning
(crates/citum-engine/src/api/warnings.rs) inspects only
processing.config().sort, not style.bibliography.sort. A style that
inherits bibliography.sort: citation-number from a numeric base (e.g. an
author-date leaf extending a numeric base, as
gb-t-7714-2025-author-date.yaml still does — see the follow-up bean tracking
its own explicit sort) silently renders its bibliography in registry order
with no warning at all, even though citation-number has no group-sort
equivalent for a non-numeric processing family.

Needs a family-aware check: warn when an author-date/note-family style
resolves to bibliography.sort: citation-number, since that's legitimate only
for numeric styles.

Discovered while tracing csl26-m8la's root cause: the migrated
gb-t-7714-2025-author-date.yaml had no bibliography.sort of its own and
silently inherited the numeric base's citation-number sort, going unnoticed
specifically because this warning didn't fire.
