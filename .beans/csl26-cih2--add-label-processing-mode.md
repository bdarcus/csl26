---
# csl26-cih2
title: Add label processing mode
status: todo
type: feature
priority: high
created_at: 2026-02-20T02:06:39Z
updated_at: 2026-02-20T02:06:39Z
---

The style author guide listed label as a valid processing mode but it does not exist in the schema or processor. Implement label mode (alphabetic labels like 'a', 'b', 'c') as a fourth processing mode alongside author-date, numeric, and note. Add to Processing enum in schema, implement in csln_processor, and restore documentation in the style author guide.
