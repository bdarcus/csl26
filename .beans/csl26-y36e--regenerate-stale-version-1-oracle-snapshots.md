---
# csl26-y36e
title: Regenerate stale version-1 oracle snapshots
status: todo
type: task
priority: low
tags:
    - test-infrastructure
    - scripts
    - oracle
created_at: 2026-08-17T00:16:28Z
updated_at: 2026-08-17T00:16:31Z
blocked_by:
    - csl26-u87d
---

scripts/oracle-snapshot.js's SNAPSHOT_VERSION is 2 (adds bibliography_ids),
but a large share of tests/snapshots/csl/*.json on disk are still version 1
and were never regenerated after the bump -- isSnapshotCurrent() correctly
flags them stale, so any --all run rewrites thousands of files at once.

Found while verifying the fixture-refresh fan-out entrypoint added in
csl26-nrks (docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md,
"Known pre-existing gap"): a real no-op `just fixture-refresh` run wrote
2,812 of 2,844 snapshots (170k+ insertions), which is out of proportion to
review in that tooling PR, so it was reverted there rather than committed.

Blocked by csl26-u87d (2 styles fail to render entirely -- fix that first
so a full regeneration can exit 0). Regenerating is mechanical
(`node scripts/oracle-snapshot.js --all`) but the ~2,800-file diff needs a
dedicated, reviewed commit per the same reasoning baselines/README.md gives
for baseline refreshes.
