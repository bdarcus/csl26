---
# csl26-0txc
title: Support multi-valued Publisher.name/place for literal-list fields
status: todo
type: task
priority: deferred
tags:
    - conversion
    - fidelity
    - schema
created_at: 2026-07-27T18:08:14Z
updated_at: 2026-07-27T18:08:19Z
---

csl26-11h2's biblatex literal-list fix (publisher/institution/organization/school/location) splits BibLaTeX's and-separated values and rejoins with '; ', but Publisher.name (a single MultilingualString) and Publisher.place (a single Place wrapper) cannot represent more than one entity -- a genuine multi-publisher or multi-location entry still collapses to one joined string. Fixing this for real needs a schema change to citum-schema-data (Publisher.name/place becoming list-valued, or a new multi-publisher relation), which is out of scope for a biblatex-conversion bean. Only worth doing if multi-publisher/multi-location entries turn out to be common in real corpora.
