---
# csl26-0s2b
title: Fix bibliography separator bug for author suffix comma
status: completed
type: bug
priority: high
created_at: 2026-02-12T21:05:16Z
updated_at: 2026-02-12T21:19:55Z
---

Missing comma after author initials in bibliography (e.g., 'S.' instead of 'S.,').

Root cause: bibliography.rs separator deduplication logic (lines 59-60) strips component suffixes when the next component doesn't have a prefix.

The YAML correctly specifies suffix: ', ' on the author component, but refs_to_string_with_format() sees the year component without a prefix and incorrectly skips the comma.

Files to fix:
- crates/csln_processor/src/render/bibliography.rs (separator logic)

Test cases:
- elsevier-harvard: 'Hawking, S., 1988.' (expected) vs 'Hawking, S. 1988.' (actual)
- APA 7th should not regress

Refs: elsevier-harvard style authoring, styleauthor evaluation
