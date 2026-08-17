---
# csl26-u87d
title: 'oracle-snapshot.js: locale-loading error on etudes-chinoises and organon'
status: todo
type: bug
priority: normal
tags:
    - test-infrastructure
    - scripts
    - oracle
created_at: 2026-08-17T00:16:14Z
updated_at: 2026-08-17T00:16:21Z
---

node scripts/oracle-snapshot.js --all fails on 2 of 2844 styles-legacy/*.csl with
"Cannot read properties of undefined (reading 'strings')": etudes-chinoises.csl
and organon.csl. Both are French-language styles; the error looks like a
locale-loading gap in renderWithCiteprocJs / loadLocale (scripts/oracle-utils.js),
not a citeproc-js style-parsing failure per se, but not root-caused this session.

Found while verifying the fixture-refresh fan-out entrypoint added in
csl26-nrks (see docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md,
"Known pre-existing gap"). Blocks a full --all snapshot run from exiting 0.
