---
# csl26-vf5x
title: 'Cluster 3: container-title terminal punctuation before volume/issue'
status: todo
type: task
priority: high
created_at: 2026-08-07T13:20:49Z
updated_at: 2026-08-08T13:36:08Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Stray/missing period after container-title before the volume:issue:page locator block (e.g. 'Urban Climate 15: 1-18.' vs Citum's 'Urban Climate. 15: 1-18'). ~68 residual observations (2026-07-30 clustering, csl26-giun evidence — flagged there as possibly stale, re-verify against current oracle before starting). Fix across all four Chicago-family styles at once.


## Re-check against current data (2026-08-08)

Re-verified against `node scripts/report-core.js` (references-expanded + chicago-18th corpora) rather than the 2026-07-30 clustering this bean's ~68 estimate came from, per this bean's own "possibly stale" flag. Result: scanning every currently-failing article-journal/article-magazine/article-newspaper bibliography entry across chicago-author-date-18th, taylor-and-francis-chicago-author-date, and chicago-shortened-notes-bibliography for the described "container-title, no period, before volume:issue:page" shape found zero clear matches. The ~68 estimate looks stale -- either already fixed by intervening work, or the pattern's actual current shape doesn't match the original description closely enough for an automated check.

The underlying population is still large and worth investigating, just not via the assumed pattern: article-journal fails 123/213 combined across the three styles (author-date 31/76, T&F 31/76, shortened 61/61), article-magazine 38/50 (11/17, 11/17, 16/16), article-newspaper 63/65 (21/22, 21/22, 21/21). Before starting, pull a fresh small sample of current failing diffs for these three types and re-derive the actual pattern(s) rather than trusting the original description. Note shortened's article-journal/magazine rates (100%/94%) are much worse than author-date/T&F's (~41%/~65%) on the same types -- likely dominated by the unrelated bibliography-separator bug tracked in csl26-zl7f, not this cluster's punctuation issue; re-check shortened's numbers after that lands so this cluster isn't scoped against a confounded baseline.
